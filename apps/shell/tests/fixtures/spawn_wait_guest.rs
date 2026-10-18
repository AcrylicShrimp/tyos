//! Bare-metal test fixture, compiled by qemu_spawn_wait.rs rather than Cargo.
#![no_std]
#![no_main]

use tyos_ulib::{Stdin, entry_point, println, spawn, wait};

const WAITER: usize = 1 << 32;

fn launch(name: &str, arg: usize) -> usize {
    let pid = spawn(name.as_ptr(), name.len(), arg);
    assert!(pid > 0, "spawn failed: {pid}");
    pid as usize
}

fn gate(expected: u8) {
    let mut byte = [0];
    loop {
        match Stdin.read(&mut byte) {
            Ok(0) => {}
            Ok(1) => {
                assert_eq!(byte[0], expected);
                return;
            }
            other => panic!("unexpected stdin result: {other:?}"),
        }
    }
}

#[entry_point]
fn main(arg: usize) -> isize {
    if arg & WAITER != 0 {
        let pid = arg & !WAITER;
        println!("WAITER_WAIT");
        assert_eq!(wait(pid), 43);
        println!("PASS second waiter resumed");
        return 17;
    }
    match arg {
        42 | 43 => {
            println!("CHILD_READY {arg}");
            gate(if arg == 42 { b'!' } else { b'?' });
            return arg as isize;
        }
        7 => return -7,
        9 => {
            println!("LATE_CHILD_EXITING");
            return 9;
        }
        100.. => return (arg - 100) as isize,
        0 => {}
        _ => panic!("unexpected child argument: {arg}"),
    }

    let missing = "not-an-application";
    assert_eq!(spawn(missing.as_ptr(), missing.len(), 0), -1);
    assert_eq!(wait(0), -1);
    assert_eq!(wait(usize::MAX), -1);
    // The kernel starts this fixture as its sole user task (PID 1).
    assert_eq!(wait(1), -1);
    println!("PASS invalid spawn and wait targets");

    let first = launch("shell", 42);
    let stack_data = core::hint::black_box([0x1234_5678usize; 32]);
    println!("PARENT_WAIT");
    assert_eq!(wait(first), 42);
    assert!(stack_data.iter().all(|&value| value == 0x1234_5678));
    println!("PASS blocking wait, scalar argument, and stack restoration");
    assert_eq!(wait(first), 42);
    println!("PASS repeated wait preserves exit record");

    let negative = launch("shell", 7);
    assert_eq!(wait(negative), -7);
    println!("PASS negative exit code");

    let late = launch("shell", 9);
    println!("LATE_PARENT_READY");
    gate(b'L');
    assert_eq!(wait(late), 9);
    println!("PASS wait after child exit");

    let shared = launch("shell", 43);
    let waiter = launch("shell", WAITER | shared);
    println!("MULTI_PARENT_WAIT");
    assert_eq!(wait(shared), 43);
    assert_eq!(wait(waiter), 17);
    println!("PASS multiple waiters");

    let alpha = launch("hello-world-alpha", 0);
    assert_eq!(wait(alpha), 0);
    println!("PASS zero-argument application entry");

    let mut previous = alpha;
    for index in 0..64 {
        let pid = launch("shell", 100 + index);
        assert!(pid > previous, "PID was reused");
        assert_eq!(wait(pid), index as isize);
        assert_eq!(wait(first), 42);
        previous = pid;
    }
    println!("PASS 64 spawn/wait cycles without PID reuse");
    println!("SPAWN_WAIT_ALL_PASS");
    0
}
