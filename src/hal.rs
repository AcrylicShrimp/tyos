use fdt::Fdt;
use spinning_top::{Spinlock, guard::SpinlockGuard};
use tyos_mem::{frame_mgr::FrameMgr, physical_address::PhysicalAddress};

unsafe extern "C" {
    static _kernel_start: u8;
    static _kernel_end: u8;
}

static FRAME_MGR: Spinlock<FrameMgr> = Spinlock::new(FrameMgr::EMPTY);
static TIMER_FREQUENCY: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

pub fn get_frame_mgr() -> SpinlockGuard<'static, FrameMgr> {
    FRAME_MGR.lock()
}

pub fn get_timer_frequency() -> usize {
    TIMER_FREQUENCY.load(core::sync::atomic::Ordering::Relaxed)
}

pub fn init_hal(dtb_pa: usize) {
    let dtb_ptr = dtb_pa as *const u8;
    let mut frame_mgr = get_frame_mgr();

    unsafe {
        let header = *(dtb_ptr as *const [u32; 2]);
        let magic = header[0].to_be();
        let total_size = header[1].to_be() as usize;

        if magic != 0xd00d_feed {
            panic!("invalid dtb magic: {magic:x}");
        }

        let dtb_slice = core::slice::from_raw_parts(dtb_ptr, total_size);
        let fdt = match Fdt::new(dtb_slice) {
            Ok(fdt) => fdt,
            Err(err) => {
                panic!("failed to parse dtb: {err:?}");
            }
        };

        let dtb_start = PhysicalAddress::from_ptr(dtb_ptr);
        let dtb_end = dtb_start + total_size;
        let reserved_regions = find_reserved_regions(Region::new(dtb_start, dtb_end), &fdt);

        for region in fdt.memory().regions() {
            let starting_address = PhysicalAddress::from_ptr(region.starting_address);
            let size = region.size.unwrap_or(0);
            let end = starting_address + size;
            let region = Region::new(starting_address, end);

            register_region(&mut frame_mgr, region, &reserved_regions);
        }

        if let Some(cpus) = fdt.find_node("/cpus")
            && let Some(prop) = cpus.property("timebase-frequency")
        {
            let frequency = prop.as_usize().unwrap_or(0);
            TIMER_FREQUENCY.store(frequency, core::sync::atomic::Ordering::Relaxed);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Region {
    start: PhysicalAddress,
    end: PhysicalAddress,
}

impl Region {
    pub fn new(start: PhysicalAddress, end: PhysicalAddress) -> Self {
        Self { start, end }
    }

    pub fn is_valid(self) -> bool {
        self.start.addr < self.end.addr
    }

    pub fn is_overlap(self, other: Self) -> bool {
        (self.start.addr >= other.start.addr && self.start.addr < other.end.addr)
            || (other.start.addr >= self.start.addr && other.start.addr < self.end.addr)
    }
}

fn find_reserved_regions(dtb_region: Region, fdt: &Fdt) -> [Region; 128] {
    let mut count = 0;
    let mut regions = [Region::new(PhysicalAddress::NULL, PhysicalAddress::NULL); 128];

    // 1. kernel
    regions[0] = Region::new(
        PhysicalAddress::from_ptr(&raw const _kernel_start),
        PhysicalAddress::from_ptr(&raw const _kernel_end),
    );
    count += 1;

    // 2. dtb
    regions[1] = dtb_region;
    count += 1;

    // 3. reserved regions from fdt
    if let Some(reserved_memory) = fdt.find_node("/reserved-memory") {
        for node in reserved_memory.children() {
            let reg = match node.reg() {
                Some(reg) => reg,
                None => {
                    continue;
                }
            };

            for region in reg {
                if count == regions.len() {
                    panic!("too many reserved regions");
                }

                let starting_address = PhysicalAddress::from_ptr(region.starting_address);
                let end = starting_address + region.size.unwrap_or(0);

                regions[count] = Region::new(starting_address, end);
                count += 1;
            }
        }
    }

    regions
}

fn register_region(frame_mgr: &mut FrameMgr, region: Region, reserved_regions: &[Region]) {
    for (reserved_index, reserved_region) in reserved_regions.iter().enumerate() {
        if !reserved_region.is_overlap(region) {
            continue;
        }

        let upper_region = Region::new(region.start, reserved_region.start);
        let lower_region = Region::new(reserved_region.end, region.end);

        if upper_region.is_valid() {
            register_region(
                frame_mgr,
                upper_region,
                &reserved_regions[reserved_index + 1..],
            );
        }

        if lower_region.is_valid() {
            register_region(
                frame_mgr,
                lower_region,
                &reserved_regions[reserved_index + 1..],
            );
        }

        return;
    }

    frame_mgr.add_region(region.start, region.end - region.start);
}
