"""Exercise the built shell through QEMU's real UART/syscall path.

From the repository root:
    cargo build --locked -p shell -p hello-world-alpha -p hello-world-beta
    cargo build --locked -p tyos
    python3 apps/shell/tests/qemu_smoke.py

The kernel must start the shell. No kernel sources are changed by this test.
"""

import os
from pathlib import Path
import select
import signal
import subprocess
import tempfile
import time


ROOT = Path(__file__).resolve().parents[3]
PROMPT = b"(shell)> "


def main():
    log_dir = Path(tempfile.mkdtemp(prefix="tyos-shell-smoke-"))
    output = bytearray()
    process = subprocess.Popen(
        [
            "qemu-system-riscv64",
            "-machine", "virt",
            "-nographic",
            "-monitor", "none",
            "-bios", "default",
            "-kernel", str(ROOT / "target/riscv64gc-unknown-none-elf/debug/tyos"),
        ],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        start_new_session=True,
    )

    def pump(timeout):
        if select.select([process.stdout], [], [], timeout)[0]:
            data = os.read(process.stdout.fileno(), 65536)
            if not data:
                raise AssertionError("QEMU exited before the test completed")
            output.extend(data)

    def until(predicate):
        deadline = time.monotonic() + 10
        while not predicate():
            if time.monotonic() >= deadline:
                raise AssertionError("timed out waiting for shell output")
            pump(0.05)

    def send(payload, chunk=64):
        for offset in range(0, len(payload), chunk):
            process.stdin.write(payload[offset:offset + chunk])
            process.stdin.flush()
            # Pace the emulated UART and drain output while sending long lines.
            pump(0.002)
            time.sleep(0.002)

    def check(label, payload, expected, prompts=1, chunk=64):
        start = len(output)
        send(payload, chunk)
        until(lambda: output[start:].count(PROMPT) >= prompts)
        actual = bytes(output[start:])
        for marker in expected:
            assert marker in actual, (label, f"missing {len(marker)}-byte output", actual[-1000:])
        assert actual.count(PROMPT) == prompts, (label, "extra prompt")
        print(f"PASS {label}")

    try:
        until(lambda: PROMPT in output)
        check("help", b"help\n", [b"help [command]", b"Ctrl-U", b"Ctrl-C"])
        check("quoted Unicode arguments", 'echo "hello world" \'한 글\'\r\n'.encode(), ["\r\nhello world 한 글\r\n".encode()])
        check("both backspace encodings", b"\x08echo abc\x08X\x7fY\n", [b"\r\nabY\r\n", b"\x1b[1D\x1b[0K"])
        check("split UTF-8 and wide deletion", "echo 한\x7fa\n".encode(), [b"\x1b[2D\x1b[0K", b"\r\na\r\n"], chunk=1)
        check("combining character deletion", "echo e\u0301\x7fx\n".encode(), [b"\r\nx\r\n"])
        check("clear line", b"echo bad\x15echo good\n", [b"\r\ngood\r\n"])
        check("cancel", b"exit\x03echo alive\n", [b"^C\r\n", b"\r\nalive\r\n"], prompts=2)
        check("buffered multiple commands", b"echo first\necho second\n", [b"\r\nfirst\r\n", b"\r\nsecond\r\n"], prompts=2)
        check("ignored arrow sequence", b"echo a\x1b[Ab\n", [b"\r\nab\r\n"])
        check("unknown command", b"missing\n", [b"unknown command: missing"])
        check("quote error", b"exit '\n", [b"error: unclosed ' quote"])
        check("invalid UTF-8", b"exit\xff\n", [b"error: invalid UTF-8 input"])
        check("incomplete UTF-8 before Enter", b"\xe3\n", [b"error: invalid UTF-8 input"])
        pasted = ("abc한글🦀 " * 4096).encode()
        check("large Unicode paste", b"echo '" + pasted + b"'\r\n", [b"\r\n" + pasted + b"\r\n"])
        check("recovery", b"echo recovered\n", [b"\r\nrecovered\r\n"])
        check("clear screen", b"clear\n", [b"\x1b[2J\x1b[H"])
        start = len(output)
        send(b"exit 7\n")
        until(lambda: b"bye\r\n" in output[start:])
        pump(0.2)
        assert PROMPT not in output[start:], "shell continued after exit"
        assert process.poll() is None, "QEMU terminated unexpectedly"
        assert b"PANIC" not in output and b"page fault" not in output, bytes(output[-2000:])
        print("PASS exit without kernel panic")
    finally:
        if process.poll() is None:
            os.killpg(process.pid, signal.SIGTERM)
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
        (log_dir / "serial.log").write_bytes(output)
        print(f"UART log: {log_dir / 'serial.log'}")


if __name__ == "__main__":
    main()
