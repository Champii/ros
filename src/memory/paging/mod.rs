use x86_64::{
    structures::paging::{PageTable, PageTableFlags},
    VirtAddr,
};

pub mod helpers;
pub mod page_tables;
pub mod remap_kernel;

pub const PAGE_SIZE: u64 = 4096;
pub const P4: *mut PageTable = 0xffffffff_fffff000 as *mut _;

pub unsafe fn get_page4_virt_ptr(virt_adr: VirtAddr) -> &'static mut PageTable {
    &mut *virt_adr.as_mut_ptr()
}

pub fn new_page_table() -> &'static mut PageTable {
    let new_addr = VirtAddr::new(0xcafeb000);
    let phys = helpers::alloc_page(new_addr);

    let page_table4 = unsafe { get_page4_virt_ptr(new_addr) };

    page_table4.zero();
    page_table4[511].set_addr(phys, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);

    page_table4
}
