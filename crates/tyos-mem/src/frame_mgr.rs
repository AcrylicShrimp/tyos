use crate::addr::PAddr;
use tyos_uart::println;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemBlock {
    next: PAddr,
    prev: PAddr,
}

#[derive(Debug)]
pub struct MemList {
    next: PAddr,
}

impl MemList {
    pub const EMPTY: Self = Self { next: PAddr::NULL };

    pub fn new(next: PAddr) -> Self {
        Self { next }
    }

    #[inline]
    unsafe fn block_mut<'a>(addr: PAddr) -> &'a mut MemBlock {
        unsafe { &mut *addr.to_hhdm().to_mut_ptr::<MemBlock>() }
    }

    pub fn push(&mut self, frame: PAddr) {
        unsafe {
            if self.next != PAddr::NULL {
                let block = Self::block_mut(self.next);
                block.prev = frame;
            }

            core::ptr::write(
                frame.to_hhdm().to_mut_ptr(),
                MemBlock {
                    next: self.next,
                    prev: PAddr::NULL,
                },
            );
        }

        self.next = frame;
    }

    pub fn pop(&mut self) -> Option<PAddr> {
        if self.next == PAddr::NULL {
            return None;
        }

        let frame = self.next;

        unsafe {
            let block = Self::block_mut(frame);
            self.next = block.next;

            if self.next != PAddr::NULL {
                let block = Self::block_mut(self.next);
                block.prev = PAddr::NULL;
            }
        }

        Some(frame)
    }

