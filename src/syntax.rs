use std::path::Path;

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};
use syntect::{
    easy::HighlightLines,
    highlighting::{FontStyle, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
};

pub struct SyntaxHighlighter {
    syntaxes: SyntaxSet,
    themes: ThemeSet,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self {
            syntaxes: SyntaxSet::load_defaults_newlines(),
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
        let theme = &self.themes.themes["base16-ocean.dark"];
        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut lines = Vec::new();
        for source_line in body.split_inclusive('\n') {
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
            lines.push(Line::from(spans));
        }
        Text::from(lines)
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
                    .and_then(|extension| self.syntaxes.find_syntax_by_extension(extension))
            })
            .unwrap_or_else(|| self.syntaxes.find_syntax_plain_text())
    }
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
    fn bundled_syntaxes_cover_static_auto_languages() {
        let highlighter = SyntaxHighlighter::new();
        for (path, language) in [
            ("x.cs", "csharp"),
            ("x.go", "go"),
            ("x.js", "javascript"),
            ("x.ts", "typescript"),
            ("x.tsx", "typescript"),
        ] {
            let syntax = highlighter.syntax_for("", Some(Path::new(path)), Some(language));
            assert_ne!(syntax.name, "Plain Text", "{path} / {language}");
        }
    }
}
