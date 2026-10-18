//! Incremental UART decoding. State survives read boundaries and command lines.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Character(char),
    Backspace,
    Enter,
    ClearLine,
    Cancel,
    InvalidUtf8,
}

#[derive(Default)]
enum Escape {
    #[default]
    None,
    Start,
    Csi,
    Ss3,
    Osc {
        escaped: bool,
    },
}

#[derive(Default)]
pub struct Decoder {
    utf8: [u8; 4],
    len: usize,
    expected: usize,
    skip_lf: bool,
    escape: Escape,
}

impl Decoder {
    // A malformed UTF-8 prefix can be followed by a useful byte, e.g. Enter.
    // Return both events so invalid input cannot swallow the line terminator.
    pub fn feed(&mut self, byte: u8) -> [Option<Key>; 2] {
        if self.skip_lf {
            self.skip_lf = false;
            if byte == b'\n' {
                return [None, None];
            }
        }

        if matches!(byte, b'\r' | b'\n' | 0x03 | 0x15) {
            self.escape = Escape::None;
        }

        if !matches!(self.escape, Escape::None) {
            self.escape = match self.escape {
                Escape::Start => match byte {
                    b'[' => Escape::Csi,
                    b'O' => Escape::Ss3,
                    b']' => Escape::Osc { escaped: false },
                    _ => Escape::None,
                },
                Escape::Csi | Escape::Ss3 if (0x40..=0x7e).contains(&byte) => Escape::None,
                Escape::Osc { escaped } => {
                    if byte == 0x07 || (escaped && byte == b'\\') {
                        Escape::None
                    } else {
                        Escape::Osc {
                            escaped: byte == 0x1b,
                        }
                    }
                }
                _ => Escape::Csi,
            };
            return [None, None];
        }

        if self.len == 0 {
            return [self.start(byte), None];
        }

        if !(0x80..=0xbf).contains(&byte) {
            self.len = 0;
            return [Some(Key::InvalidUtf8), self.start(byte)];
        }

        self.utf8[self.len] = byte;
        self.len += 1;
        if self.len < self.expected {
            return [None, None];
        }

        let key = match core::str::from_utf8(&self.utf8[..self.len]) {
            Ok(text) => text
                .chars()
                .next()
                .filter(|ch| !ch.is_control())
                .map(Key::Character),
            Err(_) => Some(Key::InvalidUtf8),
        };
        self.len = 0;
        [key, None]
    }

    fn start(&mut self, byte: u8) -> Option<Key> {
        match byte {
            b'\r' => {
                self.skip_lf = true;
                Some(Key::Enter)
            }
            b'\n' => Some(Key::Enter),
            0x08 | 0x7f => Some(Key::Backspace),
            0x03 => Some(Key::Cancel),
            0x15 => Some(Key::ClearLine),
            b'\t' => Some(Key::Character(' ')),
            0x1b => {
                self.escape = Escape::Start;
                None
            }
            0x20..=0x7e => Some(Key::Character(char::from(byte))),
            0xc2..=0xf4 => {
                self.expected = match byte {
                    0xc2..=0xdf => 2,
                    0xe0..=0xef => 3,
                    _ => 4,
                };
                self.utf8[0] = byte;
                self.len = 1;
                None
            }
            0x80..=0xff => Some(Key::InvalidUtf8),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{vec, vec::Vec};

    fn decode(bytes: &[u8]) -> Vec<Key> {
        let mut decoder = Decoder::default();
        bytes
            .iter()
            .flat_map(|&byte| decoder.feed(byte).into_iter().flatten())
            .collect()
    }

    #[test]
    fn decodes_utf8_one_byte_at_a_time() {
        assert_eq!(
            decode("a한🦀".as_bytes()),
            vec![
                Key::Character('a'),
                Key::Character('한'),
                Key::Character('🦀')
            ]
        );
    }

    #[test]
    fn normalizes_controls_and_crlf() {
        assert_eq!(
            decode(b"\r\n\n\r\x08\x7f\x03\x15\t"),
            vec![
                Key::Enter,
                Key::Enter,
                Key::Enter,
                Key::Backspace,
                Key::Backspace,
                Key::Cancel,
                Key::ClearLine,
                Key::Character(' ')
            ]
        );
    }

    #[test]
    fn invalid_prefix_does_not_swallow_enter_or_next_character() {
        assert_eq!(
            decode(b"\xe3\na"),
            vec![Key::InvalidUtf8, Key::Enter, Key::Character('a')]
        );
        assert_eq!(
            decode(b"\xe3a"),
            vec![Key::InvalidUtf8, Key::Character('a')]
        );
    }

    #[test]
    fn rejects_overlong_surrogates_and_out_of_range_codepoints() {
        for bytes in [
            &b"\xe0\x80\x80"[..],
            &b"\xed\xa0\x80"[..],
            &b"\xf4\x90\x80\x80"[..],
            &b"\xff"[..],
        ] {
            assert_eq!(decode(bytes), vec![Key::InvalidUtf8]);
        }
    }

    #[test]
    fn escape_sequences_and_control_characters_do_not_become_command_text() {
        assert_eq!(
            decode(b"a\x1b[A\x1b[3~\x1bOP\x1b]0;title\x07\x1b]title\x1b\\b\x00\x04"),
            vec![Key::Character('a'), Key::Character('b')]
        );
        assert_eq!(
            decode(b"\x1b[\x03x"),
            vec![Key::Cancel, Key::Character('x')]
        );
    }
}
