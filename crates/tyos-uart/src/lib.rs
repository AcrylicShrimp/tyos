#![no_std]

pub struct Uart;

impl Uart {
    const BASE: *mut u8 = 0xFFFF_FFC0_1000_0000 as *mut u8;
    const LSR: *mut u8 = 0xFFFF_FFC0_1000_0005 as *mut u8;
}

impl Uart {
    pub fn try_read_byte(self) -> Option<u8> {
        let status = unsafe { core::ptr::read_volatile(Self::LSR) };

        if status & 0b0000_0001 == 0 {
            None
        } else {
            Some(unsafe { core::ptr::read_volatile(Self::BASE) })
        }
    }
}

impl core::fmt::Write for Uart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.bytes() {
            unsafe {
                core::ptr::write_volatile(Self::BASE, c);
            }
        }

        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let _ = write!($crate::Uart, $($arg)*);
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
