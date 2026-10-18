use core::alloc::{GlobalAlloc, Layout};
use linked_list_allocator::LockedHeap;
use tyos_mem::frame_mgr::FrameMgr;

struct UserAllocator(LockedHeap);

#[global_allocator]
static ALLOCATOR: UserAllocator = UserAllocator(LockedHeap::empty());

unsafe impl GlobalAlloc for UserAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut locked_heap = self.0.lock();

        if let Ok(allocated) = locked_heap.allocate_first_fit(layout) {
            return allocated.as_ptr();
        }

        let page_count = layout.size().div_ceil(FrameMgr::FRAME_SIZE).max(2);
        let old_heap_start = tyos_syscall::grow_heap(page_count);

        if old_heap_start == 0 {
            return core::ptr::null_mut();
        }

        let grew_heap_size = page_count * FrameMgr::FRAME_SIZE;

        if locked_heap.bottom().is_null() {
            unsafe {
                locked_heap.init(old_heap_start as *mut u8, grew_heap_size);
            }
        } else {
            unsafe {
                locked_heap.extend(grew_heap_size);
            }
        }

        if let Ok(allocated) = locked_heap.allocate_first_fit(layout) {
            return allocated.as_ptr();
        }

        core::ptr::null_mut()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { self.0.dealloc(ptr, layout) }
    }
}
