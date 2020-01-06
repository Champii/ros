#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(ros::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use ros::{hlt_loop, println};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("Hello World{}", "!");

    ros::init(boot_info);

    #[cfg(test)]
    test_main();

    println!("NOT CRASHED!");

    hlt_loop();
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);

    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    ros::test_panic_handler(info)
}
