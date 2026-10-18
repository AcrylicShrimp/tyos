#![no_std]
#![no_main]

use core::fmt;
use tyos_ulib::{Stdin, Stdout, entry_point};

struct SystemConsole;
struct SystemProcesses;
struct SystemPower;

impl shell::PowerControl for SystemPower {
    fn shutdown(&mut self) -> ! {
        tyos_ulib::shutdown()
    }
}

impl shell::Processes for SystemProcesses {
    fn spawn(&mut self, program: &str, arg: usize) -> Result<usize, isize> {
        let result = tyos_ulib::spawn(program.as_ptr(), program.len(), arg);
        if result < 0 {
            Err(result)
        } else {
            Ok(result as usize)
        }
    }

    fn wait(&mut self, pid: usize) -> isize {
        tyos_ulib::wait(pid)
    }
}

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
fn main(_arg0: usize) -> isize {
    shell::run(&mut SystemConsole, &mut SystemProcesses, &mut SystemPower)
}
