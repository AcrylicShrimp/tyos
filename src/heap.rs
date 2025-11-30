use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init_heap(heap_addr: usize, heap_size: usize) {
    unsafe {
        ALLOCATOR.lock().init(heap_addr as *mut u8, heap_size);
    }
}
