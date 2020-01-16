use x86_64::structures::paging::{PageTableFlags, PhysFrame, UnusedPhysFrame};

pub struct InactivePageTable {
    pub p4_frame: PhysFrame,
}

impl InactivePageTable {
    pub fn new(
        frame: PhysFrame,
        active_table: &mut super::ActivePageTable,
        temporary_page: &mut super::TemporaryPage,
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
