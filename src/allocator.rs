use super::memory::BootInfoFrameAllocator;
use super::serial_println;
use core::alloc::{AllocErr, Layout};
use lazy_static::lazy_static;
use linked_list_allocator::LockedHeap;
use spin::Mutex;
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, RecursivePageTable,
        Size4KiB,
    },
    VirtAddr,
};

lazy_static! {
    pub static ref MAPPER: Mutex<Option<RecursivePageTable<'static>>> = { Mutex::new(None) };
}

lazy_static! {
    pub static ref FRAME_ALLOCATOR: Mutex<Option<BootInfoFrameAllocator>> = { Mutex::new(None) };
}

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

pub fn init_heap() -> Result<(), MapToError> {
    serial_println!("HERE2");

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}

pub fn alloc_page(page_addr: VirtAddr) {
    serial_println!("Alloc page {:#?}", page_addr,);

    let page_addr: Page<Size4KiB> = Page::containing_address(page_addr);

    if let Some(ref mut falloc) = *FRAME_ALLOCATOR.lock() {
        let frame = falloc
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)
            .unwrap();

        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

        if let Some(ref mut mapper) = *MAPPER.lock() {
            mapper
                .map_to(page_addr, frame, flags, falloc)
                .unwrap()
                .flush();
        }
    }
}

pub fn kmalloc(size: usize) -> Result<core::ptr::NonNull<u8>, AllocErr> {
    ALLOCATOR
        .lock()
        .allocate_first_fit(Layout::for_value(&size))
}
