//! Host integration test of the built shell through QEMU's UART/syscall path.
//!
//! From the repository root (use your Rust host triple if it differs):
//! ```text
//! cargo build --locked -p shell -p hello-world-alpha -p hello-world-beta
//! cargo build --locked -p tyos
//! cargo test --locked -p shell --no-default-features --target aarch64-apple-darwin --test qemu_smoke -- --ignored --nocapture
//! ```
//! Normal host tests use the same `--no-default-features` and `--target` options.
//! The runtime feature is enabled by default for building the bare-metal app.
//! The kernel must start the shell. This test does not modify kernel sources.

#[allow(dead_code)]
mod support;

use support::Qemu;

#[test]
#[ignore = "requires QEMU and freshly built RISC-V apps/kernel; see module instructions"]
fn shell_uart_smoke() {
    let mut qemu = Qemu::start();
    qemu.check(
        "help",
        b"help\n",
        &[b"help [command]", b"Ctrl-U", b"Ctrl-C"],
        1,
        64,
    );
    qemu.check(
        "quoted Unicode arguments",
        "echo \"hello world\" '한 글'\r\n".as_bytes(),
        &["\r\nhello world 한 글\r\n".as_bytes()],
        1,
        64,
    );
    qemu.check(
        "both backspace encodings",
        b"\x08echo abc\x08X\x7fY\n",
        &[b"\r\nabY\r\n", b"\x1b[1D\x1b[0K"],
        1,
        64,
    );
    qemu.check(
        "split UTF-8 and wide deletion",
        "echo 한\x7fa\n".as_bytes(),
        &[b"\x1b[2D\x1b[0K", b"\r\na\r\n"],
        1,
        1,
    );
    qemu.check(
        "combining character deletion",
        "echo e\u{301}\x7fx\n".as_bytes(),
        &[b"\r\nx\r\n"],
        1,
        64,
    );
    qemu.check(
        "clear line",
        b"echo bad\x15echo good\n",
        &[b"\r\ngood\r\n"],
        1,
        64,
    );
    qemu.check(
        "cancel",
        b"exit\x03echo alive\n",
        &[b"^C\r\n", b"\r\nalive\r\n"],
        2,
        64,
    );
    qemu.check(
        "buffered multiple commands",
        b"echo first\necho second\n",
        &[b"\r\nfirst\r\n", b"\r\nsecond\r\n"],
        2,
        64,
    );
    qemu.check(
        "ignored arrow sequence",
        b"echo a\x1b[Ab\n",
        &[b"\r\nab\r\n"],
        1,
        64,
    );
    qemu.check(
        "unknown command",
        b"missing\n",
        &[b"unknown command: missing"],
        1,
        64,
    );
    qemu.check(
        "quote error",
        b"exit '\n",
        &[b"error: unclosed ' quote"],
        1,
        64,
    );
    qemu.check(
        "invalid UTF-8",
        b"exit\xff\n",
        &[b"error: invalid UTF-8 input"],
        1,
        64,
    );
    qemu.check(
        "incomplete UTF-8 before Enter",
        b"\xe3\n",
        &[b"error: invalid UTF-8 input"],
        1,
        64,
    );

    let pasted = "abc한글🦀 ".repeat(4096);
    let command = format!("echo '{pasted}'\r\n");
    let expected = format!("\r\n{pasted}\r\n");
    qemu.check(
        "large Unicode paste",
        command.as_bytes(),
        &[expected.as_bytes()],
        1,
        64,
    );
    qemu.check(
        "recovery",
        b"echo recovered\n",
        &[b"\r\nrecovered\r\n"],
        1,
        64,
    );
    qemu.check("clear screen", b"clear\n", &[b"\x1b[2J\x1b[H"], 1, 64);
    qemu.exit_shell();
}
