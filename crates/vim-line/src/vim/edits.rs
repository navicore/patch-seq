//! Edit operations for `VimLineEditor`.
//!
//! These methods mutate `self.cursor` and `self.yank_buffer` and return
//! `EditResult`s the host applies to its text buffer. They live in a
//! separate `impl` block so the motion-only logic stays pure in `motions`
//! and `vim.rs` can focus on mode dispatch and the `LineEditor` trait.

use super::{Mode, Operator, VimLineEditor};
use crate::{EditResult, TextEdit};

impl VimLineEditor {
    /// Delete character at cursor (x).
    pub(super) fn delete_char(&mut self, text: &str) -> EditResult {
        if self.cursor >= text.len() {
            return EditResult::none();
        }

        let start = self.cursor;

        // Find the end of the current character
        let mut end = self.cursor + 1;
        while end < text.len() && !text.is_char_boundary(end) {
            end += 1;
        }

        let deleted = text[start..end].to_string();

        // If we're deleting the last character, move cursor left
        // (In Normal mode, cursor must always be ON a character)
        if end >= text.len() && self.cursor > 0 {
            self.move_left(text);
        }

        EditResult::edit_and_yank(TextEdit::Delete { start, end }, deleted)
    }

    /// Delete to end of line (D).
    pub(super) fn delete_to_end(&mut self, text: &str) -> EditResult {
        let end = text[self.cursor..]
            .find('\n')
            .map(|i| self.cursor + i)
            .unwrap_or(text.len());

        if self.cursor >= end {
            return EditResult::none();
        }

        let deleted = text[self.cursor..end].to_string();
        EditResult::edit_and_yank(
            TextEdit::Delete {
                start: self.cursor,
                end,
            },
            deleted,
        )
    }

    /// Delete entire current line (dd).
    pub(super) fn delete_line(&mut self, text: &str) -> EditResult {
        let line_start = text[..self.cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);

        let line_end = text[self.cursor..]
            .find('\n')
            .map(|i| self.cursor + i + 1) // Include the newline
            .unwrap_or(text.len());

        // If this is the only line and no newline, include leading newline if any
        let (start, end) = if line_start == 0 && line_end == text.len() {
            (0, text.len())
        } else if line_end == text.len() && line_start > 0 {
            // Last line - delete the preceding newline instead
            (line_start - 1, text.len())
        } else {
            (line_start, line_end)
        };

        let deleted = text[start..end].to_string();
        self.cursor = start;

        EditResult::edit_and_yank(TextEdit::Delete { start, end }, deleted)
    }

    /// Paste after cursor (p).
    pub(super) fn paste_after(&mut self, text: &str) -> EditResult {
        if self.yank_buffer.is_empty() {
            return EditResult::none();
        }

        let insert_pos = (self.cursor + 1).min(text.len());
        let to_insert = self.yank_buffer.clone();
        // Position cursor at end of pasted text, safely handling edge cases
        self.cursor = (insert_pos + to_insert.len())
            .saturating_sub(1)
            .min(text.len() + to_insert.len());

        EditResult::edit(TextEdit::Insert {
            at: insert_pos,
            text: to_insert,
        })
    }

    /// Paste before cursor (P).
    pub(super) fn paste_before(&mut self, text: &str) -> EditResult {
        if self.yank_buffer.is_empty() {
            return EditResult::none();
        }

        let to_insert = self.yank_buffer.clone();
        let insert_pos = self.cursor.min(text.len());
        // Position cursor at end of pasted text
        self.cursor = (insert_pos + to_insert.len()).min(text.len() + to_insert.len());

        EditResult::edit(TextEdit::Insert {
            at: insert_pos,
            text: to_insert,
        })
    }

    /// Apply an operator to a range.
    pub(super) fn apply_operator(
        &mut self,
        op: Operator,
        start: usize,
        end: usize,
        text: &str,
    ) -> EditResult {
        let affected = text[start..end].to_string();
        self.yank_buffer = affected.clone();
        self.cursor = start;

        match op {
            Operator::Delete => {
                EditResult::edit_and_yank(TextEdit::Delete { start, end }, affected)
            }
            Operator::Change => {
                self.mode = Mode::Insert;
                EditResult::edit_and_yank(TextEdit::Delete { start, end }, affected)
            }
            Operator::Yank => {
                // Just yank, no edit
                EditResult {
                    yanked: Some(affected),
                    ..Default::default()
                }
            }
        }
    }

    /// Apply an operator to the whole line.
    pub(super) fn apply_operator_line(&mut self, op: Operator, text: &str) -> EditResult {
        match op {
            Operator::Delete => self.delete_line(text),
            Operator::Change => {
                let result = self.delete_line(text);
                self.mode = Mode::Insert;
                result
            }
            Operator::Yank => {
                let line_start = text[..self.cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);
                let line_end = text[self.cursor..]
                    .find('\n')
                    .map(|i| self.cursor + i + 1)
                    .unwrap_or(text.len());
                let line = text[line_start..line_end].to_string();
                self.yank_buffer = line.clone();
                EditResult {
                    yanked: Some(line),
                    ..Default::default()
                }
            }
        }
    }
}
