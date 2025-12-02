use crate::addr::PAddr;

pub struct FrameMgr {
    next_free_frame_addr: Option<PAddr>,
    next_region_addr: Option<PAddr>,
    total_size: usize,
    free_size: usize,
}

impl FrameMgr {
    pub const EMPTY: Self = Self {
        next_free_frame_addr: None,
        next_region_addr: None,
        total_size: 0,
        free_size: 0,
    };

    /// 4 KiB
    pub const FRAME_SIZE: usize = 4 * 1024;

    pub fn total_size(&self) -> usize {
        self.total_size
    }

    pub fn free_size(&self) -> usize {
        self.free_size
    }

    pub fn add_region(&mut self, region_start: PAddr, region_size: usize) {
        let region_start = align_up(region_start, Self::FRAME_SIZE);
        let region_end = align_down(region_start + region_size, Self::FRAME_SIZE);

        if region_end.addr < region_start.addr {
            // ignore invalid region
            return;
        }

        if region_end.addr - region_start.addr < Self::FRAME_SIZE {
            // ignore too small region
            return;
        }

        let frame_ptr = region_start.to_hhdm().to_mut_ptr();

        unsafe {
            core::ptr::write_volatile(
                frame_ptr,
                [region_end, self.next_region_addr.unwrap_or(PAddr::NULL)],
            );
        }

        self.next_region_addr = Some(region_start);
        self.total_size += region_size;
        self.free_size += region_size;
    }

    pub fn alloc(&mut self) -> Option<PAddr> {
        if let Some(frame) = self.next_free_frame_addr {
            let frame_ptr = frame.to_hhdm().to_mut_ptr();
            let next_frame_addr: PAddr = unsafe { core::ptr::read_volatile(frame_ptr) };

            self.next_free_frame_addr = match next_frame_addr {
                PAddr::NULL => None,
                _ => Some(next_frame_addr),
            };

            self.free_size -= Self::FRAME_SIZE;
            return Some(frame);
        }

        if let Some(mut region_start) = self.next_region_addr {
            let [region_end, next_region_addr] = unsafe {
                core::ptr::read_volatile(region_start.to_hhdm().to_mut_ptr::<[PAddr; 2]>())
            };

            let frame = region_start;
            region_start += Self::FRAME_SIZE;

            if region_start.addr == region_end.addr {
                self.next_region_addr = match next_region_addr {
                    PAddr::NULL => None,
                    _ => Some(next_region_addr),
                };
            } else {
                unsafe {
                    core::ptr::write_volatile(
                        region_start.to_hhdm().to_mut_ptr(),
                        [region_end, next_region_addr],
                    );
                }

                self.next_region_addr = Some(region_start);
            }

            self.free_size -= Self::FRAME_SIZE;
            return Some(frame);
        }

        None
    }

    pub fn free(&mut self, frame: PAddr) {
        let frame_ptr = frame.to_hhdm().to_mut_ptr();

        unsafe {
            core::ptr::write_volatile(frame_ptr, self.next_free_frame_addr.unwrap_or(PAddr::NULL));
        }

        self.next_free_frame_addr = Some(frame);
        self.free_size += Self::FRAME_SIZE;
    }
}

fn align_up(value: PAddr, alignment: usize) -> PAddr {
    PAddr::from_raw_addr((value.addr + alignment - 1) & !(alignment - 1))
}

fn align_down(value: PAddr, alignment: usize) -> PAddr {
    PAddr::from_raw_addr(value.addr & !(alignment - 1))
}
