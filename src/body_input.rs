//! Inline "quick entry" editor for a node's body: type or paste directly
//! into the TUI instead of round-tripping through `$EDITOR`.
//!
//! Deliberately knows nothing about the outline document or `App` -- it
//! only tracks the text being edited. The caller (`tui.rs`) owns which
//! node the edit targets and what happens when it's accepted or cancelled.

use crossterm::event::{KeyCode, KeyModifiers};

/// A block of pasted text collapsed into a single placeholder character in
/// [`BodyInput::value`], the way Claude Code's own prompt collapses a large
/// paste to `[Pasted text +N lines]`. Keeps a big paste from turning the
/// inline editor into an unscrollable wall of text, and from being
/// shredded character-by-character by cursor/backspace/delete, which all
/// operate on `value` one `char` at a time.
pub(crate) struct PastedBlock {
    pub(crate) text: String,
    pub(crate) lines: usize,
}

/// The first code point of the Unicode Private Use Area used to stand in
/// for a collapsed paste inside [`BodyInput::value`]; a block's index into
/// `pastes` is added to this to form its marker char. Never produced by
/// typing or by a real paste under [`PASTE_COLLAPSE_CHARS`], so a marker
/// always round-trips through cursor motion and commit as one unit.
const PASTE_MARKER_BASE: u32 = 0xE000;

/// Pastes at or under this length, with no newline, are inserted literally
/// instead of being collapsed to a placeholder.
pub(crate) const PASTE_COLLAPSE_CHARS: usize = 200;

pub(crate) struct BodyInput {
    pub(crate) value: String,
    pub(crate) cursor: usize,
    pub(crate) selected: bool,
    pub(crate) pastes: Vec<PastedBlock>,
}

/// What a keypress did to a [`BodyInput`], for the caller (which owns the
/// outline document) to act on.
pub(crate) enum BodyInputOutcome {
    /// Nothing to do beyond the state already mutated in place.
    Continue,
    /// Accept the edit (Ctrl-D).
    Commit,
    /// Accept the edit and persist to disk (Ctrl-S).
    CommitAndSave,
    /// Discard the edit (Esc).
    Cancel,
}

impl BodyInput {
    /// Starts editing `value`, fully selected so the first keystroke or
    /// paste replaces it (matching how `HeadlineInput` primes a rename).
    pub(crate) fn new(value: String) -> Self {
        let cursor = value.len();
        let selected = !value.is_empty();
        Self {
            value,
            cursor,
            selected,
            pastes: Vec::new(),
        }
    }

    /// Resolves `value` back into real text: each paste marker character is
    /// expanded to the full text of the [`PastedBlock`] it stands in for.
    pub(crate) fn resolve(&self) -> String {
        let mut result = String::with_capacity(self.value.len());
        for character in self.value.chars() {
            let index = (character as u32).wrapping_sub(PASTE_MARKER_BASE) as usize;
            match self.pastes.get(index) {
                Some(block) => result.push_str(&block.text),
                None => result.push(character),
            }
        }
        result
    }

    fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
        self.selected = false;
        self.pastes.clear();
    }

    /// Inserts clipboard text pasted (via bracketed paste) while this field
    /// is open. Anything short and single-line is inserted literally;
    /// anything bigger is collapsed to a placeholder character so it
    /// doesn't flood the inline field.
    pub(crate) fn insert_paste(&mut self, text: String) {
        if self.selected {
            self.clear();
        }
        let lines = text.lines().count().max(1);
        if text.contains('\n') || text.chars().count() > PASTE_COLLAPSE_CHARS {
            let index = self.pastes.len();
            let Some(marker) = char::from_u32(PASTE_MARKER_BASE + index as u32) else {
                // Astronomically many pastes in one sitting; fall back to a
                // literal insert rather than losing the text.
                self.value.insert_str(self.cursor, &text);
                self.cursor += text.len();
                return;
            };
            self.pastes.push(PastedBlock { text, lines });
            self.value.insert(self.cursor, marker);
            self.cursor += marker.len_utf8();
        } else {
            self.value.insert_str(self.cursor, &text);
            self.cursor += text.len();
        }
    }

    pub(crate) fn handle_key(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
    ) -> BodyInputOutcome {
        match code {
            KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
                return BodyInputOutcome::Commit;
            }
            KeyCode::Char('s') if modifiers.contains(KeyModifiers::CONTROL) => {
                return BodyInputOutcome::CommitAndSave;
            }
            KeyCode::Esc => return BodyInputOutcome::Cancel,
            KeyCode::Enter => {
                if self.selected {
                    self.clear();
                }
                self.value.insert(self.cursor, '\n');
                self.cursor += 1;
            }
            KeyCode::Backspace => {
                if self.selected {
                    self.clear();
                } else if self.cursor > 0 {
                    let previous = self.value[..self.cursor]
                        .char_indices()
                        .next_back()
                        .map_or(0, |(index, _)| index);
                    self.value.drain(previous..self.cursor);
                    self.cursor = previous;
                }
            }
            KeyCode::Delete => {
                if self.selected {
                    self.clear();
                } else if self.cursor < self.value.len() {
                    let next = self.cursor
                        + self.value[self.cursor..]
                            .chars()
                            .next()
                            .expect("cursor precedes a character")
                            .len_utf8();
                    self.value.drain(self.cursor..next);
                }
            }
            KeyCode::Left => {
                if self.selected {
                    self.cursor = 0;
                    self.selected = false;
                } else if self.cursor > 0 {
                    self.cursor = self.value[..self.cursor]
                        .char_indices()
                        .next_back()
                        .map_or(0, |(index, _)| index);
                }
            }
            KeyCode::Right => {
                if self.selected {
                    self.cursor = self.value.len();
                    self.selected = false;
                } else if self.cursor < self.value.len() {
                    self.cursor += self.value[self.cursor..]
                        .chars()
                        .next()
                        .expect("cursor precedes a character")
                        .len_utf8();
                }
            }
            KeyCode::Home => {
                self.cursor = 0;
                self.selected = false;
            }
            KeyCode::End => {
                self.cursor = self.value.len();
                self.selected = false;
            }
            KeyCode::Char(character)
                if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if self.selected {
                    self.clear();
                }
                self.value.insert(self.cursor, character);
                self.cursor += character.len_utf8();
            }
            _ => {}
        }
        BodyInputOutcome::Continue
    }
}

