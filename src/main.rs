#![no_std]
#![no_main]

mod hal;
mod heap;

extern crate alloc;

use crate::{hal::Hal, heap::init_heap};
use tyos_mem::{
    frame_mgr::FrameMgr,
    page_table::{PageTable, PageTableEntry},
};
use tyos_uart::println;

static mut FRAME_ALLOCATOR: FrameMgr = FrameMgr::EMPTY;

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

    let hal = Hal::parse_from_dtb(dtb_pa);

    unsafe {
        FRAME_ALLOCATOR = hal.frame_mgr;
    }

    init_vm();
    init_heap_alloc(1024 * 1024);

    #[allow(static_mut_refs)]
    unsafe {
        println!(
            "[MEM] total: {} KiB, free: {} KiB",
            FRAME_ALLOCATOR.total_size() / 1024,
            FRAME_ALLOCATOR.free_size() / 1024
        );
    }

    println!("kernel initialized");

    halt();
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
    #[allow(static_mut_refs)]
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
    #[allow(static_mut_refs)]
    unsafe {
        let page_table_addr = FRAME_ALLOCATOR.alloc().unwrap();

        core::ptr::write_bytes(
            page_table_addr.to_mut_ptr::<u8>(),
            0,
            size_of::<PageTable>(),
        );

        &mut *(page_table_addr.to_mut_ptr())
    }
}

fn init_heap_alloc(heap_size: usize) {
    #[allow(static_mut_refs)]
    unsafe {
        let heap_count: usize = heap_size / FrameMgr::FRAME_SIZE;
        let heap_addr = FRAME_ALLOCATOR.alloc().unwrap();

        for _ in 1..heap_count {
            FRAME_ALLOCATOR.alloc().unwrap();
        }

        init_heap(heap_addr.addr, heap_size);
    }
}
