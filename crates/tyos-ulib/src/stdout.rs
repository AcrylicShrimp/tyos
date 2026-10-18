pub struct Stdin;

impl Stdin {
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, isize> {
        let result = tyos_syscall::read(0, buf.as_mut_ptr(), buf.len());

        if result < 0 {
            Err(result)
        } else {
            Ok(result as usize)
        }
    }
}

pub struct Stdout;

impl core::fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let result = tyos_syscall::write(1, s.as_ptr(), s.len());

        if result < 0 {
            Err(core::fmt::Error)
        } else {
            Ok(())
        }
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let _ = write!($crate::Stdout, $($arg)*);
    });
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n");
    };
    ($($arg:tt)*) => {
        $crate::print!("{}\n", format_args!($($arg)*));
    };
}
