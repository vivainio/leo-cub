use std::path::Path;

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};
use syntect::{
    easy::HighlightLines,
    highlighting::{FontStyle, ThemeSet},
    parsing::{ParseState, ScopeStack, SyntaxDefinition, SyntaxReference, SyntaxSet},
};

pub struct SyntaxHighlighter {
    syntaxes: SyntaxSet,
    themes: ThemeSet,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        let mut syntaxes = SyntaxSet::load_defaults_newlines().into_builder();
        syntaxes.add(
            SyntaxDefinition::load_from_str(
                include_str!("../syntaxes/reStructuredText.sublime-syntax"),
                true,
                Some("reStructuredText"),
            )
            .expect("bundled reStructuredText syntax must be valid"),
        );
        syntaxes.add(
            SyntaxDefinition::load_from_str(
                include_str!("../syntaxes/Nushell.sublime-syntax"),
                true,
                Some("Nushell"),
            )
            .expect("bundled Nushell syntax must be valid"),
        );
        Self {
            syntaxes: syntaxes.build(),
            themes: ThemeSet::load_defaults(),
        }
    }

    pub fn highlight_with_language(
        &self,
        body: &str,
        source_path: Option<&Path>,
        inherited_language: Option<&str>,
    ) -> Text<'static> {
        let syntax = self.syntax_for(body, source_path, inherited_language);
        // Markdown's fenced code content gets no distinguishing scope of its
        // own (see `render_preview`), so splice in a per-language highlight
        // here too. Unlike preview, the fence delimiter lines themselves
        // stay put: this mode shows raw source with color, not a rendering
        // that hides markup.
        let fence_lines = (syntax.name == "Markdown").then(|| self.fenced_code_lines(body));
        let theme = &self.themes.themes["base16-ocean.dark"];
        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut lines = Vec::new();
        for (line_index, source_line) in body.split_inclusive('\n').enumerate() {
            let spans = highlighter
                .highlight_line(source_line, &self.syntaxes)
                .map(|ranges| {
                    ranges
                        .into_iter()
                        .filter_map(|(style, text)| {
                            let text = text.strip_suffix('\n').unwrap_or(text);
                            (!text.is_empty()).then(|| {
                                let mut ratatui_style = Style::default().fg(Color::Rgb(
                                    style.foreground.r,
                                    style.foreground.g,
                                    style.foreground.b,
                                ));
                                if style.font_style.contains(FontStyle::BOLD) {
                                    ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
                                }
                                if style.font_style.contains(FontStyle::ITALIC) {
                                    ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
                                }
                                if style.font_style.contains(FontStyle::UNDERLINE) {
                                    ratatui_style =
                                        ratatui_style.add_modifier(Modifier::UNDERLINED);
                                }
                                Span::styled(text.to_owned(), ratatui_style)
                            })
                        })
                        .collect()
                })
                .unwrap_or_else(|_| vec![Span::raw(source_line.trim_end_matches('\n').to_owned())]);
            match fence_lines
                .as_ref()
                .and_then(|fence_lines| fence_lines.get(line_index))
            {
                Some(FenceLine::Content(content_line)) => lines.push(content_line.clone()),
                _ => lines.push(Line::from(spans)),
            }
        }
        Text::from(lines)
    }

    /// Renders body text with its markup applied instead of shown, when the
    /// body's language has a preview renderer: `**bold**` becomes bold text
    /// with the asterisks hidden, `# heading` becomes a styled heading with
    /// the `#` hidden, and so on. Returns `None` when the detected language
    /// (via `@language`, inherited context, or file extension — the same
    /// resolution `highlight_with_language` uses) has no preview renderer
    /// registered yet, so callers can fall back to plain syntax highlighting.
    ///
    /// The renderer walks the same bundled syntect grammar used for syntax
    /// highlighting, but reads its raw scope names instead of theme colors —
    /// no separate parser per language. Adding a new previewable language is
    /// just a new entry in `scope_styler_for`.
    pub fn render_preview(
        &self,
        body: &str,
        source_path: Option<&Path>,
        inherited_language: Option<&str>,
    ) -> Option<Text<'static>> {
        let syntax = self.syntax_for(body, source_path, inherited_language);
        let styler = scope_styler_for(&syntax.name)?;
        // The bundled Markdown grammar doesn't embed other languages into
        // its own fenced-code scope (the fence's content gets no
        // distinguishing scope at all), so fenced code is highlighted as a
        // separate pass and spliced in by line index rather than through
        // the scope walk below.
        let fence_lines = self.fenced_code_lines(body);
        let mut parse_state = ParseState::new(syntax);
        let mut scope_stack = ScopeStack::new();
        let mut lines = Vec::new();
        for (line_index, source_line) in body.split_inclusive('\n').enumerate() {
            let Ok(ops) = parse_state.parse_line(source_line, &self.syntaxes) else {
                match fence_lines.get(line_index) {
                    Some(FenceLine::Hidden) => {}
                    Some(FenceLine::Content(line)) => lines.push(line.clone()),
                    _ => {
                        let text = source_line.strip_suffix('\n').unwrap_or(source_line);
                        lines.push(Line::from(text.to_owned()));
                    }
                }
                continue;
            };
            let mut spans = Vec::new();
            let mut cursor = 0usize;
            for (index, op) in ops {
                if index > cursor {
                    push_scoped_span(
                        &mut spans,
                        &source_line[cursor..index],
                        &scope_stack,
                        styler,
                    );
                }
                let _ = scope_stack.apply(&op);
                cursor = index;
            }
            if cursor < source_line.len() {
                push_scoped_span(&mut spans, &source_line[cursor..], &scope_stack, styler);
            }
            match fence_lines.get(line_index) {
                Some(FenceLine::Hidden) => {}
                Some(FenceLine::Content(line)) => lines.push(line.clone()),
                _ => lines.push(Line::from(spans)),
            }
        }
        Some(Text::from(lines))
    }

    /// Classifies each line of `body` for fenced-code splicing: the fence
    /// delimiter lines themselves (hidden, like other Markdown markup), the
    /// fenced content re-rendered through `highlight_with_language` for its
    /// declared language, or left alone (normal Markdown scope styling)
    /// when the fence has no language tag or is never closed.
    fn fenced_code_lines(&self, body: &str) -> Vec<FenceLine> {
        let source_lines: Vec<&str> = body.split_inclusive('\n').collect();
        let mut result: Vec<FenceLine> = (0..source_lines.len()).map(|_| FenceLine::None).collect();
        let mut index = 0;
        while index < source_lines.len() {
            let Some(language) = fence_open(source_lines[index]) else {
                index += 1;
                continue;
            };
            let content_start = index + 1;
            let close_index = (content_start..source_lines.len())
                .find(|&candidate| is_fence_close(source_lines[candidate]));
            let Some(close_index) = close_index else {
                break; // Unterminated fence: leave the rest as normal Markdown.
            };

            result[index] = FenceLine::Hidden;
            result[close_index] = FenceLine::Hidden;
            if !language.is_empty() {
                let content: String = source_lines[content_start..close_index].concat();
                let highlighted = self.highlight_with_language(&content, None, Some(language));
                for (offset, line) in highlighted.lines.into_iter().enumerate() {
                    result[content_start + offset] = FenceLine::Content(line);
                }
            }
            index = close_index + 1;
        }
        result
    }

    fn syntax_for<'a>(
        &'a self,
        body: &str,
        source_path: Option<&Path>,
        inherited_language: Option<&str>,
    ) -> &'a SyntaxReference {
        language_directive(body)
            .or(inherited_language)
            .and_then(|language| {
                let token = match language {
                    // Syntect's default syntax bundle has no TypeScript entry.
                    // JavaScript is a useful baseline until one is bundled.
                    "typescript" | "tsx" => "javascript",
                    language => language,
                };
                self.syntaxes.find_syntax_by_token(token)
            })
            .or_else(|| {
                source_path
                    .and_then(Path::extension)
                    .and_then(|extension| extension.to_str())
                    .and_then(|extension| {
                        // The bundled XML grammar's file_extensions list has
                        // "xslt" but not "xsl".
                        let extension = if extension == "xsl" {
                            "xslt"
                        } else {
                            extension
                        };
                        self.syntaxes.find_syntax_by_extension(extension)
                    })
            })
            .unwrap_or_else(|| self.syntaxes.find_syntax_plain_text())
    }
}

