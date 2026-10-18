//! Exercise the real shell's run command, including nested interactive shells.
//! Build apps, then kernel, as in qemu_smoke.rs. Run this ignored test with:
//! cargo test --locked -p shell --no-default-features --target aarch64-apple-darwin --test qemu_shell_run -- --ignored --nocapture

#[allow(dead_code)]
mod support;

use support::Qemu;

#[test]
#[ignore = "requires QEMU and freshly built RISC-V apps/kernel"]
fn shell_runs_programs_and_resumes_after_children_exit() {
    let mut qemu = Qemu::start();
    qemu.check(
        "run help",
        b"help run\n",
        &[b"run <program> [arg]", b"unsigned decimal integer"],
        1,
        64,
    );
    qemu.check(
        "invalid run arguments",
        b"run\nrun hello-world-alpha -1\nrun hello-world-alpha 1 2\n",
        &[
            b"usage: run <program> [arg]",
            b"run: arg must be an unsigned decimal integer",
        ],
        3,
        64,
    );
    qemu.check(
        "unknown program",
        b"run missing\n",
        &[b"run: cannot spawn 'missing' (-1)"],
        1,
        64,
    );
    qemu.check(
        "run alpha and wait",
        b"run hello-world-alpha\n",
        &[b"alpha: 0", b"alpha: done", b"process 2 status: 0\r\n"],
        1,
        64,
    );
    qemu.check(
        "run beta with scalar argument",
        b"run hello-world-beta 42\n",
        &[b"beta: 0", b"beta: done", b"process 3 status: 0\r\n"],
        1,
        64,
    );
    qemu.check("start nested shell", b"run shell\n", &[], 1, 64);
    qemu.check(
        "child shell input and exit status",
        b"echo child\nexit 7\n",
        &[b"\r\nchild\r\n", b"bye\r\n", b"process 4 status: 7\r\n"],
        2,
        64,
    );
    qemu.check(
        "paste across parent and child shell boundaries",
        b"run shell\necho pasted-child\nexit -7\necho parent-again\n",
        &[
            b"\r\npasted-child\r\n",
            b"process 5 status: -7\r\n",
            b"\r\nparent-again\r\n",
        ],
        4,
        64,
    );
    qemu.exit_shell();
}
