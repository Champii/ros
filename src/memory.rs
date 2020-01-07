use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use x86_64::{
    structures::paging::{
        FrameAllocator, PageTable, PhysFrame, RecursivePageTable, Size4KiB, UnusedPhysFrame,
    },
    PhysAddr, VirtAddr,
};
pub const P4: *mut PageTable = 0xffffffff_fffff000 as *mut _;
unsafe fn active_level_4_table() -> &'static mut PageTable {
    // let page_table_ptr: *mut PageTable = physical_memory_offset.as_mut_ptr();

    &mut *P4
}

pub unsafe fn init() -> RecursivePageTable<'static> {
    let level_4_table = active_level_4_table();
    RecursivePageTable::new(level_4_table).expect("New Recursive page table")
}

pub struct BootInfoFrameAllocator {
    // memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init() -> Self {
        BootInfoFrameAllocator {
            // memory_map,
            next: 0,
        }
    }
    /// Returns an iterator over the usable frames specified in the memory map.
    fn usable_frames(&self) -> impl Iterator<Item = UnusedPhysFrame> {
        // get usable regions from memory map
        let regions = (*super::BOOTINFO.lock())
            .unwrap()
            .memory_map_tag()
            .unwrap()
            .memory_areas();
        // let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);
        // map each region to its address range
        let addr_ranges = regions.map(|r| r.base_addr..r.base_addr + r.length);
        // transform to an iterator of frame start addresses
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        // create `PhysFrame` types from the start addresses
        let frames = frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)));
        // we know that the frames are really unused
        frames.map(|f| unsafe { UnusedPhysFrame::new(f) })
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<UnusedPhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}
