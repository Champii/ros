use crate::serial_println;
use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use multiboot2::BootInformation;
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

use heapless::consts::*;
use heapless::{ArrayLength, Vec};

unsafe fn get_memory_map(multiboot_information_address: usize) -> &'static BootInformation {
    unsafe { &multiboot2::load(multiboot_information_address) }
}

pub struct AvailableFrames(Vec<UnusedPhysFrame<Size4KiB>, U512>);

impl AvailableFrames {
    pub fn new(multiboot_information_address: usize) -> Self {
        let boot_info: BootInformation = unsafe { multiboot2::load(multiboot_information_address) };

        let elf_sections_tag = boot_info
            .elf_sections_tag()
            .expect("Elf-sections tag required");

        let kernel_start = elf_sections_tag
            .sections()
            .map(|s| s.start_address())
            .min()
            .unwrap();
        let kernel_end = elf_sections_tag
            .sections()
            .map(|s| s.start_address() + s.size())
            .max()
            .unwrap();

        let multiboot_start = multiboot_information_address as u64;
        let multiboot_end = multiboot_start + (boot_info.total_size() as usize) as u64;

        let kernal_range = kernel_start..kernel_end;
        let multiboot_range = multiboot_start..multiboot_end;

        let regions = boot_info.memory_map_tag().unwrap().memory_areas();

        let addr_ranges = regions.skip(1).map(|r| r.start_address()..r.end_address());

        let frame_addresses = addr_ranges
            .flat_map(|r| r.step_by(4096))
            .filter(|r| !kernal_range.contains(&r) && !multiboot_range.contains(&r));

        // for lol in frame_addresses.clone() {
        //     serial_println!("AVAILABLE FRAME : {:x}", lol);
        // }

        let frames = frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)));

        let res = frames.map(|f| unsafe { UnusedPhysFrame::new(f) });

        let mut collec = Vec::<UnusedPhysFrame<Size4KiB>, U512>::new();

        serial_println!("SIZE {}", res.clone().count());
        for frame in res {
            collec.push(frame);
        }

        Self(collec)
    }
}

pub struct BootInfoFrameAllocator {
    available_frames: AvailableFrames,
    next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(multiboot_information_address: usize) -> Self {
        Self {
            available_frames: AvailableFrames::new(multiboot_information_address),
            next: 0,
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<UnusedPhysFrame> {
        // let frame = self.usable_frames().nth(self.next);
        let frame = self.available_frames.0.iter().nth(self.next);
        self.next += 1;
        frame.map(|f| unsafe { UnusedPhysFrame::new((*f).clone()) })
    }
}
