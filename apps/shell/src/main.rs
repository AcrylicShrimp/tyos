#![no_std]
#![no_main]

extern crate alloc;

use tyos_ulib::{Stdin, entry_point, print, println};

#[entry_point]
fn main() -> isize {
    let mut line_reader = LineReader::new();

    loop {
        print!("(shell)> ");

        let input = match line_reader.read_line() {
            Ok(input) => input.trim(),
            Err(err) => {
                println!("error: {err}");
                println!();
                continue;
            }
        };

        println!();

        match input {
            "exit" => {
                println!("bye");
                break;
            }
            _ => {
                println!("unrecognized command: {}", input);
                println!();
            }
        }
    }

    0
}

struct LineReader {
    buf: [u8; 4096],
    buf_pos: usize,
    line_pos: usize,
    echo_pos: usize,
    in_recovery_mode: bool,
}

impl LineReader {
    pub fn new() -> Self {
        Self {
            buf: [0u8; 4096],
            buf_pos: 0,
            line_pos: 0,
            echo_pos: 0,
            in_recovery_mode: false,
        }
    }

    pub fn read_line(&mut self) -> Result<&str, &'static str> {
        self.trim_previous_line();

        while !self.read_and_mark_line()? {}

        core::str::from_utf8(&self.buf[..self.line_pos])
            .map_err(|_| "failed to convert the input to a valid utf-8 string")
    }

    fn trim_previous_line(&mut self) {
        self.buf.copy_within(self.line_pos..self.buf_pos, 0);
        self.buf_pos -= self.line_pos;
        self.echo_pos -= self.line_pos;
        self.line_pos = 0;
    }

    fn read_and_mark_line(&mut self) -> Result<bool, &'static str> {
        if self.line_pos != 0 {
            // a line is already present and marked; return as marked
            return Ok(true);
        }

        let len = Stdin
            .read(&mut self.buf[self.buf_pos..])
            .expect("failed to read from the stdin");
        self.buf_pos += len;
        self.echo();
        self.line_pos = match self.buf[..self.buf_pos]
            .iter()
            .position(|ch| *ch == b'\n' || *ch == b'\r')
        {
            Some(pos) => pos + 1,
            None if self.buf_pos == self.buf.len() => {
                let not_echoed_len = self.buf_pos - self.echo_pos;
                self.buf.copy_within(self.echo_pos..self.buf_pos, 0);
                self.buf_pos = not_echoed_len;
                self.echo_pos = 0;
                self.line_pos = 0;

                if self.in_recovery_mode {
                    return Ok(false);
                }

                self.in_recovery_mode = true;

                return Err("the input buffer is full");
            }
            None => {
                return Ok(false);
            }
        };

        if self.in_recovery_mode {
            self.trim_previous_line();
            self.in_recovery_mode = false;
            return Ok(false);
        }

        Ok(true)
    }

    fn echo(&mut self) {
        let echo_target = &self.buf[self.echo_pos..self.buf_pos];
        let valid_echo_target = match core::str::from_utf8(echo_target) {
            Ok(valid) => valid,
            Err(err) => unsafe {
                core::str::from_utf8_unchecked(&echo_target[..err.valid_up_to()])
            },
        };

        print!("{valid_echo_target}");
        self.echo_pos += valid_echo_target.len();
    }
}
