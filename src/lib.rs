#![no_std]
#![cfg_attr(test, no_main)]
#![feature(
    custom_test_frameworks,
    abi_x86_interrupt,
    alloc_error_handler,
    allocator_api
)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::BootInfo;
use core::panic::PanicInfo;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::VirtAddr;

lazy_static! {
    static ref BOOTINFO: Mutex<Option<&'static BootInfo>> = { Mutex::new(None) };
}

pub mod allocator;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod schedule;
pub mod serial;
pub mod vga_buffer;

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

pub fn init(boot_info: &'static BootInfo) {
    serial_println!("Kernel init: {:#?}", boot_info);

    use self::memory::BootInfoFrameAllocator;

    *BOOTINFO.lock() = Some(boot_info);

    gdt::init();

    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() };

    serial_println!("Enabling interrupts");

    x86_64::instructions::interrupts::enable();

    serial_println!("Init Paging");

    let level_4_table_addr = VirtAddr::new(boot_info.recursive_page_table_addr);
    let mapper = unsafe { memory::init(level_4_table_addr) };

    let frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    *allocator::MAPPER.lock() = Some(mapper);
    *allocator::FRAME_ALLOCATOR.lock() = Some(frame_allocator);

    serial_println!("Init Kernel Heap");

    allocator::init_heap().expect("heap initialization failed");

    serial_println!("Starting Schduler");

    schedule::init();
}

// tests

#[cfg(test)]
use bootloader::entry_point;

pub fn test_runner(tests: &[&dyn Fn()]) {
    serial_println!("Running {} tests", tests.len());

    for test in tests {
        test();
    }

    exit_qemu(QemuExitCode::Success);
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");

    serial_println!("Error: {}\n", info);

    exit_qemu(QemuExitCode::Failed);

    hlt_loop();
}

#[cfg(test)]
entry_point!(test_kernel_main);

#[cfg(test)]
fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
    init(boot_info);

    test_main();

    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}
