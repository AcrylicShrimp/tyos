//! System power control, separate from shell exit and process management.

use core::fmt::{self, Write};

pub trait PowerControl {
    /// Shuts down the whole system using the kernel's non-returning interface.
    fn shutdown(&mut self) -> !;
}

pub(crate) fn poweroff(power: &mut impl PowerControl, out: &mut impl Write) -> fmt::Result {
    writeln!(out, "powering off")?;
    power.shutdown()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use alloc::string::String;

    pub struct FakePower;

    impl PowerControl for FakePower {
        fn shutdown(&mut self) -> ! {
            panic!("test shutdown requested")
        }
    }

    #[test]
    #[should_panic(expected = "test shutdown requested")]
    fn poweroff_calls_the_nonreturning_shutdown_interface() {
        poweroff(&mut FakePower, &mut String::new()).unwrap();
    }

    #[test]
    fn output_failure_returns_without_requesting_shutdown() {
        struct Broken;
        impl Write for Broken {
            fn write_str(&mut self, _: &str) -> fmt::Result {
                Err(fmt::Error)
            }
        }
        assert!(poweroff(&mut FakePower, &mut Broken).is_err());
    }
}