/// One piece of a [`BodyInput::value`] slice, for rendering. Keeps the
/// paste-marker encoding private to this module -- the caller just matches
/// on tokens instead of reimplementing marker detection.
pub(crate) enum BodyInputToken<'a> {
    Text(&'a str),
    Newline,
    Paste(&'a PastedBlock),
}

/// Splits `text` (a [`BodyInput::value`] slice) into display tokens against
/// `pastes`. `text` need not be the whole value -- the caller can tokenize
/// either side of the cursor separately to draw a cursor marker between
/// them, since paste markers encode an absolute index into `pastes` that's
/// valid from any substring.
pub(crate) fn tokenize<'a>(text: &'a str, pastes: &'a [PastedBlock]) -> Vec<BodyInputToken<'a>> {
    let mut tokens = Vec::new();
    let mut run_start = 0;
    for (index, character) in text.char_indices() {
        if character == '\n' {
            if index > run_start {
                tokens.push(BodyInputToken::Text(&text[run_start..index]));
            }
            tokens.push(BodyInputToken::Newline);
            run_start = index + 1;
            continue;
        }
        let marker_index = (character as u32).wrapping_sub(PASTE_MARKER_BASE) as usize;
        if let Some(block) = pastes.get(marker_index) {
            if index > run_start {
                tokens.push(BodyInputToken::Text(&text[run_start..index]));
            }
            tokens.push(BodyInputToken::Paste(block));
            run_start = index + character.len_utf8();
        }
    }
    if text.len() > run_start {
        tokens.push(BodyInputToken::Text(&text[run_start..]));
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_original_starts_unselected_but_nonempty_starts_fully_selected() {
        assert!(!BodyInput::new(String::new()).selected);
        assert!(BodyInput::new("existing".into()).selected);
    }

    #[test]
    fn typing_over_a_selected_value_replaces_it() {
        let mut input = BodyInput::new("existing".into());
        input.handle_key(KeyCode::Char('Z'), KeyModifiers::NONE);
        assert_eq!(input.value, "Z");
        assert_eq!(input.cursor, 1);
        assert!(!input.selected);
    }

    #[test]
    fn enter_inserts_a_newline_instead_of_committing() {
        let mut input = BodyInput::new(String::new());
        input.handle_key(KeyCode::Char('a'), KeyModifiers::NONE);
        let outcome = input.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        input.handle_key(KeyCode::Char('b'), KeyModifiers::NONE);
        assert!(matches!(outcome, BodyInputOutcome::Continue));
        assert_eq!(input.value, "a\nb");
    }

    #[test]
    fn ctrl_d_and_ctrl_s_and_esc_report_the_right_outcome() {
        let mut input = BodyInput::new(String::new());
        assert!(matches!(
            input.handle_key(KeyCode::Char('d'), KeyModifiers::CONTROL),
            BodyInputOutcome::Commit
        ));
        assert!(matches!(
            input.handle_key(KeyCode::Char('s'), KeyModifiers::CONTROL),
            BodyInputOutcome::CommitAndSave
        ));
        assert!(matches!(
            input.handle_key(KeyCode::Esc, KeyModifiers::NONE),
            BodyInputOutcome::Cancel
        ));
    }

    #[test]
    fn a_short_single_line_paste_is_inserted_literally() {
        let mut input = BodyInput::new(String::new());
        input.insert_paste("hello".into());
        assert_eq!(input.value, "hello");
        assert_eq!(input.cursor, 5);
        assert!(input.pastes.is_empty());
    }

    #[test]
    fn a_large_paste_collapses_to_one_marker_char_and_resolves_back_to_the_full_text() {
        let mut input = BodyInput::new(String::new());
        let pasted = "line one\nline two\nline three".to_owned();
        input.insert_paste(pasted.clone());

        // Collapsed to exactly one char in `value`, not the whole pasted blob.
        assert_eq!(input.value.chars().count(), 1);
        assert_eq!(input.pastes.len(), 1);
        assert_eq!(input.pastes[0].lines, 3);
        assert_eq!(input.resolve(), pasted);
    }

    #[test]
    fn a_paste_over_a_selected_value_replaces_it_and_drops_stale_paste_blocks() {
        let mut input = BodyInput::new("existing".into());
        input.insert_paste("a\nb\nc\nd".into());
        assert_eq!(input.resolve(), "a\nb\nc\nd");
        assert_eq!(input.pastes.len(), 1);
    }

    #[test]
    fn backspace_removes_a_collapsed_paste_as_one_unit() {
        let mut input = BodyInput::new(String::new());
        input.insert_paste("x\ny\nz".into());
        input.handle_key(KeyCode::Char('!'), KeyModifiers::NONE);
        assert_eq!(input.value.chars().count(), 2);

        input.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(input.value.chars().count(), 1);
        input.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(input.value, "");
        // Body is now empty; the stale paste block is simply unreferenced.
        assert_eq!(input.resolve(), "");
    }

    #[test]
    fn a_long_single_line_paste_over_the_threshold_still_collapses() {
        let mut input = BodyInput::new(String::new());
        let pasted = "x".repeat(PASTE_COLLAPSE_CHARS + 1);
        input.insert_paste(pasted.clone());
        assert_eq!(input.value.chars().count(), 1);
        assert_eq!(input.resolve(), pasted);
    }

    #[test]
    fn tokenize_splits_text_newlines_and_pastes_into_separate_tokens() {
        let mut input = BodyInput::new(String::new());
        for character in "before ".chars() {
            input.handle_key(KeyCode::Char(character), KeyModifiers::NONE);
        }
        input.insert_paste("multi\nline".into());
        for character in " after".chars() {
            input.handle_key(KeyCode::Char(character), KeyModifiers::NONE);
        }
        input.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        for character in "next line".chars() {
            input.handle_key(KeyCode::Char(character), KeyModifiers::NONE);
        }

        let tokens = tokenize(&input.value, &input.pastes);
        let mut texts = Vec::new();
        let mut newlines = 0;
        let mut pastes = 0;
        for token in tokens {
            match token {
                BodyInputToken::Text(text) => texts.push(text),
                BodyInputToken::Newline => newlines += 1,
                BodyInputToken::Paste(block) => {
                    pastes += 1;
                    assert_eq!(block.lines, 2);
                }
            }
        }
        assert_eq!(texts, vec!["before ", " after", "next line"]);
        assert_eq!(newlines, 1);
        assert_eq!(pastes, 1);
    }
}
