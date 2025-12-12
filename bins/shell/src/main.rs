#![no_std]
#![no_main]

use tyos_ulib::println;

#[unsafe(no_mangle)]
fn main() -> isize {
    println!("Hello, world! This is from user app!");
    0
}
