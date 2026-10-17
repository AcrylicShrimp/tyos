use tyos_mem::{frame_alloc::FrameAlloc, frame_mgr::PhysicalMemoryProbe};

static FRAME_ALLOC: FrameAlloc = FrameAlloc::new();

#[inline]
pub fn init_memory<T: PhysicalMemoryProbe>(probe: &mut T) {
    FRAME_ALLOC.init(probe);
}

#[inline]
pub const fn frame_alloc() -> &'static FrameAlloc {
    &FRAME_ALLOC
}
