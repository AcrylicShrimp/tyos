#![no_std]
#![no_main]

use tyos_ulib::{entry_point, println};

#[entry_point]
fn main() -> isize {
    println!("Hello, world! This is from user app!");
    0
}
