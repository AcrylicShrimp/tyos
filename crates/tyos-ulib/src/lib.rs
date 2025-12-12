#![no_std]

mod stdout;

pub use stdout::*;
pub use tyos_syscall;

unsafe extern "Rust" {
    fn main() -> isize;
}

core::arch::global_asm!(
    ".section .text.entry",
    ".global _start",
    "",
    "_start:",
    "call {main}",
    "call {exit}",
    "unimp",
    main = sym main,
    exit = sym tyos_syscall::exit,
);

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("PANIC: {info}");
    tyos_syscall::exit(-1);
}
