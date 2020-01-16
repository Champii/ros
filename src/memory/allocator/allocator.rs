use crate::memory::allocator::BootInfoFrameAllocator;
use crate::serial_println;
use lazy_static::lazy_static;
use linked_list_allocator::LockedHeap;
use spin::Mutex;
use x86_64::structures::paging::{mapper::MapToError, RecursivePageTable};
use x86_64::VirtAddr;

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
    super::super::paging::helpers::alloc_page(VirtAddr::new(HEAP_START as u64));

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}
