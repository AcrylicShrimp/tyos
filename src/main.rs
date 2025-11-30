#![no_std]
#![no_main]

mod hal;
mod heap;
mod trap;

extern crate alloc;

use crate::{
    hal::{get_frame_mgr, get_timer_frequency, init_hal},
    heap::init_heap,
    trap::trap_all,
};
use tyos_mem::{
    frame_mgr::FrameMgr,
    page_table::{PageTable, PageTableEntry},
};
use tyos_sbi::set_timer::{get_current_ticks, set_timer};
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
    if hart_id != 0 {
        halt();
    }

    zero_bss();

    init_trap();
    init_hal(dtb_pa);

    init_vm();
    init_heap_alloc(1024 * 1024);

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

    enable_interrupts();
    set_timer(get_current_ticks() + get_timer_frequency());

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

fn halt() -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
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
    let heap_count: usize = heap_size / FrameMgr::FRAME_SIZE;
    let heap_addr = get_frame_mgr().alloc().unwrap();

    for _ in 1..heap_count {
        get_frame_mgr().alloc().unwrap();
    }

    init_heap(heap_addr.addr, heap_size);
}

fn enable_interrupts() {
    /// Software Interrupt Enable
    const SIE_SSIE: usize = 1 << 1;
    /// Timer Interrupt Enable
    const SIE_STIE: usize = 1 << 5;
    /// External Interrupt Enable
    const SIE_SEIE: usize = 1 << 9;

    /// Global Interrupt Enable
    const SSTATUS_SIE: usize = 1 << 1;

    unsafe {
        core::arch::asm!("csrs sie, {}", in(reg) SIE_SSIE | SIE_STIE | SIE_SEIE);
        core::arch::asm!("csrs sstatus, {}", in(reg) SSTATUS_SIE);
    }
}
