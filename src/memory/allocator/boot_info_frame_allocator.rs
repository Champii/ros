use x86_64::{
    structures::paging::{FrameAllocator, PhysFrame, Size4KiB, UnusedPhysFrame},
    PhysAddr,
};

pub struct BootInfoFrameAllocator {
    multiboot_information_address: usize,
    next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(multiboot_information_address: usize) -> Self {
        Self {
            multiboot_information_address,
            next: 0,
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<UnusedPhysFrame> {
        let boot_info = unsafe { multiboot2::load(self.multiboot_information_address) };

        let elf_sections_tag = boot_info
            .elf_sections_tag()
            .expect("Elf-sections tag required");

        let kernel_start = elf_sections_tag
            .sections()
            .map(|s| s.start_address())
            .min()
            .unwrap();

        let multiboot_start = boot_info.start_address() as u64;
        let multiboot_end = multiboot_start + (boot_info.total_size() as u64);

        // Assuming that grub modules lies between kernel and multiboot structure
        let reserved_memory = kernel_start..multiboot_end;

        let regions = boot_info.memory_map_tag().unwrap().memory_areas();

        let addr_ranges = regions.map(|r| r.start_address()..r.end_address());

        let frame_addresses = addr_ranges
            .flat_map(|r| r.step_by(4096))
            .filter(|r| r > &0x100000 && !reserved_memory.contains(&r));

        let frames = frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)));

        let mut res = frames.map(|f| unsafe { UnusedPhysFrame::new(f) });

        let frame = res.nth(self.next);

        self.next += 1;

        frame.map(|f| unsafe { UnusedPhysFrame::new((*f).clone()) })
    }
}
