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
        syntaxes.add(
            SyntaxDefinition::load_from_str(
                include_str!("../syntaxes/Rhai.sublime-syntax"),
                true,
                Some("Rhai"),
            )
            .expect("bundled Rhai syntax must be valid"),
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
            let spans = leo_directive_spans(source_line).unwrap_or(spans);
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
        // the scope walk below. GFM tables get the same treatment: syntect
        // styles the pipes and dashes as punctuation but has no notion of
        // column alignment, so a block-level pass computes column widths
        // and re-renders the rows as plain, padded text.
        let fence_lines = self.fenced_code_lines(body);
        let table_lines = table_lines(body, &fence_lines);
        let mut parse_state = ParseState::new(syntax);
        let mut scope_stack = ScopeStack::new();
        let mut lines = Vec::new();
        for (line_index, source_line) in body.split_inclusive('\n').enumerate() {
            let Ok(ops) = parse_state.parse_line(source_line, &self.syntaxes) else {
                let text = source_line.strip_suffix('\n').unwrap_or(source_line);
                if let Some(line) =
                    resolve_preview_line(&fence_lines, &table_lines, line_index, || {
                        Line::from(text.to_owned())
                    })
                {
                    lines.push(line);
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
            if let Some(line) =
                resolve_preview_line(&fence_lines, &table_lines, line_index, || Line::from(spans))
            {
                lines.push(line);
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
            } else {
                // Left to the normal per-line scope pass (see
                // `resolve_preview_line`), but still marked as inside a
                // fence so block-level passes like `table_lines` don't
                // mistake fenced content for a table.
                for line in result.iter_mut().take(close_index).skip(content_start) {
                    *line = FenceLine::PlainContent;
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

/// Resolves what a preview line should render as, giving block-level
/// passes (fenced code, tables) priority over the per-line scope-styled
/// `fallback`: hidden lines (a fence delimiter, a table's `---` separator
/// row) are dropped, replaced lines (fenced content, a table row) are used
/// as-is, and everything else falls back to the scope-styled spans.
fn resolve_preview_line(
    fence_lines: &[FenceLine],
    table_lines: &[TableLine],
    line_index: usize,
    fallback: impl FnOnce() -> Line<'static>,
) -> Option<Line<'static>> {
    match fence_lines.get(line_index) {
        Some(FenceLine::Hidden) => return None,
        Some(FenceLine::Content(line)) => return Some(line.clone()),
        Some(FenceLine::PlainContent) => return Some(fallback()),
        _ => {}
    }
    match table_lines.get(line_index) {
        Some(TableLine::Hidden) => None,
        Some(TableLine::Content(line)) => Some(line.clone()),
        _ => Some(fallback()),
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
    /// A line of fenced content with no declared language: rendered
    /// through the normal per-line scope pass rather than replaced here,
    /// but still inside a fence for the purposes of other block-level
    /// passes (e.g. `table_lines` must not treat it as a table row).
    PlainContent,
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

/// How a source line participates in a GFM table, per `table_lines`.
#[derive(Clone)]
enum TableLine {
    /// Not part of a (recognized) table.
    None,
    /// The delimiter row (e.g. `|---|:--:|`) — pure markup, hidden in
    /// preview like a fence delimiter.
    Hidden,
    /// A header or data row, re-rendered with its cells padded to their
    /// column's width.
    Content(Line<'static>),
}

#[derive(Clone, Copy)]
enum TableAlign {
    Left,
    Center,
    Right,
}

/// Scans `body` for GFM tables (a header row immediately followed by a
/// `|---|---|`-style delimiter row) and, for each one found, computes
/// column widths from the header and all contiguous data rows that follow,
/// then re-renders every row padded to those widths. Cell content is shown
/// as plain text — no inline styling (bold, code, links) is applied within
/// a cell.
///
/// Lines already claimed by a fenced code block (`fence_lines`) are never
/// treated as part of a table, since a fence can legitimately contain
/// pipe-delimited text that isn't meant to be rendered as one.
fn table_lines(body: &str, fence_lines: &[FenceLine]) -> Vec<TableLine> {
    let source_lines: Vec<&str> = body.split_inclusive('\n').collect();
    let mut result: Vec<TableLine> = vec![TableLine::None; source_lines.len()];
    let is_free = |i: usize| matches!(fence_lines.get(i), None | Some(FenceLine::None));
    let mut index = 0;
    while index < source_lines.len() {
        if !is_free(index) {
            index += 1;
            continue;
        }
        let delim_index = index + 1;
        let header_raw = source_lines[index].trim_end_matches(['\n', '\r']);
        if header_raw.trim().is_empty()
            || !header_raw.contains('|')
            || delim_index >= source_lines.len()
            || !is_free(delim_index)
        {
            index += 1;
            continue;
        }
        let header_cells = split_table_row(header_raw);
        let delim_raw = source_lines[delim_index].trim_end_matches(['\n', '\r']);
        let delim_cells = split_table_row(delim_raw);
        let (Some(aligns), false) = (parse_delimiter_row(&delim_cells), header_cells.is_empty())
        else {
            index += 1;
            continue;
        };

        let column_count = header_cells.len();
        let mut data_rows: Vec<Vec<String>> = Vec::new();
        let mut cursor = delim_index + 1;
        while cursor < source_lines.len() && is_free(cursor) {
            let raw = source_lines[cursor].trim_end_matches(['\n', '\r']);
            if raw.trim().is_empty() || !raw.contains('|') {
                break;
            }
            data_rows.push(split_table_row(raw));
            cursor += 1;
        }

        let mut widths: Vec<usize> = (0..column_count)
            .map(|column| header_cells[column].chars().count())
            .collect();
        for row in &data_rows {
            for (column, width) in widths.iter_mut().enumerate().take(column_count) {
                let len = row.get(column).map_or(0, |cell| cell.chars().count());
                *width = (*width).max(len);
            }
        }

        result[index] = TableLine::Content(render_table_row(&header_cells, &widths, &aligns));
        result[delim_index] = TableLine::Hidden;
        for (offset, row) in data_rows.iter().enumerate() {
            result[delim_index + 1 + offset] =
                TableLine::Content(render_table_row(row, &widths, &aligns));
        }
        index = cursor;
    }
    result
}

/// Splits a table row on unescaped `|` characters (`\|` is a literal pipe,
/// per GFM), trims each cell, and drops a leading/trailing empty cell
/// caused by the row's own leading/trailing pipe (`| a | b |` and `a | b`
/// both yield `["a", "b"]`).
fn split_table_row(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut chars = line.trim().chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'|') {
            current.push('|');
            chars.next();
        } else if c == '|' {
            cells.push(current.trim().to_owned());
            current.clear();
        } else {
            current.push(c);
        }
    }
    cells.push(current.trim().to_owned());
    if cells.first().is_some_and(String::is_empty) {
        cells.remove(0);
    }
    if cells.last().is_some_and(String::is_empty) {
        cells.pop();
    }
    cells
}

/// Parses a delimiter row's cells (already split by `split_table_row`)
/// into per-column alignment, or `None` if any cell isn't a valid
/// delimiter (`-+`, optionally flanked by `:`).
fn parse_delimiter_row(cells: &[String]) -> Option<Vec<TableAlign>> {
    if cells.is_empty() {
        return None;
    }
    cells
        .iter()
        .map(|cell| {
            let left = cell.starts_with(':');
            let right = cell.ends_with(':');
            let dashes = cell.trim_matches(':');
            (!dashes.is_empty() && dashes.chars().all(|c| c == '-')).then_some(
                match (left, right) {
                    (true, true) => TableAlign::Center,
                    (false, true) => TableAlign::Right,
                    _ => TableAlign::Left,
                },
            )
        })
        .collect()
}

/// Renders one table row as plain, padded text (e.g. `"| Name  | Age |"`),
/// aligning each cell within its column's width per `aligns`. Missing
/// trailing cells (a short data row) render as blank padding.
fn render_table_row(cells: &[String], widths: &[usize], aligns: &[TableAlign]) -> Line<'static> {
    let mut text = String::from("|");
    for (column, &width) in widths.iter().enumerate() {
        let cell = cells.get(column).map_or("", String::as_str);
        let pad = width.saturating_sub(cell.chars().count());
        text.push(' ');
        match aligns.get(column).copied().unwrap_or(TableAlign::Left) {
            TableAlign::Left => {
                text.push_str(cell);
                text.push_str(&" ".repeat(pad));
            }
            TableAlign::Right => {
                text.push_str(&" ".repeat(pad));
                text.push_str(cell);
            }
            TableAlign::Center => {
                let left_pad = pad / 2;
                text.push_str(&" ".repeat(left_pad));
                text.push_str(cell);
                text.push_str(&" ".repeat(pad - left_pad));
            }
        }
        text.push_str(" |");
    }
    Line::from(text)
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

/// Body-level Leo directives (`@language`, `@tabwidth`, `@others`,
/// `@nonl`, `@first`, `@last`) aren't part of any target language's
/// grammar, so no grammar highlights them meaningfully on its own -- not
/// even a hand-picked one like the bundled Nushell syntax, and syntect's
/// built-in defaults (Python, Rust, Bash, JavaScript, ...) can't be patched
/// from here at all. Overriding a directive line's rendering with the same
/// cyan used for headline directives (see `tui::headline_spans`) keeps it
/// visually distinct regardless of which language is active, instead of
/// needing every grammar -- vendored or not -- to special-case Leo's
/// syntax.
fn leo_directive_spans(line: &str) -> Option<Vec<Span<'static>>> {
    const DIRECTIVES: &[&str] = &[
        "@language",
        "@tabwidth",
        "@others",
        "@nonl",
        "@first",
        "@last",
    ];
    let trimmed = line.trim_end_matches(['\n', '\r']);
    let content = trimmed.trim_start();
    let leading = trimmed.len() - content.len();
    let directive_len = content.find(char::is_whitespace).unwrap_or(content.len());
    let directive = &content[..directive_len];
    if !DIRECTIVES.contains(&directive) {
        return None;
    }
    let mut spans = Vec::with_capacity(3);
    if leading > 0 {
        spans.push(Span::raw(trimmed[..leading].to_owned()));
    }
    spans.push(Span::styled(
        directive.to_owned(),
        Style::default().fg(Color::Cyan),
    ));
    let remainder = &content[directive_len..];
    if !remainder.is_empty() {
        spans.push(Span::raw(remainder.to_owned()));
    }
    Some(spans)
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
    fn directive_lines_are_highlighted_the_same_regardless_of_language() {
        // "nu" has no syntect grammar of its own for anything but Nushell
        // code, and "cobol" has no bundled grammar at all, so both bodies
        // fall back to different (or no) syntax highlighting for their
        // code -- but the directive lines should render identically in
        // both, since neither grammar can be expected to know about them.
        let nu = SyntaxHighlighter::new().highlight_with_language("@language nu\nls\n", None, None);
        let cobol = SyntaxHighlighter::new().highlight_with_language(
            "@language cobol\nDISPLAY 'HI'.\n",
            None,
            None,
        );
        for text in [&nu, &cobol] {
            assert_eq!(text.lines[0].spans[0].content, "@language");
            assert_eq!(text.lines[0].spans[0].style.fg, Some(Color::Cyan));
        }
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
    fn rhai_source_gets_real_highlighting_not_a_single_plain_span() {
        let highlighter = SyntaxHighlighter::new();
        let text = highlighter.highlight_with_language(
            "fn shq(s) {\n    // comment\n    let escaped = s.to_string();\n    \"'\" + escaped + \"'\"\n}\n",
            Some(Path::new("x.rhai")),
            None,
        );
        assert!(
            text.lines[0].spans.len() > 1,
            "a line with `fn` and an identifier should carry more than one span"
        );
        let fn_span = text.lines[0]
            .spans
            .iter()
            .find(|span| span.content.as_ref() == "fn")
            .expect("`fn` keyword should be its own span");
        let comment_span = text.lines[1]
            .spans
            .iter()
            .find(|span| span.content.as_ref().contains("comment"))
            .expect("comment text should be its own span");
        assert_ne!(fn_span.style.fg, comment_span.style.fg);
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
            ("x.rhai", "rhai"),
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
    fn markdown_preview_aligns_table_columns_and_hides_the_delimiter_row() {
        let text = SyntaxHighlighter::new()
            .render_preview(
                "| Name | Age |\n|---|---|\n| Alice | 30 |\n| Bob | 5 |\n",
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
        // The delimiter row is gone; every cell is padded to its column's
        // widest entry ("Alice" for column 1, "Age"/"30" for column 2).
        assert_eq!(
            rendered,
            vec!["| Name  | Age |", "| Alice | 30  |", "| Bob   | 5   |"]
        );
    }

    #[test]
    fn markdown_preview_honors_table_column_alignment_markers() {
        let text = SyntaxHighlighter::new()
            .render_preview(
                "| Left | Mid | Right |\n|:---|:---:|---:|\n| a | b | c |\n",
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
        assert_eq!(
            rendered,
            vec!["| Left | Mid | Right |", "| a    |  b  |     c |"]
        );
    }

    #[test]
    fn markdown_preview_does_not_render_pipe_text_inside_a_fence_as_a_table() {
        let text = SyntaxHighlighter::new()
            .render_preview(
                "```\n| a | b |\n|---|---|\n```\n",
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
        // Fenced content stays verbatim -- the delimiter row inside the
        // fence must not be swallowed as if it were a real table.
        assert_eq!(rendered, vec!["| a | b |", "|---|---|"]);
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
