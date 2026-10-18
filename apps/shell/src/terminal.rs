//! Incremental console transport and terminal output, without command policy.

use core::fmt::{self, Write};

use crate::input::{Decoder, Key};

pub trait Console: Write {
    /// Nonblocking read: Ok(0) means no input is currently available.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, isize>;
}

pub struct Terminal<'a, C> {
    console: &'a mut C,
    decoder: Decoder,
    pending: Option<Key>,
}

impl<'a, C: Console> Terminal<'a, C> {
    pub fn new(console: &'a mut C) -> Self {
        Self {
            console,
            decoder: Decoder::default(),
            pending: None,
        }
    }

    pub fn read_key(&mut self) -> Result<Key, isize> {
        if let Some(key) = self.pending.take() {
            return Ok(key);
        }
        // Do not prefetch bytes past Enter: the next reader may be a child
        // process with its own console. Line text still grows in the editor.
        let mut byte = [0];
        loop {
            if self.console.read(&mut byte)? == 0 {
                continue;
            }
            let [first, second] = self.decoder.feed(byte[0]);
            self.pending = second;
            if let Some(key) = first {
                return Ok(key);
            }
        }
    }

    pub fn erase(&mut self, columns: usize) -> fmt::Result {
        // End-of-line editing on a single terminal row; wrapping needs terminal
        // dimensions and cursor tracking, which this editor does not yet have.
        if columns != 0 {
            write!(self.console, "\x1b[{columns}D\x1b[0K")?;
        }
        Ok(())
    }
}

impl<C: Console> Write for Terminal<'_, C> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        // UART is raw; move to column zero as well as advancing the row.
        for segment in text.split_inclusive('\n') {
            if let Some(line) = segment.strip_suffix('\n') {
                self.console.write_str(line)?;
                self.console.write_str("\r\n")?;
            } else {
                self.console.write_str(segment)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use alloc::{string::String, vec::Vec};

    pub struct FakeConsole {
        pub input: Vec<u8>,
        pub output: String,
        pub chunk: usize,
        pub fail_write: bool,
        pos: usize,
        empty_reads: usize,
    }

    impl FakeConsole {
        pub fn new(input: &[u8], chunk: usize) -> Self {
            Self {
                input: input.into(),
                output: String::new(),
                chunk,
                fail_write: false,
                pos: 0,
                empty_reads: 0,
            }
        }
    }

    impl Write for FakeConsole {
        fn write_str(&mut self, text: &str) -> fmt::Result {
            if self.fail_write {
                return Err(fmt::Error);
            }
            self.output.push_str(text);
            Ok(())
        }
    }

    impl Console for FakeConsole {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, isize> {
            if self.empty_reads != 0 {
                self.empty_reads -= 1;
                return Ok(0);
            }
            if self.pos == self.input.len() {
                return Err(-9);
            }
            let len = self.chunk.min(buf.len()).min(self.input.len() - self.pos);
            buf[..len].copy_from_slice(&self.input[self.pos..self.pos + len]);
            self.pos += len;
            Ok(len)
        }
    }

    #[test]
    fn reads_keep_bytes_after_enter_in_the_console() {
        let mut console = FakeConsole::new(b"a\nb\n", 128);
        {
            let mut terminal = Terminal::new(&mut console);
            assert_eq!(terminal.read_key(), Ok(Key::Character('a')));
            assert_eq!(terminal.read_key(), Ok(Key::Enter));
        }
        let mut remaining = [0; 128];
        assert_eq!(console.read(&mut remaining), Ok(2));
        assert_eq!(&remaining[..2], b"b\n");
    }

    #[test]
    fn empty_reads_and_split_utf8_keep_decoder_state() {
        let mut console = FakeConsole::new("한\r\nx".as_bytes(), 1);
        console.empty_reads = 3;
        let mut terminal = Terminal::new(&mut console);
        assert_eq!(terminal.read_key(), Ok(Key::Character('한')));
        assert_eq!(terminal.read_key(), Ok(Key::Enter));
        assert_eq!(terminal.read_key(), Ok(Key::Character('x')));
    }

    #[test]
    fn preserves_enter_after_incomplete_utf8_prefix() {
        let mut console = FakeConsole::new(b"\xe3\n", 128);
        let mut terminal = Terminal::new(&mut console);
        assert_eq!(terminal.read_key(), Ok(Key::InvalidUtf8));
        assert_eq!(terminal.read_key(), Ok(Key::Enter));
    }

    #[test]
    fn formats_newlines_and_erases_display_columns() {
        let mut console = FakeConsole::new(b"", 128);
        let mut terminal = Terminal::new(&mut console);
        terminal.write_str("a\n한\n").unwrap();
        terminal.erase(2).unwrap();
        terminal.erase(0).unwrap();
        assert_eq!(console.output, "a\r\n한\r\n\x1b[2D\x1b[0K");
    }

    #[test]
    fn propagates_write_errors() {
        let mut console = FakeConsole::new(b"", 128);
        console.fail_write = true;
        let mut terminal = Terminal::new(&mut console);
        assert!(terminal.write_str("x\n").is_err());
        assert!(terminal.erase(1).is_err());
    }
}