    pub fn detach(&mut self, addr: PAddr) {
        unsafe {
            let block = Self::block_mut(addr);
            let prev = block.prev;
            let next = block.next;

            if prev != PAddr::NULL {
                let prev_block = Self::block_mut(prev);
                prev_block.next = next;
            }

            if next != PAddr::NULL {
                let next_block = Self::block_mut(next);
                next_block.prev = prev;
            }

            if self.next == addr {
                self.next = next;
            }
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemMetadata(u8);

impl MemMetadata {
    pub fn new(order: u8, is_free: bool) -> Self {
        Self((order << 1) | (is_free as u8))
    }

    pub fn order(self) -> u8 {
        self.0 >> 1
    }

    pub fn is_free(self) -> bool {
        self.0 & 1 != 0
    }
}

impl From<u8> for MemMetadata {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<MemMetadata> for u8 {
    fn from(value: MemMetadata) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Region {
    pub start: PAddr,
    pub end: PAddr,
}

impl Region {
    pub fn new(start: PAddr, end: PAddr) -> Self {
        Self { start, end }
    }

    pub fn merge(lhs: Self, rhs: Self) -> Self {
        Self {
            start: lhs.start.min(rhs.start),
            end: lhs.end.max(rhs.end),
        }
    }

    pub fn is_empty(self) -> bool {
        self.end.addr <= self.start.addr
    }

    pub fn is_overlap(self, other: Self) -> bool {
        self.start.addr < other.end.addr && other.start.addr < self.end.addr
    }

    pub fn len(self) -> usize {
        self.end - self.start
    }

    pub fn align(self, alignment: usize) -> Self {
        Self {
            start: self.start.align_up(alignment),
            end: self.end.align_down(alignment),
        }
    }
}

pub trait PhysicalMemoryProbe {
    fn regions(&self) -> &[Region];
    fn set_region(&mut self, index: usize, region: Region);
}

pub trait PhysicalMemoryProbeExt {
    fn regions_iter(&self) -> impl Iterator<Item = Region>;
    fn entire_region(&self) -> Region;
    fn find_free_contiguous_region(&self, size: usize) -> Option<(usize, Region)>;
}

impl<T> PhysicalMemoryProbeExt for T
where
    T: PhysicalMemoryProbe,
{
    fn regions_iter(&self) -> impl Iterator<Item = Region> {
        self.regions()
            .iter()
            .map(|region| region.align(FrameMgr::FRAME_SIZE))
    }

    fn entire_region(&self) -> Region {
        self.regions_iter()
            .reduce(Region::merge)
            .expect("no available memory region found")
    }

    fn find_free_contiguous_region(&self, size: usize) -> Option<(usize, Region)> {
        self.regions_iter()
            .enumerate()
            .find(|(_, region)| !region.is_empty() && size <= region.len())
            .map(|(index, region)| (index, Region::new(region.start, region.start + size)))
    }
}

#[derive(Debug)]
pub struct FrameMgr {
    mem_base: PAddr,
    mem_lists: [MemList; Self::MAX_ORDER],
    mem_metadata: &'static mut [MemMetadata],
}

impl Default for FrameMgr {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameMgr {
    /// 4 KiB
    pub const FRAME_SIZE: usize = 4096;

    /// 4 MiB
    pub const MAX_ORDER: usize = 11;

    pub const fn new() -> Self {
        Self {
            mem_base: PAddr::NULL,
            mem_lists: [MemList::EMPTY; Self::MAX_ORDER],
            mem_metadata: &mut [],
        }
    }

    #[inline]
    fn metadata(&self, frame: PAddr) -> Option<MemMetadata> {
        if frame < self.mem_base {
            return None;
        }

        let metadata_index = (frame - self.mem_base) / Self::FRAME_SIZE;
        self.mem_metadata.get(metadata_index).copied()
    }

    #[inline]
    fn set_metadata(&mut self, frame: PAddr, metadata: MemMetadata) {
        if frame < self.mem_base {
            return;
        }

        let metadata_index = (frame - self.mem_base) / Self::FRAME_SIZE;

        if let Some(metadata_mut) = self.mem_metadata.get_mut(metadata_index) {
            *metadata_mut = metadata;
        }
    }

    pub fn init<T: PhysicalMemoryProbe>(&mut self, probe: &mut T) {
        let mem_region = probe.entire_region();
        self.mem_base = mem_region.start;

        let mem_metadata_count = mem_region.len() / Self::FRAME_SIZE;
        let mem_metadata_size = mem_metadata_count * size_of::<MemMetadata>();

        let (mem_metadata_region_index, mem_metadata_region) =
            match probe.find_free_contiguous_region(mem_metadata_size) {
                Some(region) => region,
                None => {
                    panic!("not enough contiguous memory for memory metadata");
                }
            };
        self.mem_metadata = unsafe {
            core::slice::from_raw_parts_mut(
                mem_metadata_region.start.to_hhdm().to_mut_ptr(),
                mem_metadata_count,
            )
        };

        let original_region = probe.regions()[mem_metadata_region_index];
        let modified_region = Region::new(mem_metadata_region.end, original_region.end);
        probe.set_region(mem_metadata_region_index, modified_region);

        println!(
            "mem_metadata_size: {}KiB ({} ~ {})",
            mem_metadata_size / 1024,
            mem_metadata_region.start,
            mem_metadata_region.end
        );

        unsafe {
            // zero out the memory metadata, making all memory metadata as:
            // - order: 0
            // - is_free: false
            core::ptr::write_bytes::<MemMetadata>(
                self.mem_metadata.as_mut_ptr(),
                0,
                mem_metadata_count,
            );
        }

        for mut mem_region in probe.regions_iter() {
            if mem_region.is_empty() {
                continue;
            }

            println!(
                "mem_region: {} ~ {} (Size: {} KiB)",
                mem_region.start,
                mem_region.end,
                mem_region.len() / 1024
            );

            for order in (0..self.mem_lists.len()).rev() {
                if mem_region.is_empty() {
                    break;
                }

                let block_size = Self::FRAME_SIZE * (1 << order);
                let block_count = mem_region.len() / block_size;

                println!("block size: {} KiB", block_size / 1024);
                println!("block count: {}", block_count);

                if block_count == 0 {
                    continue;
                }

                unsafe {
                    // set all memory metadata as free and of the same order
                    core::ptr::write_bytes::<MemMetadata>(
                        &raw mut self.mem_metadata
                            [(mem_region.start - self.mem_base) / Self::FRAME_SIZE],
                        MemMetadata::new(order as u8, true).into(),
                        mem_region.len() / Self::FRAME_SIZE,
                    );
                }

                for block_index in 0..block_count {
                    let block_paddr = mem_region.start + block_index * block_size;
                    self.mem_lists[order].push(block_paddr);
                }

                mem_region.start += block_count * block_size;
            }
        }

        println!("mem_lists: {:#?}", self.mem_lists);
    }

    fn alloc_from_order(&mut self, order: usize) -> Option<(usize, PAddr)> {
        for order in order..self.mem_lists.len() {
            if let Some(frame) = self.mem_lists[order].pop() {
                return Some((order, frame));
            }
        }

        None
    }

    pub fn alloc(&mut self, frame_count: usize) -> Option<PAddr> {
        if frame_count == 0 {
            return None;
        }

        let target_order = frame_count.next_power_of_two().trailing_zeros() as usize;
        let (alloc_order, frame) = self.alloc_from_order(target_order)?;

        for order in (target_order..alloc_order).rev() {
            let block_size = Self::FRAME_SIZE * (1 << order);
            let buddy_frame = PAddr::from_raw_addr(frame.addr + block_size);
            self.set_metadata(buddy_frame, MemMetadata::new(order as u8, true));
            self.mem_lists[order].push(buddy_frame);
        }

        self.set_metadata(frame, MemMetadata::new(target_order as u8, false));
        Some(frame)
    }

    pub fn free(&mut self, frame: PAddr, frame_count: usize) {
        if frame_count == 0 {
            return;
        }

        let target_order = frame_count.next_power_of_two().trailing_zeros() as usize;
        let mut final_order = target_order;
        let mut final_frame = frame;

        for order in target_order..(self.mem_lists.len() - 1) {
            let block_size = Self::FRAME_SIZE * (1 << order);
            let buddy_frame = PAddr::from_raw_addr(final_frame.addr ^ block_size);
            let buddy_frame_metadata = match self.metadata(buddy_frame) {
                Some(metadata) => metadata,
                None => {
                    break;
                }
            };

            if buddy_frame_metadata.order() as usize != order || !buddy_frame_metadata.is_free() {
                break;
            }

            self.mem_lists[order].detach(buddy_frame);

            final_order = order + 1;
            final_frame = PAddr::from_raw_addr(final_frame.addr & buddy_frame.addr);
        }

        self.set_metadata(final_frame, MemMetadata::new(final_order as u8, true));
        self.mem_lists[final_order].push(final_frame);
    }
}
