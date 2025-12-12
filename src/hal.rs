use fdt::Fdt;
use spinning_top::{Spinlock, guard::SpinlockGuard};
use tyos_mem::{
    addr::{PAddr, VAddr},
    frame_mgr::{FrameMgr, PhysicalMemoryProbe, Region},
};

unsafe extern "C" {
    static _kernel_start: u8;
    static _kernel_end: u8;
}

static FRAME_MGR: Spinlock<FrameMgr> = Spinlock::new(FrameMgr::new());
static TIMER_FREQUENCY: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

pub fn get_frame_mgr() -> SpinlockGuard<'static, FrameMgr> {
    FRAME_MGR.lock()
}

pub fn get_timer_frequency() -> usize {
    TIMER_FREQUENCY.load(core::sync::atomic::Ordering::Relaxed)
}

pub fn init_hal(kernel_heap_size: usize, dtb_paddr: PAddr) {
    let dtb_ptr = dtb_paddr.to_hhdm().to_ptr::<u8>();
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

        let kernel_heap_start = VAddr::from_ptr(&raw const _kernel_end).from_kernel_to_physical();
        let kernel_heap_end = kernel_heap_start + kernel_heap_size;

        let dtb_start = VAddr::from_ptr(dtb_ptr).from_hhdm_to_physical();
        let dtb_end = dtb_start + total_size;

        let reserved_regions = find_reserved_regions(
            Region::new(kernel_heap_start, kernel_heap_end),
            Region::new(dtb_start, dtb_end),
            &fdt,
        );
        let available_regions = find_available_regions(&reserved_regions, &fdt);

        struct Probe<'a> {
            available_regions: &'a [Region],
        }

        impl<'a> Probe<'a> {
            fn new(available_regions: &'a [Region]) -> Self {
                Self { available_regions }
            }
        }

        impl PhysicalMemoryProbe for Probe<'_> {
            fn unaligned_regions_iter(&self) -> impl Iterator<Item = Region> {
                self.available_regions.iter().copied()
            }
        }

        frame_mgr.init(&Probe::new(&available_regions));

        if let Some(cpus) = fdt.find_node("/cpus")
            && let Some(prop) = cpus.property("timebase-frequency")
        {
            let frequency = prop.as_usize().unwrap_or(0);
            TIMER_FREQUENCY.store(frequency, core::sync::atomic::Ordering::Relaxed);
        }
    }
}

fn find_reserved_regions(
    kernel_heap_region: Region,
    dtb_region: Region,
    fdt: &Fdt,
) -> [Region; 64] {
    let mut count = 0;
    let mut regions = [Region::new(PAddr::NULL, PAddr::NULL); 64];

    // 1. kernel
    regions[0] = Region::new(
        VAddr::from_ptr(&raw const _kernel_start).from_kernel_to_physical(),
        VAddr::from_ptr(&raw const _kernel_end).from_kernel_to_physical(),
    );
    count += 1;

    // 2. kernel heap
    regions[1] = kernel_heap_region;
    count += 1;

    // 3. dtb
    regions[2] = dtb_region;
    count += 1;

    // 4. reserved regions from fdt
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

                let starting_address = PAddr::from_ptr(region.starting_address);
                let end = starting_address + region.size.unwrap_or(0);

                regions[count] = Region::new(starting_address, end);
                count += 1;
            }
        }
    }

    regions
}

fn find_available_regions(reserved_regions: &[Region], fdt: &Fdt) -> [Region; 128] {
    let mut count = 0;
    let mut regions = [Region::new(PAddr::NULL, PAddr::NULL); 128];

    for region in fdt.memory().regions() {
        let starting_address = PAddr::from_ptr(region.starting_address);
        let size = region.size.unwrap_or(0);
        let end = starting_address + size;
        let region = Region::new(starting_address, end);

        register_available_regions(region, reserved_regions, &mut count, &mut regions);
    }

    regions
}

fn register_available_regions(
    region: Region,
    reserved_regions: &[Region],
    available_regions_count: &mut usize,
    available_regions: &mut [Region; 128],
) {
    for (reserved_index, reserved_region) in reserved_regions.iter().enumerate() {
        if !reserved_region.is_overlap(region) {
            continue;
        }

        let upper_region = Region::new(region.start, reserved_region.start);
        let lower_region = Region::new(reserved_region.end, region.end);

        if !upper_region.is_empty() {
            register_available_regions(
                upper_region,
                &reserved_regions[reserved_index + 1..],
                available_regions_count,
                available_regions,
            );
        }

        if !lower_region.is_empty() {
            register_available_regions(
                lower_region,
                &reserved_regions[reserved_index + 1..],
                available_regions_count,
                available_regions,
            );
        }

        return;
    }

    let count = *available_regions_count;

    if count == available_regions.len() {
        panic!("too many available regions");
    }

    available_regions[count] = region;
    *available_regions_count += 1;
}
