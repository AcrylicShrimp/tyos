#![no_std]
#![no_main]

mod hal;
mod halt;
mod heap;
mod interrupt;
mod pid_allocator;
mod scheduler;
mod syscall;
mod task;
mod timer_interrupt_handler;
mod trap;
mod vmem;

extern crate alloc;

use crate::{
    hal::{get_timer_frequency, init_hal},
    halt::halt,
    heap::init_heap,
    interrupt::enable_interrupts,
    scheduler::{schedule, spawn_idle_task, spawn_task},
    trap::trap_all,
    vmem::remove_temporal_double_mapping,
};
use tyos_mem::addr::PAddr;
use tyos_sbi::set_timer::get_current_ticks;
use tyos_uart::println;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("--------------------------------");
    println!("        TYOS PANIC ERROR        ");
    println!("--------------------------------");

    match info.location() {
        Some(location) => {
            println!(
                "[PANIC] location: {}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            );
        }
        None => {
            println!("[PANIC] location: unknown");
        }
    }

    println!("[PANIC] cause:");
    println!("{}", info.message());
    println!("--------------------------------");

    halt();
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.init")]
pub extern "C" fn _start() -> ! {
    const KERNEL_BASE_OFFSET: usize = 0xFFFF_FFFF_0000_0000;

    core::arch::naked_asm!(
        // set stack pointer
        "la sp, _stack_top",
        "",
        // back-up hart id and dtb physical address
        "mv s0, a0",
        "mv s1, a1",
        "",
        // build boot page table
        "la a0, PAGE_TABLE",
        "call build_boot_page_table",
        "",
        // addr -> ppn
        "la t0, PAGE_TABLE",
        "srli t0, t0, 12",
        "",
        // mode 8 (Sv39)
        "li t1, 8 << 60",
        "or t0, t0, t1",
        "",
        // engage virtual memory
        "csrw satp, t0",
        "sfence.vma",
        "",
        // jump to kentry, with virtual address
        "la ra, kentry",
        "li t0, {offset}",
        "add ra, ra, t0",
        "ret",
        offset = const KERNEL_BASE_OFFSET,
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn kentry() {
    // make stack pointer point to the end of the stack, in virtual address
    unsafe {
        core::arch::asm!("la sp, _stack_top");
    }

    // restore hart id and dtb physical address from back-up
    let hart_id: usize;
    let dtb_paddr: usize;

    unsafe {
        core::arch::asm!("mv {}, s0", out(reg) hart_id);
        core::arch::asm!("mv {}, s1", out(reg) dtb_paddr);
    }

    // remove temporal double mapping
    remove_temporal_double_mapping();

    unsafe {
        core::arch::asm!("sfence.vma");
    }

    kmain(hart_id, dtb_paddr);
}

#[unsafe(no_mangle)]
pub extern "C" fn kmain(hart_id: usize, dtb_paddr: usize) -> ! {
    if hart_id != 0 {
        halt();
    }

    zero_bss();

    /// 16 MiB
    const KERNEL_HEAP_SIZE: usize = 16 * 1024 * 1024;

    init_trap();
    init_hal(KERNEL_HEAP_SIZE, PAddr::from_raw_addr(dtb_paddr));
    init_heap_alloc(KERNEL_HEAP_SIZE);

    println!("kernel initialized");

    spawn_idle_task();

    // {
    //     fn sleep(time_in_ms: usize) {
    //         let timer_frequency = get_timer_frequency();
    //         let target_ticks = get_current_ticks() + (timer_frequency * time_in_ms / 1000);

    //         while get_current_ticks() < target_ticks {
    //             schedule();
    //         }
    //     }

    //     fn task_a() {
    //         loop {
    //             println!("Hello from Task A!");
    //             sleep(1000);
    //         }
    //     }

    //     fn task_b() {
    //         loop {
    //             println!("Hello from Task B!");
    //             sleep(1000);
    //         }
    //     }

    //     spawn_task(task_a);
    //     spawn_task(task_b);
    // }

    enable_interrupts();
    halt();
}

fn init_trap() {
    unsafe {
        core::arch::asm!("csrw stvec, {}", in(reg) trap_all);
    }
}

fn zero_bss() {
    unsafe extern "C" {
        static mut _bss_start: u8;
        static mut _bss_end: u8;
    }

    unsafe {
        let start = &raw mut _bss_start;
        let end = &raw mut _bss_end;
        let count = end as usize - start as usize;
        core::ptr::write_bytes(start, 0, count);
    }
}

fn init_heap_alloc(heap_size: usize) {
    unsafe extern "C" {
        static _kernel_end: u8;
    }

    init_heap(&raw const _kernel_end as usize, heap_size);
}
