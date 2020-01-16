use core::ops::DerefMut;
use x86_64::{
    structures::paging::{
        mapper::MapToError, page::PageSize, FrameAllocator, Mapper, Page, PageTable,
        PageTableFlags, PhysFrame, RecursivePageTable, Size4KiB, UnusedPhysFrame,
    },
    PhysAddr, VirtAddr,
};

use super::ActivePageTable;

pub struct TemporaryPage {
    page: Page,
    allocator: TinyAllocator,
}

impl TemporaryPage {
    pub fn new(page: Page) -> TemporaryPage {
        TemporaryPage {
            page: page,
            allocator: TinyAllocator::new(),
        }
    }

    /// Maps the temporary page to the given frame in the active table.
    /// Returns the start address of the temporary page.
    pub fn map(&mut self, frame: UnusedPhysFrame, active_table: &mut ActivePageTable) -> VirtAddr {
        assert!(
            active_table.translate_page(self.page).is_err(),
            "temporary page is already mapped"
        );

        super::super::helpers::map_to_with_alloc(
            self.page.start_address(),
            frame,
            PageTableFlags::WRITABLE | PageTableFlags::PRESENT,
            active_table.deref_mut(),
            &mut self.allocator,
        );

        self.page.start_address()
    }

    pub fn unmap(&mut self, active_table: &mut ActivePageTable) {
        active_table.unmap(self.page);
    }

    pub fn map_table_frame(
        &mut self,
        frame: UnusedPhysFrame,
        active_table: &mut ActivePageTable,
    ) -> &mut PageTable {
        unsafe { &mut *(self.map(frame, active_table).as_mut_ptr()) }
    }
}

struct TinyAllocator([Option<UnusedPhysFrame>; 3]);

impl TinyAllocator {
    fn new() -> TinyAllocator {
        super::super::helpers::use_global_allocator(|falloc| {
            let mut f = || falloc.allocate_frame();
            let frames = [f(), f(), f()];

            TinyAllocator(frames)
        })
    }
}

unsafe impl FrameAllocator<Size4KiB> for TinyAllocator {
    fn allocate_frame(&mut self) -> Option<UnusedPhysFrame> {
        for frame_option in &mut self.0 {
            if frame_option.is_some() {
                return frame_option.take();
            }
        }
        None
    }
}