/// How a source line participates in a fenced code block, per
/// `SyntaxHighlighter::fenced_code_lines`.
#[derive(Clone)]
enum FenceLine {
    /// Not part of a (recognized) fenced code block.
    None,
    /// A fence delimiter line (` ```lang ` or ` ``` `) — hidden in preview,
    /// like other Markdown markup.
    Hidden,
    /// A line of fenced content, already highlighted for its language.
    Content(Line<'static>),
}

/// Detects a fence-opening line like "```rust" or a bare "```", returning
/// the language token (empty string if none given). Handles exact
/// triple-backtick fences only, which covers ordinary notes; four-or-more
/// backtick fences, tilde fences, and indented fences aren't recognized.
fn fence_open(line: &str) -> Option<&str> {
    let trimmed = line.trim_end_matches(['\n', '\r']).trim_start();
    let rest = trimmed.strip_prefix("```")?;
    if rest.starts_with('`') {
        return None;
    }
    Some(rest.trim())
}

fn is_fence_close(line: &str) -> bool {
    line.trim_end_matches(['\n', '\r']).trim() == "```"
}

/// Maps a syntect scope stack (rendered as a space-separated string, e.g.
/// `"meta.paragraph.markdown markup.bold.markdown"`) to a display style, or
/// `None` to hide that span entirely. One of these per previewable language.
type ScopeStyler = fn(&str) -> Option<Style>;

/// Looks up the preview styler for a syntax by name, if one exists yet.
/// Unregistered languages (including plain code) fall back to `None`, and
/// `render_preview` reports that as "no preview available" to its caller.
fn scope_styler_for(syntax_name: &str) -> Option<ScopeStyler> {
    match syntax_name {
        "Markdown" => Some(markdown_scope_style as ScopeStyler),
        _ => None,
    }
}

/// Appends the styled span for `text` under the given scope stack, dropping
/// it entirely when `styler` says the span (a delimiter like `**`, `` ` ``,
/// or `#`) should be hidden rather than shown.
fn push_scoped_span(
    spans: &mut Vec<Span<'static>>,
    text: &str,
    scope_stack: &ScopeStack,
    styler: ScopeStyler,
) {
    let text = text.strip_suffix('\n').unwrap_or(text);
    let scopes = scope_stack.to_string();
    let Some(style) = styler(&scopes) else {
        return;
    };
    // The space between a hidden "#" marker and the heading text isn't its
    // own scope, so once the marker is dropped, trim the separator too.
    let text = if spans.is_empty() && scopes.contains("markup.heading") {
        text.trim_start_matches(' ')
    } else {
        text
    };
    if text.is_empty() {
        return;
    }
    spans.push(Span::styled(text.to_owned(), style));
}

fn markdown_scope_style(scopes: &str) -> Option<Style> {
    if scopes.contains("punctuation.definition.bold")
        || scopes.contains("punctuation.definition.italic")
        || scopes.contains("punctuation.definition.raw")
        || scopes.contains("punctuation.definition.heading")
    {
        return None;
    }
    let mut style = Style::default();
    if scopes.contains("markup.heading") {
        style = style.fg(Color::Cyan).add_modifier(Modifier::BOLD);
    }
    if scopes.contains("markup.bold") {
        style = style.add_modifier(Modifier::BOLD);
    }
    if scopes.contains("markup.italic") {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if scopes.contains("markup.raw.inline") {
        style = style.fg(Color::Yellow);
    }
    if scopes.contains("markup.underline.link") {
        style = style.fg(Color::Blue).add_modifier(Modifier::UNDERLINED);
    }
    if scopes.contains("markup.quote") {
        style = style.fg(Color::DarkGray).add_modifier(Modifier::ITALIC);
    }
    Some(style)
}

pub(crate) fn language_directive(body: &str) -> Option<&str> {
    body.lines().find_map(|line| {
        line.trim_start()
            .strip_prefix("@language")
            .and_then(|rest| rest.split_whitespace().next())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_leo_language_directives() {
        assert_eq!(
            language_directive("@tabwidth -4\n@language rust\n"),
            Some("rust")
        );
        assert_eq!(language_directive("let language = rust;"), None);
    }

    #[test]
    fn highlighting_preserves_line_structure_and_whitespace() {
        let text = SyntaxHighlighter::new().highlight_with_language(
            "fn main() {\n    true\n}\n",
            Some(Path::new("main.rs")),
            None,
        );
        assert_eq!(text.lines.len(), 3);
        assert_eq!(text.lines[1].width(), 8);
    }

    #[test]
    fn plain_syntax_highlighting_colors_fenced_code_but_keeps_markup_visible() {
        let text = SyntaxHighlighter::new().highlight_with_language(
            "Some **bold** text.\n\n```rust\nlet x = 1;\n```\n",
            Some(Path::new("x.md")),
            None,
        );
        let rendered: Vec<String> = text
            .lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect()
            })
            .collect();
        // Unlike preview, delimiters (** and ```rust/```) stay in the text.
        assert_eq!(
            rendered,
            vec!["Some **bold** text.", "", "```rust", "let x = 1;", "```"]
        );

        let code_line = &text.lines[3];
        assert!(
            code_line.spans.len() > 1,
            "fenced content should carry per-token highlighting, not one plain span"
        );
        let number_span = code_line
            .spans
            .iter()
            .find(|span| span.content.as_ref() == "1")
            .expect("the literal 1 is highlighted as its own span");
        assert_ne!(
            number_span.style, code_line.spans[0].style,
            "the number should be styled differently than surrounding code"
        );
    }

    #[test]
    fn bundled_syntaxes_cover_static_auto_languages() {
        let highlighter = SyntaxHighlighter::new();
        for (path, language) in [
            ("x.cs", "csharp"),
            ("x.go", "go"),
            ("x.js", "javascript"),
            ("x.ts", "typescript"),
            ("x.tsx", "typescript"),
            ("x.xslt", "xslt"),
            ("x.nu", "nushell"),
        ] {
            let syntax = highlighter.syntax_for("", Some(Path::new(path)), Some(language));
            assert_ne!(syntax.name, "Plain Text", "{path} / {language}");
        }
    }

    #[test]
    fn xsl_and_xslt_extensions_resolve_to_xml_without_a_language_directive() {
        let highlighter = SyntaxHighlighter::new();
        for path in ["x.xsl", "x.xslt"] {
            let syntax = highlighter.syntax_for("", Some(Path::new(path)), None);
            assert_eq!(syntax.name, "XML", "{path}");
        }
    }

    #[test]
    fn markdown_preview_hides_delimiters_and_applies_styles() {
        let text = SyntaxHighlighter::new()
            .render_preview(
                "# Title\n\nSome **bold** and *italic* text.\n",
                Some(Path::new("x.md")),
                None,
            )
            .expect("markdown has a preview renderer");
        let rendered: Vec<String> = text
            .lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect()
            })
            .collect();
        assert_eq!(rendered, vec!["Title", "", "Some bold and italic text."]);

        let heading_style = text.lines[0].spans[0].style;
        assert!(heading_style.add_modifier.contains(Modifier::BOLD));

        let body_spans = &text.lines[2].spans;
        let bold_span = body_spans
            .iter()
            .find(|span| span.content.as_ref() == "bold")
            .expect("bold span present");
        assert!(bold_span.style.add_modifier.contains(Modifier::BOLD));
        let italic_span = body_spans
            .iter()
            .find(|span| span.content.as_ref() == "italic")
            .expect("italic span present");
        assert!(italic_span.style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn preview_is_unavailable_for_languages_without_a_renderer() {
        let text = SyntaxHighlighter::new().render_preview(
            "fn main() {}\n",
            Some(Path::new("main.rs")),
            None,
        );
        assert!(text.is_none());
    }

    #[test]
    fn markdown_preview_highlights_fenced_code_by_its_own_language() {
        let text = SyntaxHighlighter::new()
            .render_preview(
                "Before.\n\n```rust\nlet x = 1;\n```\n\nAfter.\n",
                Some(Path::new("x.md")),
                None,
            )
            .expect("markdown has a preview renderer");
        let rendered: Vec<String> = text
            .lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect()
            })
            .collect();
        // The ``` delimiter lines are gone; the fenced line survives intact.
        assert_eq!(rendered, vec!["Before.", "", "let x = 1;", "", "After."]);

        let code_line = &text.lines[2];
        assert!(
            code_line.spans.len() > 1,
            "fenced content should carry per-token highlighting, not one plain span"
        );
        let number_span = code_line
            .spans
            .iter()
            .find(|span| span.content.as_ref() == "1")
            .expect("the literal 1 is highlighted as its own span");
        assert_ne!(
            number_span.style, code_line.spans[0].style,
            "the number should be styled differently than surrounding code"
        );
    }

