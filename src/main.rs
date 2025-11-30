#![no_std]
#![no_main]

mod hal;
mod halt;
mod heap;
mod interrupt;
mod pid_allocator;
mod scheduler;
mod task;
mod timer_interrupt_handler;
mod trap;

extern crate alloc;

use crate::{
    hal::{get_frame_mgr, get_timer_frequency, init_hal},
    halt::halt,
    heap::init_heap,
    interrupt::enable_interrupts,
    scheduler::{schedule, spawn_idle_task, spawn_task},
    trap::trap_all,
};
use tyos_mem::page_table::{PageTable, PageTableEntry};
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
    core::arch::naked_asm!("la sp, _stack_top", "jal kmain", "j .halt", ".halt:", "wfi");
}

#[unsafe(no_mangle)]
pub extern "C" fn kmain(hart_id: usize, dtb_pa: usize) -> ! {
    // 16 MiB
    const KERNEL_HEAP_SIZE: usize = 16 * 1024 * 1024;

    if hart_id != 0 {
        halt();
    }

    zero_bss();

    init_trap();
    init_hal(KERNEL_HEAP_SIZE, dtb_pa);

    init_vm();
    init_heap_alloc(KERNEL_HEAP_SIZE);

    {
        let frame_mgr = get_frame_mgr();

        println!(
            "[MEM] total: {} KiB, free: {} KiB",
            frame_mgr.total_size() / 1024,
            frame_mgr.free_size() / 1024
        );
        println!("[TIMER] frequency: {} Hz", get_timer_frequency());
    }

    println!("kernel initialized");

    spawn_idle_task();

    {
        fn sleep(time_in_ms: usize) {
            let timer_frequency = get_timer_frequency();
            let target_ticks = get_current_ticks() + (timer_frequency * time_in_ms / 1000);

            while get_current_ticks() < target_ticks {
                schedule();
            }
        }

        fn task_a() {
            loop {
                println!("Hello from Task A!");
                sleep(1000);
            }
        }

        fn task_b() {
            loop {
                println!("Hello from Task B!");
                sleep(1000);
            }
        }

        spawn_task(task_a);
        spawn_task(task_b);
    }

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

fn init_vm() {
    unsafe {
        let l2 = build_page_table();

        l2.entries[0] = PageTableEntry::new(
            0x0000_0000,
            PageTableEntry::FLAG_VALID
                | PageTableEntry::FLAG_READ
                | PageTableEntry::FLAG_WRITE
                | PageTableEntry::FLAG_GLOBAL
                | PageTableEntry::FLAG_ACCESSED
                | PageTableEntry::FLAG_DIRTY,
        );
        l2.entries[2] = PageTableEntry::new(
            0x8000_0000,
            PageTableEntry::FLAG_VALID
                | PageTableEntry::FLAG_READ
                | PageTableEntry::FLAG_WRITE
                | PageTableEntry::FLAG_EXECUTE
                | PageTableEntry::FLAG_GLOBAL
                | PageTableEntry::FLAG_ACCESSED
                | PageTableEntry::FLAG_DIRTY,
        );

        let root_ppn = (l2 as *const _ as usize) >> 12;
        let satp_val = (8 << 60) | root_ppn;

        core::arch::asm!("csrw satp, {}", in(reg) satp_val);
        core::arch::asm!("sfence.vma");
    }
}

fn build_page_table() -> &'static mut PageTable {
    let page_table_addr = get_frame_mgr().alloc().unwrap();

    unsafe {
        core::ptr::write_bytes(
            page_table_addr.to_mut_ptr::<u8>(),
            0,
            size_of::<PageTable>(),
        );

        &mut *(page_table_addr.to_mut_ptr())
    }
}

fn init_heap_alloc(heap_size: usize) {
    unsafe extern "C" {
        static _kernel_end: u8;
    }

    init_heap(&raw const _kernel_end as usize, heap_size);
}
