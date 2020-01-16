use crate::memory::allocator::BootInfoFrameAllocator;
use crate::serial_println;
use core::alloc::{AllocErr, Layout};
use lazy_static::lazy_static;
use linked_list_allocator::LockedHeap;
use spin::Mutex;
use x86_64::{
    structures::paging::{
        mapper::MapToError, page::PageSize, FrameAllocator, Mapper, Page, PageTable,
        PageTableFlags, PhysFrame, RecursivePageTable, Size4KiB, UnusedPhysFrame,
    },
    PhysAddr, VirtAddr,
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
