use tyos_mem::{
    addr::{PAddr, VAddr},
    page_table::{PageTable, PageTableEntry},
};

#[unsafe(no_mangle)]
#[unsafe(link_section = ".page_table")]
pub static mut PAGE_TABLE: PageTable = PageTable::EMPTY;

/// 1 GiB in Sv39
const L2_PTE_SIZE: usize = 1024 * 1024 * 1024;

#[unsafe(no_mangle)]
pub extern "C" fn build_boot_page_table(page_table_ptr: *mut PageTable) {
    unsafe {
        let page_table = &mut *page_table_ptr;

        // 1. clear
        core::ptr::write_bytes(page_table.entries.as_mut_ptr(), 0, 512);

        // LAYOUT:
        // 0 ~ 255: user space
        // 256 ~ 509: hhdm
        // 510 ~ 511: kernel space

        // 2. hhdm
        for (index, entry_index) in (256..510).enumerate() {
            page_table.entries[entry_index] = PageTableEntry::new(
                PAddr::from_raw_addr(L2_PTE_SIZE * index),
                PageTableEntry::FLAG_VALID
                    | PageTableEntry::FLAG_READ
                    | PageTableEntry::FLAG_WRITE
                    | PageTableEntry::FLAG_GLOBAL
                    | PageTableEntry::FLAG_ACCESSED
                    | PageTableEntry::FLAG_DIRTY,
            );
        }

        unsafe extern "C" {
            static _kernel_start: u8;
            static _kernel_end: u8;
        }

        let kernel_paddr_start = PAddr::from_ptr(&raw const _kernel_start).align_down(L2_PTE_SIZE);
        let kernel_paddr_end = PAddr::from_ptr(&raw const _kernel_end).align_up(L2_PTE_SIZE);

        // 3. kernel space
        for (index, entry_index) in (510..512).enumerate() {
            page_table.entries[entry_index] = PageTableEntry::new(
                kernel_paddr_start + L2_PTE_SIZE * index,
                PageTableEntry::FLAG_VALID
                    | PageTableEntry::FLAG_READ
                    | PageTableEntry::FLAG_WRITE
                    | PageTableEntry::FLAG_EXECUTE
                    | PageTableEntry::FLAG_GLOBAL
                    | PageTableEntry::FLAG_ACCESSED
                    | PageTableEntry::FLAG_DIRTY,
            );
        }

        let kernel_index_start = kernel_paddr_start.addr / L2_PTE_SIZE;
        let kernel_index_end = kernel_paddr_end.addr / L2_PTE_SIZE;

        // 4. temporal double mapping to kernel space before trampoline (identity mapping)
        for (index, entry_index) in (kernel_index_start..kernel_index_end).enumerate() {
            page_table.entries[entry_index] = PageTableEntry::new(
                kernel_paddr_start + L2_PTE_SIZE * index,
                PageTableEntry::FLAG_VALID
                    | PageTableEntry::FLAG_READ
                    | PageTableEntry::FLAG_WRITE
                    | PageTableEntry::FLAG_EXECUTE
                    | PageTableEntry::FLAG_GLOBAL
                    | PageTableEntry::FLAG_ACCESSED
                    | PageTableEntry::FLAG_DIRTY,
            );
        }
    }
}

pub fn build_task_page_table(page_table: &mut PageTable) {
    unsafe {
        // zero out the page table
        core::ptr::write_bytes::<PageTableEntry>(
            page_table.entries.as_mut_ptr(),
            0,
            page_table.entries.len(),
        );

        // copy the shared page table to the task page table (hhdm + kernel space)
        core::ptr::copy_nonoverlapping::<PageTableEntry>(
            PAGE_TABLE.entries[256..].as_ptr(),
            page_table.entries[256..].as_mut_ptr(),
            256,
        );
    }
}

pub fn remove_temporal_double_mapping() {
    let page_table_vaddr = VAddr::from_ptr(&raw const PAGE_TABLE);

    unsafe {
        let page_table_ptr = page_table_vaddr.to_mut_ptr::<PageTable>();
        let page_table = &mut *page_table_ptr;

        unsafe extern "C" {
            static _kernel_start: u8;
            static _kernel_end: u8;
        }

        let kernel_paddr_start = VAddr::from_ptr(&raw const _kernel_start)
            .from_kernel_to_physical()
            .align_down(L2_PTE_SIZE);
        let kernel_paddr_end = VAddr::from_ptr(&raw const _kernel_end)
            .from_kernel_to_physical()
            .align_up(L2_PTE_SIZE);

        let kernel_index_start = kernel_paddr_start.addr / L2_PTE_SIZE;
        let kernel_index_end = kernel_paddr_end.addr / L2_PTE_SIZE;
        let kernel_range = kernel_index_start..kernel_index_end;

        core::ptr::write_bytes::<PageTableEntry>(
            page_table.entries[kernel_range.clone()].as_mut_ptr(),
            0,
            kernel_range.len(),
        );
    }
}
