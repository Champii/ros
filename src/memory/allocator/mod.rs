mod allocator;
mod boot_info_frame_allocator;

pub use allocator::{init_heap, FRAME_ALLOCATOR, MAPPER};
pub use boot_info_frame_allocator::BootInfoFrameAllocator;
