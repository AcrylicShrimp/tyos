#![no_std]

mod heap;
mod stdout;

pub use stdout::*;
pub use tyos_syscall::*;
pub use tyos_ulib_macro::*;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("USER PANIC: {info}");
    tyos_syscall::exit(-1);
}
