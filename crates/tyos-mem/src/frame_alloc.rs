use crate::{
    addr::PAddr,
    frame_mgr::{FrameMgr, PhysicalMemoryProbe},
};
use tyos_sync::irq_lock::IrqSpinLock;

#[derive(Debug)]
pub struct FrameAlloc {
    frame_mgr: IrqSpinLock<FrameMgr>,
}

impl Default for FrameAlloc {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameAlloc {
    pub const fn new() -> Self {
        Self {
            frame_mgr: IrqSpinLock::new(FrameMgr::new()),
        }
    }

    pub fn init<T: PhysicalMemoryProbe>(&self, probe: &mut T) {
        self.frame_mgr.lock().init(probe);
    }

    pub fn alloc_contiguous(&self, frame_count: usize) -> Option<PAddr> {
        self.frame_mgr.lock().alloc(frame_count)
    }

    pub fn free_contiguous(&self, frame: PAddr, frame_count: usize) {
        self.frame_mgr.lock().free(frame, frame_count)
    }

    pub fn alloc_many_into(&self, frames: &mut [PAddr]) -> bool {
        frames.fill(PAddr::NULL);

        {
            let mut frame_mgr = self.frame_mgr.lock();
            let mut allocated_frames = 0;

            for frame in frames.iter_mut() {
                match frame_mgr.alloc(1) {
                    Some(paddr) => {
                        *frame = paddr;
                        allocated_frames += 1;
                    }
                    None => {
                        break;
                    }
                }
            }

            if allocated_frames == frames.len() {
                return true;
            }

            for frame in frames[..allocated_frames].iter_mut() {
                frame_mgr.free(*frame, 1);
            }
        }

        frames.fill(PAddr::NULL);

        false
    }

    pub fn free_many(&self, frames: &[PAddr]) {
        let mut frame_mgr = self.frame_mgr.lock();

        for frame in frames.iter() {
            frame_mgr.free(*frame, 1);
        }
    }
}
