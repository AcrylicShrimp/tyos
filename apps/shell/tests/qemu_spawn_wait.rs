//! Run real spawn/wait syscalls without changing application or kernel sources.
//!
//! Build apps plus `-p tyos-ulib`, then the kernel, as in qemu_smoke.rs. Run:
//! cargo test --locked -p shell --no-default-features --target aarch64-apple-darwin --test qemu_spawn_wait -- --ignored --nocapture
//!
//! A temporary COPY of the kernel has its embedded shell ELF replaced with a
//! smaller test fixture. The original kernel, app binaries, and source stay intact.

#[allow(dead_code)]
mod support;

use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use support::Qemu;

#[test]
#[ignore = "requires QEMU and freshly built RISC-V apps, tyos-ulib, and kernel"]
fn spawn_wait_lifecycle() {
    let app = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = app.join("../..");
    let built = root.join("target/riscv64gc-unknown-none-elf/debug");
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp = std::env::temp_dir().join(format!("tyos-spawn-wait-{}-{stamp}", std::process::id()));
    fs::create_dir(&temp).expect("create test artifact directory");
    let guest = temp.join("guest.elf");
    let result = Command::new("rustc")
        .arg(app.join("tests/fixtures/spawn_wait_guest.rs"))
        .args([
            "--edition=2024",
            "--crate-name=spawn_wait_guest",
            "--target=riscv64gc-unknown-none-elf",
        ])
        .arg("--extern")
        .arg(format!(
            "tyos_ulib={}",
            built.join("libtyos_ulib.rlib").display()
        ))
        .arg("-L")
        .arg(format!("dependency={}", built.join("deps").display()))
        .arg("-L")
        .arg(format!(
            "dependency={}",
            root.join("target/debug/deps").display()
        ))
        .arg("-C")
        .arg(format!(
            "link-arg=-T{}",
            app.join("linker.user.ld").display()
        ))
        .args(["-C", "panic=abort"])
        .arg("-o")
        .arg(&guest)
        .output()
        .expect("compile RISC-V test fixture");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );

    let fixture = fs::read(&guest).unwrap();
    let shell = fs::read(built.join("shell")).expect("build the shell first");
    let mut kernel = fs::read(built.join("tyos")).expect("build the kernel after the apps");
    assert!(
        fixture.len() <= shell.len(),
        "fixture exceeds embedded shell size"
    );
    let offsets: Vec<_> = kernel
        .windows(shell.len())
        .enumerate()
        .filter_map(|(offset, bytes)| (bytes == shell).then_some(offset))
        .collect();
    assert_eq!(
        offsets.len(),
        1,
        "expected exactly one embedded shell; rebuild kernel after apps"
    );
    let offset = offsets[0];
    kernel[offset..offset + shell.len()].fill(0);
    kernel[offset..offset + fixture.len()].copy_from_slice(&fixture);
    let test_kernel = temp.join("tyos-test.elf");
    fs::write(&test_kernel, kernel).unwrap();
    eprintln!("Test kernel copy: {}", test_kernel.display());

    let mut qemu = Qemu::start_with_kernel(&test_kernel, b"CHILD_READY 42");
    qemu.expect_output("parent entering wait", b"PARENT_WAIT");
    qemu.observe_quiet(Duration::from_millis(100), b"PASS blocking wait");
    qemu.release_guest(b'!');
    qemu.expect_output("late-wait parent ready", b"LATE_PARENT_READY");
    qemu.expect_output("late-wait child exiting", b"LATE_CHILD_EXITING");
    qemu.observe_quiet(Duration::from_millis(200), b"PASS wait after child exit");
    qemu.release_guest(b'L');
    qemu.expect_output("shared child ready", b"CHILD_READY 43");
    qemu.expect_output("second waiter entering wait", b"WAITER_WAIT");
    qemu.expect_output("parent entering shared wait", b"MULTI_PARENT_WAIT");
    qemu.observe_quiet(Duration::from_millis(100), b"PASS multiple waiters");
    qemu.release_guest(b'?');
    qemu.expect_output("all spawn/wait assertions", b"SPAWN_WAIT_ALL_PASS");
    qemu.observe_quiet(Duration::from_millis(200), b"THIS_MARKER_MUST_NEVER_APPEAR");
    println!("PASS spawn/wait lifecycle, arguments, exit records, multiple waiters, and 64 cycles");
}
