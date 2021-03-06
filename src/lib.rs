#![no_std]
#![cfg_attr(test, no_main)]
#![feature(
    custom_test_frameworks,
    abi_x86_interrupt,
    alloc_error_handler,
    allocator_api,
    lang_items
)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::panic::PanicInfo;

#[lang = "eh_personality"]
#[no_mangle]
pub extern "C" fn eh_personality() {}

pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod schedule;
pub mod serial;
pub mod vga_buffer;

use memory::paging::page_tables::ActivePageTable;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);

    hlt_loop();
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

pub fn init(multiboot_information_address: usize) {
    serial_println!("Kernel init");

    serial_println!("Init GDT:");
    gdt::init();

    serial_println!("Init IDT:");
    interrupts::init_idt();

    serial_println!("Enabling interrupts");
    x86_64::instructions::interrupts::enable();

    serial_println!("Init Paging");
    let mut active_page_table = unsafe { ActivePageTable::new(multiboot_information_address) };

    serial_println!("Remap Kernel");
    memory::paging::remap_kernel::remap_kernel(
        &mut active_page_table,
        multiboot_information_address,
    );

    serial_println!("Init Kernel Heap");
    memory::allocator::init_heap().expect("heap initialization failed");

    serial_println!("Starting Schduler");
    schedule::init();
}

// tests

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

// #[cfg(test)]
// entry_point!(test_kernel_main);
#[no_mangle]
#[cfg(test)]
pub extern "C" fn _start(multiboot_information_address: usize) -> ! {
    init(multiboot_information_address);

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

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn _start(multiboot_information_address: usize) -> ! {
    serial_println!("Starting kernel...");

    let boot_info = unsafe { multiboot2::load(multiboot_information_address) };
    // let boot_info = &boot_info;

    let memory_map_tag = boot_info.memory_map_tag().expect("Memory map tag required");

    serial_println!("memory areas:");
    for area in memory_map_tag.memory_areas() {
        serial_println!(
            "    start: 0x{:x}, end: 0x{:x}",
            area.start_address(),
            area.end_address(),
        );
    }
    // boot_info.
    serial_println!("modules: {:#?}", boot_info.module_tags());
    for module in boot_info.module_tags() {
        serial_println!(
            "module_start: {:x} {:x}",
            module.start_address(),
            module.end_address()
        );
    }

    let elf_sections_tag = boot_info
        .elf_sections_tag()
        .expect("Elf-sections tag required");

    serial_println!("kernel sections:");

    for section in elf_sections_tag.sections() {
        serial_println!(
            "    addr: 0x{:x}, end: 0x{:x}, flags: 0x{:x}",
            section.start_address(),
            section.end_address(),
            section.flags()
        );
    }

    let kernel_start = elf_sections_tag
        .sections()
        .map(|s| s.start_address())
        .min()
        .unwrap();
    let kernel_end = elf_sections_tag
        .sections()
        .map(|s| s.start_address() + s.size())
        .max()
        .unwrap();

    let multiboot_start = multiboot_information_address;
    let multiboot_end = multiboot_start + (boot_info.total_size() as usize);

    serial_println!("Kernel start-end: 0x{:x}-0x{:x}", kernel_start, kernel_end);
    serial_println!(
        "Multiboot start-end: 0x{:x}-0x{:x}",
        multiboot_start,
        multiboot_end
    );

    init(multiboot_information_address);

    use x86_64::{structures::paging::MapperAllSizes, VirtAddr};

    let addresses = [
        0,
        // the identity-mapped vga buffer page
        0xb8000,
        // some code page
        0x201008,
        // some stack page
        0x0100_0020_1a10,
        // virtual address mapped to physical address 0
        boot_info.start_address() as u64,
        0x000100000,
        0x444444440000,
        0xffffffff_fffff000,
    ];

    for &address in &addresses {
        let virt = VirtAddr::new(address);
        // new: use the `mapper.translate_addr` method
        if let Some(mapper) = &*memory::allocator::MAPPER.lock() {
            serial_println!("{:?} -> {:?}", virt, mapper.translate_addr(virt));
        };
    }

    serial_println!("Kernel started.");

    // use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};

    // // allocate a number on the heap
    // let heap_value = Box::new(41);
    // println!("heap_value at {:p}", heap_value);

    // // create a dynamically sized vector
    // let mut vec = Vec::new();
    // for i in 0..5000 {
    //     vec.push(i);
    // }

    // println!("vec at {:p}", vec.as_slice());

    // // create a reference counted vector -> will be freed when count reaches 0
    // let reference_counted = Rc::new(vec![1, 2, 3]);
    // let cloned_reference = reference_counted.clone();
    // println!(
    //     "current reference count is {}",
    //     Rc::strong_count(&cloned_reference)
    // );
    // core::mem::drop(reference_counted);
    // println!(
    //     "reference count is {} now",
    //     Rc::strong_count(&cloned_reference)
    // );

    #[cfg(test)]
    test_main();

    println!("OK!");

    hlt_loop();
}
