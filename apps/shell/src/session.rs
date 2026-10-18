//! Read/edit/parse/execute orchestration over console and process interfaces.

use core::fmt::{self, Write};

use crate::{
    commands::{self, Outcome},
    editor::{Edit, Editor},
    parser,
    process::{self, Processes},
    system::{self, PowerControl},
    terminal::{Console, Terminal},
};

const PROMPT: &str = "(shell)> ";

pub fn run(
    console: &mut impl Console,
    processes: &mut impl Processes,
    power: &mut impl PowerControl,
) -> isize {
    run_session(&mut Terminal::new(console), processes, power).unwrap_or(1)
}

fn run_session(
    terminal: &mut Terminal<'_, impl Console>,
    processes: &mut impl Processes,
    power: &mut impl PowerControl,
) -> Result<isize, fmt::Error> {
    let mut editor = Editor::default();
    terminal.write_str(PROMPT)?;
    loop {
        let key = match terminal.read_key() {
            Ok(key) => key,
            Err(code) => {
                writeln!(terminal, "\nerror: stdin read failed ({code})")?;
                return Ok(1);
            }
        };
        match editor.feed(key) {
            Edit::None => {}
            Edit::Echo(ch) => write!(terminal, "{ch}")?,
            Edit::Erase(columns) => terminal.erase(columns)?,
            Edit::Cancel => {
                writeln!(terminal, "^C")?;
                terminal.write_str(PROMPT)?;
            }
            Edit::Submit(line) => {
                writeln!(terminal)?;
                match line {
                    Err(err) => writeln!(terminal, "error: {err}")?,
                    Ok(line) => match parser::parse(&line) {
                        Err(err) => writeln!(terminal, "error: {err}")?,
                        Ok(words) => match commands::execute(&words, terminal)? {
                            Outcome::Continue => {}
                            Outcome::Exit(code) => return Ok(code),
                            Outcome::PowerOff => system::poweroff(power, terminal)?,
                            Outcome::Run { program, arg } => {
                                process::run(processes, &program, arg, terminal)?
                            }
                        },
                    },
                }
                terminal.write_str(PROMPT)?;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::tests::{Call, FakeProcesses};
    use crate::system::tests::FakePower;
    use crate::terminal::tests::FakeConsole;
    use alloc::{format, vec::Vec};

    #[test]
    fn scripts_work_for_every_uart_chunk_size() {
        let script = "\necho 'hello world' 한글\r\nhelp echo\nexit 7\n";
        for chunk in 1..=128 {
            let mut console = FakeConsole::new(script.as_bytes(), chunk);
            assert_eq!(
                run(&mut console, &mut FakeProcesses::default(), &mut FakePower),
                7,
                "chunk={chunk}"
            );
            assert_eq!(console.output.matches(PROMPT).count(), 4, "chunk={chunk}");
            assert!(console.output.contains("\r\nhello world 한글\r\n"));
            assert!(console.output.contains("usage: echo [text ...]"));
            assert!(console.output.ends_with("bye\r\n"));
        }
    }

    #[test]
    fn editing_and_cancel_change_the_command_that_runs() {
        let mut console = FakeConsole::new(
            "\u{8}echo ab\u{7f}한\u{8}c\nexit\u{3}echo bad\u{15}echo good\nexit\n".as_bytes(),
            128,
        );
        assert_eq!(
            run(&mut console, &mut FakeProcesses::default(), &mut FakePower),
            0
        );
        assert!(console.output.contains("\r\nac\r\n"));
        assert!(console.output.contains("\x1b[2D\x1b[0K"));
        assert!(console.output.contains("^C\r\n"));
        assert!(console.output.contains("\r\ngood\r\n"));
        assert_eq!(console.output.matches("bye\r\n").count(), 1);
    }

    #[test]
    fn malformed_lines_recover_without_executing_suffixes() {
        let mut script = Vec::from(&b"exit\xff\nexit '\n"[..]);
        script.extend_from_slice(b"echo recovered\nexit\n");
        for chunk in [1, 7, 128] {
            let mut console = FakeConsole::new(&script, chunk);
            assert_eq!(
                run(&mut console, &mut FakeProcesses::default(), &mut FakePower),
                0
            );
            for message in ["error: invalid UTF-8 input", "error: unclosed ' quote"] {
                assert_eq!(console.output.matches(message).count(), 1);
            }
            assert!(console.output.contains("\r\nrecovered\r\n"));
            assert_eq!(console.output.matches("bye\r\n").count(), 1);
        }
    }

    #[test]
    fn large_pastes_survive_chunk_boundaries_and_leave_the_next_command_intact() {
        let text = "abc한글🦀 ".repeat(8192);
        let script = format!("echo '{text}'\r\necho next\nexit\n");
        for chunk in [1, 7, 128] {
            let mut console = FakeConsole::new(script.as_bytes(), chunk);
            assert_eq!(
                run(&mut console, &mut FakeProcesses::default(), &mut FakePower),
                0
            );
            assert!(console.output.contains(&format!("\r\n{text}\r\n")));
            assert!(console.output.contains("\r\nnext\r\n"));
            assert_eq!(console.output.matches(PROMPT).count(), 3);
            assert!(!console.output.contains("error:"));
        }
    }

    #[test]
    fn read_and_write_failures_return_nonzero_instead_of_panicking() {
        let mut console = FakeConsole::new(b"", 1);
        assert_eq!(
            run(&mut console, &mut FakeProcesses::default(), &mut FakePower),
            1
        );
        assert!(console.output.contains("stdin read failed (-9)"));
        console.fail_write = true;
        assert_eq!(
            run(&mut console, &mut FakeProcesses::default(), &mut FakePower),
            1
        );
    }

    #[test]
    fn run_waits_and_prints_status_before_resuming_commands() {
        let mut console = FakeConsole::new(b"run example 42\necho parent\nexit 3\n", 128);
        let mut processes = FakeProcesses {
            status: -7,
            ..FakeProcesses::default()
        };
        assert_eq!(run(&mut console, &mut processes, &mut FakePower), 3);
        assert_eq!(
            processes.calls,
            alloc::vec![Call::Spawn("example".into(), 42), Call::Wait(2)]
        );
        assert!(
            console
                .output
                .contains("process 2 status: -7\r\n(shell)> echo parent\r\nparent\r\n")
        );
    }

    #[test]
    fn invalid_run_never_spawns_and_spawn_failure_keeps_shell_alive() {
        let mut console = FakeConsole::new(b"run example -1\nrun missing\necho alive\nexit\n", 128);
        let mut processes = FakeProcesses {
            spawn_result: Err(-1),
            ..FakeProcesses::default()
        };
        assert_eq!(run(&mut console, &mut processes, &mut FakePower), 0);
        assert_eq!(
            processes.calls,
            alloc::vec![Call::Spawn("missing".into(), 0)]
        );
        assert!(
            console
                .output
                .contains("run: arg must be an unsigned decimal integer")
        );
        assert!(
            console
                .output
                .contains("run: cannot spawn 'missing' (-1)\r\n(shell)> echo alive\r\nalive\r\n")
        );
    }

    #[test]
    #[should_panic(expected = "test shutdown requested")]
    fn poweroff_reaches_system_control() {
        let mut console = FakeConsole::new(b"poweroff\n", 128);
        run(&mut console, &mut FakeProcesses::default(), &mut FakePower);
    }

    #[test]
    fn poweroff_help_and_invalid_arguments_keep_the_shell_running() {
        let mut console =
            FakeConsole::new(b"poweroff --help\npoweroff extra\necho alive\nexit\n", 128);
        assert_eq!(
            run(&mut console, &mut FakeProcesses::default(), &mut FakePower),
            0
        );
        assert!(console.output.contains("\r\nalive\r\n"));
        assert!(!console.output.contains("powering off"));
    }
}
