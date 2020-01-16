use core::ops::{Deref, DerefMut};
use x86_64::{
    structures::paging::{
        PageTableFlags, PhysFrame, RecursivePageTable, Size4KiB, UnusedPhysFrame,
    },
    VirtAddr,
};

use crate::memory::allocator::{BootInfoFrameAllocator, FRAME_ALLOCATOR, MAPPER};
use crate::memory::paging::{
    get_page4_virt_ptr,
    helpers::translate_addr,
    page_tables::{InactivePageTable, TemporaryPage},
    P4,
};

pub struct ActivePageTable {
    pub frame: PhysFrame<Size4KiB>,
    pub page_directory: RecursivePageTable<'static>,
}

impl ActivePageTable {
    pub unsafe fn new(multiboot_information_address: usize) -> Self {
        let mutable_page_4 = get_page4_virt_ptr(VirtAddr::from_ptr(P4));

        Self {
            frame: Self::init(multiboot_information_address),
            page_directory: RecursivePageTable::new(mutable_page_4).unwrap(),
        }
    }

    fn init(multiboot_information_address: usize) -> PhysFrame {
        let mutable_page_4 = unsafe { get_page4_virt_ptr(VirtAddr::from_ptr(P4)) };

        *MAPPER.lock() = Some(RecursivePageTable::new(mutable_page_4).unwrap());

        let frame_allocator =
            unsafe { BootInfoFrameAllocator::init(multiboot_information_address) };

        *FRAME_ALLOCATOR.lock() = Some(frame_allocator);

        let phys = translate_addr(VirtAddr::from_ptr(P4));

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

    pub fn switch(&mut self, new_page_table: &mut InactivePageTable) {
        let old_table = InactivePageTable {
            p4_frame: PhysFrame::containing_address(
                x86_64::registers::control::Cr3::read().0.start_address(),
            ),
        };

        unsafe {
            x86_64::registers::control::Cr3::write(
                new_page_table.p4_frame,
                x86_64::registers::control::Cr3Flags::empty(),
            );
        }

        // old_table
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
