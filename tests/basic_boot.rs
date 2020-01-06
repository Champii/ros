#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(ros::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

use bootloader::{entry_point, BootInfo};

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    ros::init(boot_info);

    test_main();

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    ros::test_panic_handler(info)
}

use ros::{println, serial_print, serial_println};

#[test_case]
fn test_println() {
    serial_print!("test_println... ");
    println!("test_println output");
    serial_println!("[ok]");
}
