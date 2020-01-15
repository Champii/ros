use super::temporary_page::TemporaryPage;
use crate::serial_println;
use alloc::boxed::Box;
use core::ops::{Deref, DerefMut};
use x86_64::{
    structures::paging::{
        mapper::{FlagUpdateError, MapToError, MapperFlush, TranslateError, UnmapError},
        FrameAllocator, Mapper, Page, PageSize, PageTable, PageTableFlags, PageTableIndex,
        PhysFrame, RecursivePageTable, Size4KiB, UnusedPhysFrame,
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
    let new_addr = VirtAddr::new(0xcafeb000);
    let phys = super::allocator::alloc_page(new_addr);

    let page_table4 = unsafe { get_page4_virt_ptr(new_addr) };

    page_table4.zero();
    page_table4[511].set_addr(phys, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);

    page_table4
}

pub fn remap_kernel(active: &mut ActivePageTable, multiboot_information_address: usize) {
    // let new_page_table_4 = new_page_table();

    serial_println!("HERE1");

    let mut temporary_page =
        TemporaryPage::new(Page::containing_address(VirtAddr::new(0xcafebabe)));

    serial_println!("HERE2");

    let mut new_page_table_4 = super::allocator::use_allocator(|falloc| {
        let frame = falloc.allocate_frame().expect("no more frames");

        InactivePageTable::new(
            PhysFrame::from_start_address(frame.start_address()).unwrap(),
            active,
            &mut temporary_page,
        )
    });
    serial_println!("HERE3");

    let boot_info = unsafe { multiboot2::load(multiboot_information_address) };

    active.with(&mut new_page_table_4, &mut temporary_page, |mapper| {
        serial_println!("ACTIVE PAGE TABLE");

        let elf_sections_tag = boot_info
            .elf_sections_tag()
            .expect("Memory map tag required");

        for section in elf_sections_tag.sections() {
            serial_println!("SECTION {:x}", section.start_address());

            if !section.is_allocated() {
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

                super::allocator::identity_map_with(phys_frame, flags, mapper);
            }
        }
        serial_println!("END_KERNEL_REMAP");
    });

    // active.switch(new_page_table_4);

    // let mutable_page_4 = unsafe { get_page4_virt_ptr(VirtAddr::from_ptr(P4)) };

    // page_table4
}

pub struct ActivePageTable {
    pub frame: PhysFrame<Size4KiB>,
    pub page_directory: RecursivePageTable<'static>,
}

impl ActivePageTable {
    pub unsafe fn new(multiboot_information_address: usize) -> Self {
        let frame = PhysFrame::<Size4KiB>::from_start_address(PhysAddr::new(0)).unwrap();

        let mutable_page_4 = unsafe { get_page4_virt_ptr(VirtAddr::from_ptr(P4)) };

        Self {
            frame: Self::init(multiboot_information_address),
            page_directory: RecursivePageTable::new(mutable_page_4).unwrap(),
        }
    }

    fn init(multiboot_information_address: usize) -> PhysFrame {
        let mutable_page_4 = unsafe { get_page4_virt_ptr(VirtAddr::from_ptr(P4)) };

        *super::allocator::MAPPER.lock() = Some(RecursivePageTable::new(mutable_page_4).unwrap());

        let frame_allocator =
            unsafe { BootInfoFrameAllocator::init(multiboot_information_address) };

        *super::allocator::FRAME_ALLOCATOR.lock() = Some(frame_allocator);

        let phys = super::allocator::translate_addr(VirtAddr::from_ptr(P4));

        PhysFrame::from_start_address(phys).unwrap()
    }

    pub fn with<F>(
        &mut self,
        inactive_page_table: &mut InactivePageTable,
        temporary_page: &mut TemporaryPage,
        f: F,
    ) where
        F: FnOnce(&mut RecursivePageTable<'static>),
    {
        // use x86_64::instructions::tlb;

        // let backup = x86_64::registers::control::Cr3::read();

        // let virt_addr = VirtAddr::from_ptr(&*new_page_table);
        // serial_println!("NEW PAGE VIRT ADDR 0x{:x}", virt_addr.as_u64());
        // let phys_addr = super::allocator::translate_addr(virt_addr);
        // serial_println!("NEW PAGE PHYS ADDR 0x{:x}", phys_addr.as_u64());

        // let current_page_table = unsafe { get_page4_virt_ptr(VirtAddr::from_ptr(P4)) };

        // let mutable_new_page_table_4_virt = unsafe { get_page4_virt_ptr(virt_addr) };

        // let current_page_virt2 = VirtAddr::from_ptr(current_page_table);
        // serial_println!(
        //     "1:CURRENT PAGE VIRT ADDR 0x{:x}",
        //     current_page_virt2.as_u64()
        // );
        // let phys_addr2 = super::allocator::translate_addr(current_page_virt2);
        // serial_println!("1:CURRENT PAGE PHYS ADDR 0x{:x}", phys_addr2.as_u64());

        // let new_page_virt = VirtAddr::from_ptr(mutable_new_page_table_4_virt);
        // let new_phys_addr = super::allocator::translate_addr(new_page_virt);

        // let mut on_heap = unsafe {
        //     RecursivePageTable::new_unchecked(
        //         mutable_new_page_table_4_virt,
        //         PageTableIndex::new(511),
        //     )
        // };

        // serial_println!(
        //     "RECURSIVE PAGE TABLE ADDR 0x{:x}",
        //     VirtAddr::from_ptr(&on_heap).as_u64()
        // );

        // // super::allocator::map_to_with(
        // //     VirtAddr::new(0xcafeb000),
        // //     unsafe {
        // //         UnusedPhysFrame::new(
        // //             PhysFrame::<Size4KiB>::from_start_address(new_phys_addr).unwrap(),
        // //         )
        // //     },
        // //     PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        // //     &mut on_heap,
        // // );

        // // Activate the new mapping
        // // current_page_table[511].set_addr(
        // //     new_phys_addr.clone(),
        // //     PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        // // );

        // // tlb::flush_all();

        // // // map new table into itself at  0xCAFEB000 (same mapping as old table)
        // // super::allocator::map_to_with(
        // //     VirtAddr::new(0xcafeb000),
        // //     unsafe {
        // //         UnusedPhysFrame::new(
        // //             PhysFrame::<Size4KiB>::from_start_address(new_phys_addr).unwrap(),
        // //         )
        // //     },
        // //     PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        // //     &mut on_heap,
        // // );

        // // // map old table into new one at  0xCAFEC000
        // // super::allocator::map_to_with(
        // //     VirtAddr::new(0xcafec000),
        // //     unsafe { UnusedPhysFrame::new(backup.0) },
        // //     PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        // //     &mut on_heap,
        // // );

        // f(&mut on_heap);

        // // let old = unsafe { get_page4_virt_ptr(VirtAddr::new(0xcafec000)) };

        // // old[511].set_addr(
        // //     backup.0.start_address(),
        // //     PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        // // );

        // // tlb::flush_all();

        // serial_println!("WOOT");

        // //
        // //
        // //
        {
            let backup = PhysFrame::containing_address(
                x86_64::registers::control::Cr3::read().0.start_address(),
            );

            let backup_unused = unsafe { UnusedPhysFrame::new(backup) };

            let p4_table = temporary_page.map_table_frame(backup_unused, self);

            let current_page_table = unsafe { get_page4_virt_ptr(VirtAddr::from_ptr(P4)) };

            current_page_table[511].set_addr(
                inactive_page_table.p4_frame.start_address(),
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
            );

            x86_64::instructions::tlb::flush_all();

            f(self);

            p4_table[511].set_addr(
                backup.start_address(),
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
            );

            x86_64::instructions::tlb::flush_all();
        }

        temporary_page.unmap(self);
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

impl Deref for ActivePageTable {
    type Target = RecursivePageTable<'static>;

    fn deref(&self) -> &Self::Target {
        &self.page_directory
    }
}

impl DerefMut for ActivePageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.page_directory
    }
}

// impl Mapper<Size4KiB> for ActivePageTable {
//     fn map_to<A>(
//         &mut self,
//         page: Page<Size4KiB>,
//         frame: UnusedPhysFrame<Size4KiB>,
//         flags: PageTableFlags,
//         falloc: &mut A,
//     ) -> Result<MapperFlush<Size4KiB>, MapToError>
//     where
//         Self: Sized,
//         A: FrameAllocator<Size4KiB>,
//     {
//         self.page_directory.map_to(page, frame, flags, falloc)
//     }

//     fn unmap(
//         &mut self,
//         page: Page<Size4KiB>,
//     ) -> Result<(PhysFrame<Size4KiB>, MapperFlush<Size4KiB>), UnmapError> {
//         self.page_directory.unmap(page)
//     }

//     fn update_flags(
//         &mut self,
//         page: Page<Size4KiB>,
//         flags: PageTableFlags,
//     ) -> Result<MapperFlush<Size4KiB>, FlagUpdateError> {
//         self.page_directory.update_flags(page, flags)
//     }

//     fn translate_page(&self, page: Page<Size4KiB>) -> Result<PhysFrame<Size4KiB>, TranslateError> {
//         self.page_directory.translate_page(page)
//     }
// }

pub struct InactivePageTable {
    p4_frame: PhysFrame,
}

impl InactivePageTable {
    pub fn new(
        frame: PhysFrame,
        active_table: &mut ActivePageTable,
        temporary_page: &mut TemporaryPage,
    ) -> InactivePageTable {
        {
            let table = temporary_page
                .map_table_frame(unsafe { UnusedPhysFrame::new(frame.clone()) }, active_table);

            table.zero();

            table[511].set_addr(
                frame.start_address(),
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
            );
        }
        temporary_page.unmap(active_table);

        InactivePageTable { p4_frame: frame }
    }
}
