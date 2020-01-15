use super::memory::BootInfoFrameAllocator;
use super::serial_println;
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

pub fn alloc_page(page_addr: VirtAddr) -> PhysAddr {
    let page_addr: Page<Size4KiB> = Page::containing_address(page_addr);

    // TODO: Check if page is already used
    use_allocator(|falloc| {
        let frame = falloc
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)
            .unwrap();

        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

        serial_println!(
            "Alloc page {:#?} -> {:#?}",
            // "Alloc page {:#?} ",
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
    })
}

pub fn translate_addr(virt: VirtAddr) -> PhysAddr {
    if let Some(mapper) = &*MAPPER.lock() {
        use x86_64::structures::paging::MapperAllSizes;

        mapper.translate_addr(virt).unwrap()
    } else {
        panic!("translate_addr(): Cannot get MAPPER");
    }
}

pub fn translate_addr_with(virt: VirtAddr, mapper: &RecursivePageTable<'static>) -> PhysAddr {
    use x86_64::structures::paging::MapperAllSizes;

    mapper.translate_addr(virt).unwrap()
}

pub fn map_to(page_addr: VirtAddr, frame: UnusedPhysFrame, flags: PageTableFlags) {
    if let Some(ref mut mapper) = *MAPPER.lock() {
        map_to_with(page_addr, frame, flags, mapper);
    } else {
        panic!("map_to(): Cannot get MAPPER");
    }
}

pub fn map_to_with<M, S>(
    page_addr: VirtAddr,
    frame: UnusedPhysFrame<S>,
    flags: PageTableFlags,
    mapper: &mut M,
) where
    S: PageSize,
    M: Mapper<S>,
{
    let page_addr: Page<S> = Page::containing_address(page_addr);

    use_allocator(|falloc| {
        serial_println!(
            "Alloc page (map_to) {:#?} -> {:#?}",
            page_addr,
            frame.start_address()
        );

        mapper
            .map_to(page_addr, frame, flags, falloc)
            .unwrap()
            .flush();
    });
}

pub fn map_to_with_alloc<M, S, F>(
    page_addr: VirtAddr,
    frame: UnusedPhysFrame<S>,
    flags: PageTableFlags,
    mapper: &mut M,
    falloc: &mut F,
) where
    S: PageSize,
    M: Mapper<S>,
    F: FrameAllocator<Size4KiB>,
{
    let page_addr: Page<S> = Page::containing_address(page_addr);

    serial_println!(
        "Alloc page (map_to) {:#?} -> {:#?}",
        page_addr,
        frame.start_address()
    );

    mapper
        .map_to(page_addr, frame, flags, falloc)
        .unwrap()
        .flush();
}

pub fn identity_map(frame: UnusedPhysFrame, flags: PageTableFlags) {
    if let Some(ref mut mapper) = *MAPPER.lock() {
        identity_map_with(frame, flags, mapper);
    } else {
        panic!("map_to(): Cannot get MAPPER");
    }
}

pub fn identity_map_with<M, S>(frame: UnusedPhysFrame<S>, flags: PageTableFlags, mapper: &mut M)
where
    S: PageSize,
    M: Mapper<S>,
{
    use_allocator(|falloc| {
        serial_println!(
            "Identity Alloc page (identity_map()) {:#?} ",
            frame.start_address()
        );

        unsafe { mapper.identity_map(frame, flags, falloc).unwrap() };
    });
}

pub fn use_allocator<F, R>(f: F) -> R
where
    F: FnOnce(&mut BootInfoFrameAllocator) -> R,
{
    if let Some(ref mut falloc) = *FRAME_ALLOCATOR.lock() {
        f(falloc)
    } else {
        panic!("map_to(): Cannot get FRAME_ALLOCATOR");
    }
}
