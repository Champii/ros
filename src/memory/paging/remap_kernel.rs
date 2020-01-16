use x86_64::{
    structures::paging::{Page, PageTableFlags, PhysFrame, Size4KiB, UnusedPhysFrame},
    PhysAddr, VirtAddr,
};

use super::page_tables::{ActivePageTable, InactivePageTable, TemporaryPage};
use crate::serial_println;

pub fn remap_kernel(active: &mut ActivePageTable, multiboot_information_address: usize) {
    let mut temporary_page =
        TemporaryPage::new(Page::containing_address(VirtAddr::new(0xcafebabe)));

    let mut new_page_table_4 = super::helpers::use_global_allocator(|falloc| {
        use x86_64::structures::paging::FrameAllocator;

        let frame = falloc.allocate_frame().expect("no more frames");

        InactivePageTable::new(
            PhysFrame::from_start_address(frame.start_address()).unwrap(),
            active,
            &mut temporary_page,
        )
    });

    let boot_info = unsafe { multiboot2::load(multiboot_information_address) };

    active.with(&mut new_page_table_4, &mut temporary_page, |mapper| {
        let elf_sections_tag = boot_info
            .elf_sections_tag()
            .expect("Memory map tag required");

        for section in elf_sections_tag.sections() {
            if !section.is_allocated() {
                continue;
            }

            assert!(
                section.start_address() % super::PAGE_SIZE == 0,
                "sections need to be page aligned"
            );

            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

            let start_frame: PhysFrame<Size4KiB> =
                PhysFrame::containing_address(PhysAddr::new(section.start_address()));
            let end_frame = PhysFrame::containing_address(PhysAddr::new(section.end_address() - 1));

            for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
                let phys_frame = unsafe { UnusedPhysFrame::new(frame) };

                super::helpers::identity_map_with(phys_frame, flags, mapper);
            }
        }

        // Remap VGA
        let flags = PageTableFlags::WRITABLE | PageTableFlags::PRESENT;
        let vga_buffer_frame = unsafe {
            UnusedPhysFrame::new(PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(
                0xb8000,
            )))
        };

        super::helpers::identity_map_with(vga_buffer_frame, flags, mapper);

        // Remap Multiboot Structure
        let multiboot_start = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(
            boot_info.start_address() as u64,
        ));
        let multiboot_end =
            PhysFrame::containing_address(PhysAddr::new(boot_info.end_address() as u64 - 1));

        for frame in PhysFrame::range_inclusive(multiboot_start, multiboot_end) {
            let phys_frame = unsafe { UnusedPhysFrame::new(frame) };

            super::helpers::identity_map_with(phys_frame, flags, mapper);
        }
    });

    active.switch(&mut new_page_table_4);
}
