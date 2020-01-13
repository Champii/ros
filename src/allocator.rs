use super::memory::BootInfoFrameAllocator;
use super::serial_println;
use core::alloc::{AllocErr, Layout};
use lazy_static::lazy_static;
use linked_list_allocator::LockedHeap;
use spin::Mutex;
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTable, PageTableFlags, PhysFrame,
        RecursivePageTable, Size4KiB, UnusedPhysFrame,
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

pub fn alloc_page(page_addr: VirtAddr) -> PhysAddr {
    let page_addr: Page<Size4KiB> = Page::containing_address(page_addr);

    // TODO: Check if page is already used
    if let Some(ref mut falloc) = *FRAME_ALLOCATOR.lock() {
        let frame = falloc
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)
            .unwrap();

        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

        serial_println!(
            "Alloc page {:#?} -> {:#?}",
            page_addr,
            frame.start_address()
        );

        if let Some(ref mut mapper) = *MAPPER.lock() {
            let start_addr = frame.start_address();

            mapper
                .map_to(page_addr, frame, flags, falloc)
                .unwrap()
                .flush();

            return start_addr;
        } else {
            panic!("alloc_page(): Cannot access MAPPER");
        }
    } else {
        panic!("alloc_page(): Cannot access FRAME_ALLOCATOR");
    }
}

pub fn translate_addr(virt: VirtAddr) -> PhysAddr {
    if let Some(mapper) = &*MAPPER.lock() {
        use x86_64::structures::paging::MapperAllSizes;

        mapper.translate_addr(virt).unwrap()
    } else {
        panic!("translate_addr(): Cannot get MAPPER");
    }
}

pub fn map_to(page_addr: VirtAddr, frame: UnusedPhysFrame, flags: PageTableFlags) {
    let page_addr: Page<Size4KiB> = Page::containing_address(page_addr);

    if let Some(ref mut falloc) = *FRAME_ALLOCATOR.lock() {
        serial_println!(
            "Alloc page (map_to) {:#?} -> {:#?}",
            page_addr,
            frame.start_address()
        );

        if let Some(ref mut mapper) = *MAPPER.lock() {
            mapper
                .map_to(page_addr, frame, flags, falloc)
                .unwrap()
                .flush();
        } else {
            panic!("map_to(): Cannot get MAPPER");
        }
    } else {
        panic!("map_to(): Cannot get FRAME_ALLOCATOR");
    }
}

pub fn identity_map(frame: UnusedPhysFrame, flags: PageTableFlags) {
    // let page_addr: Page<Size4KiB> = Page::containing_address(page_addr);

    if let Some(ref mut falloc) = *FRAME_ALLOCATOR.lock() {
        serial_println!(
            "Identity Alloc page (identity_map()) {:#?} ",
            frame.start_address()
        );

        if let Some(ref mut mapper) = *MAPPER.lock() {
            unsafe { mapper.identity_map(frame, flags, falloc).unwrap().flush() };
        } else {
            panic!("map_to(): Cannot get MAPPER");
        }
    } else {
        panic!("map_to(): Cannot get FRAME_ALLOCATOR");
    }
}

// pub fn kmalloc(size: usize) -> Result<core::ptr::NonNull<u8>, AllocErr> {
//     ALLOCATOR
//         .lock()
//         .allocate_first_fit(Layout::for_value(&size))
// }
