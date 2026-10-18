//! Line contents and editing policy, independent of terminal I/O.

use alloc::string::String;
use core::fmt;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::input::Key;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineError {
    OutOfMemory,
    InvalidUtf8,
}

impl fmt::Display for LineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::OutOfMemory => "not enough memory to grow the input line",
            Self::InvalidUtf8 => "invalid UTF-8 input",
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Edit {
    None,
    Echo(char),
    Erase(usize),
    Submit(Result<String, LineError>),
    Cancel,
}

#[derive(Default)]
pub struct Editor {
    line: String,
    error: Option<LineError>,
}

impl Editor {
    pub fn feed(&mut self, key: Key) -> Edit {
        match key {
            Key::Enter => {
                let line = core::mem::take(&mut self.line);
                Edit::Submit(match self.error.take() {
                    Some(err) => Err(err),
                    None => Ok(line),
                })
            }
            Key::Cancel => {
                self.line.clear();
                self.error = None;
                Edit::Cancel
            }
            Key::ClearLine => {
                let width = self.line.width();
                self.line.clear();
                self.error = None;
                Edit::Erase(width)
            }
            _ if self.error.is_some() => Edit::None,
            Key::InvalidUtf8 => {
                self.error = Some(LineError::InvalidUtf8);
                Edit::None
            }
            Key::Character(ch) => {
                if self.line.try_reserve(ch.len_utf8()).is_err() {
                    self.error = Some(LineError::OutOfMemory);
                    return Edit::None;
                }
                self.line.push(ch);
                Edit::Echo(ch)
            }
            Key::Backspace => match self.line.grapheme_indices(true).next_back() {
                Some((start, grapheme)) => {
                    let width = grapheme.width();
                    self.line.truncate(start);
                    Edit::Erase(width)
                }
                None => Edit::None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_text(editor: &mut Editor, text: &str) {
        for ch in text.chars() {
            editor.feed(Key::Character(ch));
        }
    }

    #[test]
    fn deletes_ascii_without_erasing_past_start() {
        let mut editor = Editor::default();
        assert_eq!(editor.feed(Key::Backspace), Edit::None);
        type_text(&mut editor, "ab");
        assert_eq!(editor.feed(Key::Backspace), Edit::Erase(1));
        assert_eq!(editor.feed(Key::Enter), Edit::Submit(Ok(String::from("a"))));
        assert_eq!(editor.feed(Key::Enter), Edit::Submit(Ok(String::new())));
    }

    #[test]
    fn deletes_whole_graphemes_and_tracks_terminal_columns() {
        for (text, columns) in [("한", 2), ("e\u{301}", 1), ("👩‍💻", 2), ("한", 2)] {
            let mut editor = Editor::default();
            type_text(&mut editor, text);
            assert_eq!(editor.feed(Key::Backspace), Edit::Erase(columns), "{text}");
            assert_eq!(editor.feed(Key::Enter), Edit::Submit(Ok(String::new())));
        }
    }

    #[test]
    fn grows_to_hold_large_unicode_lines_without_a_fixed_limit() {
        let mut editor = Editor::default();
        let text = "한글🦀e\u{301} ".repeat(8192);
        assert!(text.len() > 64 * 1024);
        type_text(&mut editor, &text);
        assert_eq!(editor.feed(Key::Enter), Edit::Submit(Ok(text)));
        type_text(&mut editor, "next");
        assert_eq!(
            editor.feed(Key::Enter),
            Edit::Submit(Ok(String::from("next")))
        );
    }

    #[test]
    fn unicode_and_backspace_work_across_the_old_page_sized_limit() {
        let mut editor = Editor::default();
        let mut expected = "a".repeat(4095);
        type_text(&mut editor, &expected);
        editor.feed(Key::Character('한'));
        assert_eq!(editor.feed(Key::Backspace), Edit::Erase(2));
        editor.feed(Key::Character('글'));
        expected.push('글');
        assert_eq!(editor.feed(Key::Enter), Edit::Submit(Ok(expected)));
    }

    #[test]
    fn malformed_line_is_discarded_until_enter() {
        let mut editor = Editor::default();
        type_text(&mut editor, "exit");
        editor.feed(Key::InvalidUtf8);
        assert_eq!(editor.feed(Key::Character('x')), Edit::None);
        assert_eq!(
            editor.feed(Key::Enter),
            Edit::Submit(Err(LineError::InvalidUtf8))
        );
        assert_eq!(editor.feed(Key::Enter), Edit::Submit(Ok(String::new())));
    }

    #[test]
    fn clear_and_cancel_reset_text_and_error_state() {
        let mut editor = Editor::default();
        type_text(&mut editor, "한a");
        editor.feed(Key::InvalidUtf8);
        assert_eq!(editor.feed(Key::ClearLine), Edit::Erase(3));
        assert_eq!(editor.feed(Key::Enter), Edit::Submit(Ok(String::new())));
        type_text(&mut editor, "exit");
        editor.feed(Key::InvalidUtf8);
        assert_eq!(editor.feed(Key::Cancel), Edit::Cancel);
        type_text(&mut editor, "help");
        assert_eq!(
            editor.feed(Key::Enter),
            Edit::Submit(Ok(String::from("help")))
        );
    }
}
