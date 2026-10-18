#![no_std]
#![no_main]

use core::fmt;
use tyos_ulib::{Stdin, Stdout, entry_point};

struct SystemConsole;

impl shell::Console for SystemConsole {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, isize> {
        Stdin.read(buf)
    }
}

impl fmt::Write for SystemConsole {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        Stdout.write_str(text)
    }
}

#[entry_point]
fn main() -> isize {
    shell::run(&mut SystemConsole)
}
