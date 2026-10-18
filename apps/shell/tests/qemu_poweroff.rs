//! Build apps, then kernel, as in qemu_smoke.rs. Run:
//! cargo test --locked -p shell --no-default-features --target aarch64-apple-darwin --test qemu_poweroff -- --ignored --nocapture

#[allow(dead_code)]
mod support;

use support::Qemu;

#[test]
#[ignore = "requires QEMU and freshly built RISC-V apps/kernel"]
fn shell_poweroff_terminates_the_machine() {
    let mut qemu = Qemu::start();
    qemu.check(
        "poweroff help and invalid arguments do not shut down",
        b"poweroff --help\npoweroff extra\necho alive\n",
        &[
            b"shut down the whole system",
            b"usage: poweroff",
            b"\r\nalive\r\n",
        ],
        3,
        64,
    );
    qemu.poweroff_shell();
}

#[test]
#[ignore = "requires QEMU and freshly built RISC-V apps/kernel"]
fn nested_shell_poweroff_terminates_the_whole_machine() {
    let mut qemu = Qemu::start();
    qemu.check("start child shell", b"run shell\n", &[], 1, 64);
    qemu.poweroff_shell();
}
