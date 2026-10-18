//! Built-ins run in the shell task and need no process-management syscalls.

use alloc::string::String;
use core::fmt::{self, Write};

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Continue,
    Exit(isize),
}

const COMMANDS: &[(&str, &str, &str)] = &[
    ("help", "help [command]", "show commands or command usage"),
    (
        "echo",
        "echo [text ...]",
        "print arguments separated by spaces",
    ),
    ("clear", "clear", "clear the terminal screen"),
    ("exit", "exit [code]", "exit the shell (default code: 0)"),
];

pub fn execute(words: &[String], out: &mut impl Write) -> Result<Outcome, fmt::Error> {
    let Some((name, args)) = words.split_first() else {
        return Ok(Outcome::Continue);
    };
    let Some(&(_, usage, description)) = COMMANDS.iter().find(|&&(cmd, _, _)| cmd == name) else {
        writeln!(out, "unknown command: {name} (try 'help')")?;
        return Ok(Outcome::Continue);
    };

    if args.len() == 1 && matches!(args[0].as_str(), "-h" | "--help") {
        writeln!(out, "usage: {usage}\n{description}")?;
        return Ok(Outcome::Continue);
    }

    match name.as_str() {
        "help" if args.is_empty() => {
            for (_, usage, description) in COMMANDS {
                writeln!(out, "  {usage:18} {description}")?;
            }
            writeln!(
                out,
                "\nEditing: Backspace/DEL, Ctrl-U to clear, Ctrl-C to cancel."
            )?;
            writeln!(
                out,
                "Arguments support quotes and backslash escapes; no expansion."
            )?;
        }
        "help" if args.len() == 1 => match COMMANDS.iter().find(|&&(cmd, _, _)| cmd == args[0]) {
            Some((_, usage, description)) => writeln!(out, "usage: {usage}\n{description}")?,
            None => writeln!(out, "unknown command: {}", args[0])?,
        },
        "echo" => {
            for (index, arg) in args.iter().enumerate() {
                if index != 0 {
                    out.write_char(' ')?;
                }
                out.write_str(arg)?;
            }
            writeln!(out)?;
        }
        "clear" if args.is_empty() => out.write_str("\x1b[2J\x1b[H")?,
        "exit" if args.len() <= 1 => {
            let code = match args.first() {
                None => 0,
                Some(arg) => match arg.parse::<isize>() {
                    Ok(code) => code,
                    Err(_) => {
                        writeln!(out, "exit: code must be a signed integer")?;
                        return Ok(Outcome::Continue);
                    }
                },
            };
            writeln!(out, "bye")?;
            return Ok(Outcome::Exit(code));
        }
        _ => writeln!(out, "usage: {usage}")?,
    }
    Ok(Outcome::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn run(line: &str) -> (Outcome, String) {
        let mut output = String::new();
        let outcome = execute(&parser::parse(line).unwrap(), &mut output).unwrap();
        (outcome, output)
    }

    #[test]
    fn blank_input_is_silent() {
        assert_eq!(run("  "), (Outcome::Continue, String::new()));
    }

    #[test]
    fn echo_prints_parsed_arguments_and_empty_lines() {
        assert_eq!(run("echo 'hello world' 한글").1, "hello world 한글\n");
        assert_eq!(run("echo '' x").1, " x\n");
        assert_eq!(run("echo").1, "\n");
    }

    #[test]
    fn exit_validates_arguments_before_exiting() {
        assert_eq!(run("exit"), (Outcome::Exit(0), String::from("bye\n")));
        assert_eq!(run("exit -42").0, Outcome::Exit(-42));
        for line in [
            "exit nope",
            "exit 1 2",
            "exit 999999999999999999999999999999999999999",
        ] {
            assert_eq!(run(line).0, Outcome::Continue);
            assert!(!run(line).1.contains("bye"));
        }
    }

    #[test]
    fn help_lists_real_commands_and_supports_per_command_help() {
        let output = run("help").1;
        for (_, usage, _) in COMMANDS {
            assert!(output.contains(usage));
        }
        assert_eq!(run("help exit").1, run("exit --help").1);
        assert!(run("help missing").1.contains("unknown command: missing"));
        assert!(run("help a b").1.starts_with("usage:"));
    }

    #[test]
    fn clear_and_unknown_commands_do_not_exit() {
        assert_eq!(
            run("clear"),
            (Outcome::Continue, String::from("\x1b[2J\x1b[H"))
        );
        assert!(run("clear extra").1.starts_with("usage:"));
        assert_eq!(run("missing").0, Outcome::Continue);
        assert!(run("missing").1.contains("unknown command: missing"));
    }

    #[test]
    fn propagates_output_failure() {
        struct Broken;
        impl Write for Broken {
            fn write_str(&mut self, _: &str) -> fmt::Result {
                Err(fmt::Error)
            }
        }
        assert!(execute(&parser::parse("exit").unwrap(), &mut Broken).is_err());
    }
}
