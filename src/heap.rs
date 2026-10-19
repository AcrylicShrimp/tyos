use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init_heap(heap_addr: usize, heap_size: usize) {
    unsafe {
        ALLOCATOR.lock().init(heap_addr as *mut u8, heap_size);
    }
}

pub fn heap_size() -> usize {
    ALLOCATOR.lock().size()
}

pub fn heap_used() -> usize {
    ALLOCATOR.lock().used()
}

pub fn heap_free() -> usize {
    ALLOCATOR.lock().free()
}