    #[test]
    fn markdown_preview_hides_fence_delimiters_even_without_a_language_tag() {
        let text = SyntaxHighlighter::new()
            .render_preview("```\nplain block\n```\n", Some(Path::new("x.md")), None)
            .expect("markdown has a preview renderer");
        let rendered: Vec<String> = text
            .lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect()
            })
            .collect();
        assert_eq!(rendered, vec!["plain block"]);
    }

    #[test]
    fn markdown_preview_leaves_an_unterminated_fence_as_plain_markdown() {
        let text = SyntaxHighlighter::new()
            .render_preview("```rust\nlet x = 1;\n", Some(Path::new("x.md")), None)
            .expect("markdown has a preview renderer");
        // No panic, and the fence marker line is still present since it was
        // never recognized as closed.
        assert_eq!(text.lines.len(), 2);
    }

    #[test]
    fn bundled_syntaxes_cover_restructured_text() {
        let highlighter = SyntaxHighlighter::new();
        for (path, language) in [("x.rst", "rst"), ("x.rest", "restructuredtext")] {
            let by_extension = highlighter.syntax_for("", Some(Path::new(path)), None);
            assert_eq!(by_extension.name, "reStructuredText", "{path}");

            let by_language = highlighter.syntax_for("", None, Some(language));
            assert_eq!(by_language.name, "reStructuredText", "{language}");
        }
    }
}
