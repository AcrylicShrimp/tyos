//! Foreground process execution, independent of the syscall transport.

use core::fmt::{self, Write};

pub trait Processes {
    fn spawn(&mut self, program: &str, arg: usize) -> Result<usize, isize>;
    /// Returns the raw wait status. The current ABI also uses -1 for failure,
    /// so the shell must not interpret every negative status as an error.
    fn wait(&mut self, pid: usize) -> isize;
}

pub(crate) fn run(
    processes: &mut impl Processes,
    program: &str,
    arg: usize,
    out: &mut impl Write,
) -> fmt::Result {
    let pid = match processes.spawn(program, arg) {
        Ok(pid) => pid,
        Err(code) => return writeln!(out, "run: cannot spawn '{program}' ({code})"),
    };
    let status = processes.wait(pid);
    writeln!(out, "process {pid} status: {status}")
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use alloc::{string::String, vec, vec::Vec};

    #[derive(Debug, PartialEq, Eq)]
    pub enum Call {
        Spawn(String, usize),
        Wait(usize),
    }

    pub struct FakeProcesses {
        pub spawn_result: Result<usize, isize>,
        pub status: isize,
        pub calls: Vec<Call>,
    }

    impl Default for FakeProcesses {
        fn default() -> Self {
            Self {
                spawn_result: Ok(2),
                status: 0,
                calls: Vec::new(),
            }
        }
    }

    impl Processes for FakeProcesses {
        fn spawn(&mut self, program: &str, arg: usize) -> Result<usize, isize> {
            self.calls.push(Call::Spawn(program.into(), arg));
            self.spawn_result
        }

        fn wait(&mut self, pid: usize) -> isize {
            self.calls.push(Call::Wait(pid));
            self.status
        }
    }

    #[test]
    fn waits_for_the_spawned_pid_and_preserves_signed_status() {
        for status in [0, 7, -1, -44] {
            let mut processes = FakeProcesses {
                status,
                ..FakeProcesses::default()
            };
            let mut output = String::new();
            run(&mut processes, "example", 42, &mut output).unwrap();
            assert_eq!(
                processes.calls,
                vec![Call::Spawn("example".into(), 42), Call::Wait(2)]
            );
            assert_eq!(output, alloc::format!("process 2 status: {status}\n"));
        }
    }

    #[test]
    fn failed_spawn_reports_the_error_without_waiting() {
        let mut processes = FakeProcesses {
            spawn_result: Err(-1),
            ..FakeProcesses::default()
        };
        let mut output = String::new();
        run(&mut processes, "missing", 0, &mut output).unwrap();
        assert_eq!(processes.calls, vec![Call::Spawn("missing".into(), 0)]);
        assert_eq!(output, "run: cannot spawn 'missing' (-1)\n");
    }

    #[test]
    fn propagates_status_output_failure() {
        struct Broken;
        impl Write for Broken {
            fn write_str(&mut self, _: &str) -> fmt::Result {
                Err(fmt::Error)
            }
        }
        assert!(run(&mut FakeProcesses::default(), "example", 0, &mut Broken).is_err());
    }
}
