#![no_std]
#![no_main]

use tyos_ulib::{entry_point, println};

#[entry_point]
fn main(arg0: usize) -> isize {
    for count in 0..10 {
        println!("beta: {}", count);

        for work in 0usize..1_000_000 {
            core::hint::black_box(work);
        }
    }

    println!("beta: done");
    0
}
