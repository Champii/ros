use crate::serial_println;
use alloc::boxed::Box;
use x86_64::{
    structures::paging::{
        FrameAllocator, Mapper, PageTable, PageTableFlags, PageTableIndex, PhysFrame,
        RecursivePageTable, Size4KiB, UnusedPhysFrame,
    },
    PhysAddr, VirtAddr,
};

pub const PAGE_SIZE: u64 = 4096;
pub const P4: *mut PageTable = 0xffffffff_fffff000 as *mut _;

// unsafe fn boot_level_4_table() -> &'static mut PageTable {
//     &mut *P4
// }

pub unsafe fn get_page4_virt_ptr(virt_adr: VirtAddr) -> &'static mut PageTable {
    &mut *virt_adr.as_mut_ptr()
}

// unsafe fn active_level_4_table() -> &'static mut PageTable {
//     let (phys_addr, _) = x86_64::registers::control::Cr3::read();

//     serial_println!("ACTIVE PAGE PHYS_ADDR {:?}", phys_addr);

//     &mut *P4
// }

// pub unsafe fn init() -> RecursivePageTable<'static> {
//     let level_4_table = active_level_4_table();
//     RecursivePageTable::new(level_4_table).expect("New Recursive page table")
// }

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

pub fn new_page_table() -> &'static mut PageTable {
    use alloc::boxed::Box;
    use x86_64::structures::paging::page::PageSize;
    // let mut page_table4 = Box::new(PageTable::new());

    let new_addr = VirtAddr::new(0xcafeb000);

    let phys = super::allocator::alloc_page(new_addr);

    let mut page_table4 = unsafe { get_page4_virt_ptr(new_addr) };

    // let mut page_table4 = PageTable::new();

    // let pointer: *mut PageTable = page_table4;

    // PhysFrame::containing_address(address: PhysAddr);

    // let phys = super::allocator::translate_addr(VirtAddr::new(pointer as u64));

    page_table4.zero();

    page_table4[511].set_addr(phys, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);

    page_table4
}

pub fn remap_kernel(active: &mut ActivePageTable, multiboot_information_address: usize) {
    let new_page_table_4 = new_page_table();

    let boot_info = unsafe { multiboot2::load(multiboot_information_address) };

    active.with(new_page_table_4, |mapper| {
        serial_println!("ACTIVE PAGE TABLE");

        let elf_sections_tag = boot_info
            .elf_sections_tag()
            .expect("Memory map tag required");

        for section in elf_sections_tag.sections() {
            serial_println!("SECTION");
            // use self::entry::WRITABLE;

            if !section.is_allocated() {
                // section is not loaded to memory
                continue;
            }
            assert!(
                section.start_address() % PAGE_SIZE == 0,
                "sections need to be page aligned"
            );

            serial_println!(
                "mapping section at addr: {:#x}, size: {:#x}",
                section.start_address(),
                section.size()
            );

            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE; // TODO use real section flags

            let start_frame: PhysFrame<Size4KiB> =
                PhysFrame::containing_address(PhysAddr::new(section.start_address()));
            let end_frame = PhysFrame::containing_address(PhysAddr::new(section.end_address() - 1));

            for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
                let phys_frame = unsafe { UnusedPhysFrame::new(frame) };

                // super::allocator::identity_map(unsafe { UnusedPhysFrame::new(frame) }, flags);
                // mapper.identity_map(frame, flags, allocator);
                if let Some(ref mut falloc) = *super::allocator::FRAME_ALLOCATOR.lock() {
                    serial_println!(
                        "Identity Alloc page (identity_map()) {:#?} ",
                        frame.start_address()
                    );

                    if let Some(ref mut mapper) = *super::allocator::MAPPER.lock() {
                        unsafe {
                            mapper
                                .identity_map(phys_frame, flags, falloc)
                                .unwrap()
                                .flush()
                        };
                    } else {
                        panic!("map_to(): Cannot get MAPPER");
                    }
                } else {
                    panic!("map_to(): Cannot get FRAME_ALLOCATOR");
                }
            }
        }
    });

    // let mutable_page_4 = unsafe { get_page4_virt_ptr(VirtAddr::from_ptr(P4)) };

    // page_table4
}

pub struct ActivePageTable {
    // pub frame: PhysFrame,
// pub page_directory: RecursivePageTable<'static>,
}

impl ActivePageTable {
    pub unsafe fn new() -> Self {
        Self {
            // frame,
            // page_directory: init(),
        }
    }

    pub fn init(multiboot_information_address: usize) {
        let mutable_page_4 = unsafe { get_page4_virt_ptr(VirtAddr::from_ptr(P4)) };

        *super::allocator::MAPPER.lock() = Some(RecursivePageTable::new(mutable_page_4).unwrap());

        let frame_allocator =
            unsafe { BootInfoFrameAllocator::init(multiboot_information_address) };

        *super::allocator::FRAME_ALLOCATOR.lock() = Some(frame_allocator);
    }

    pub fn with<F>(&mut self, new_page_table: &'static mut PageTable, f: F)
    where
        F: FnOnce(RecursivePageTable<'static>),
    {
        use x86_64::instructions::tlb;

        let backup: PhysFrame<Size4KiB> = PhysFrame::containing_address(
            x86_64::registers::control::Cr3::read().0.start_address(),
        );

        let virt_addr = VirtAddr::from_ptr(&*new_page_table);

        let phys_addr = super::allocator::translate_addr(virt_addr);

        let mutable_new_page_table_4_virt = unsafe { get_page4_virt_ptr(virt_addr) };

        mutable_new_page_table_4_virt[511].set_addr(
            phys_addr,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        );

        tlb::flush_all();

        {
            let on_heap = unsafe {
                RecursivePageTable::new_unchecked(
                    mutable_new_page_table_4_virt,
                    PageTableIndex::new(511),
                )
            };

            f(on_heap);

            // on_heap.drop();
        }

        // mutable_new_page_table_4_virt[511].set_addr(
        //     backup.start_address(),
        //     PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        // );

        // tlb::flush_all();

        //
        //
        //

        // let page = super::allocator::alloc_page(page_addr: VirtAddr);

        // let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

        // super::allocator::map_to(virt_addr, frame: UnusedPhysFrame, flags: PageTableFlags)

        // self.page_directory
        //     .map_to(page, frame, flags, super::allocator::FRAME_ALLOCATOR.lock());
    }

    pub fn switch(&mut self, page_table_4: &mut PageTable) {
        // page_table_4[511].addr();
        let virt_addr = VirtAddr::from_ptr(&*page_table_4);

        let phys_p4 = super::allocator::translate_addr(virt_addr);

        let frame = PhysFrame::containing_address(phys_p4);

        // self.frame = frame.clone();

        unsafe {
            x86_64::registers::control::Cr3::write(
                frame,
                x86_64::registers::control::Cr3Flags::empty(),
            );
        };

        let mutable_page_4 = unsafe { get_page4_virt_ptr(virt_addr) };

        // self.page_directory = RecursivePageTable::new(mutable_page_4).unwrap();

        if let Some(ref mut mapper) = *super::allocator::MAPPER.lock() {
            *mapper = RecursivePageTable::new(mutable_page_4).unwrap();
        }
        // self.page_directory.

        // self.page_directory = RecursivePageTable::new(page_table_4).unwrap();
    }
}
