#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(custom_test_frameworks)]
#![test_runner(metal_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use metal_os::println;
use x86_64::VirtAddr;

entry_point!(kernel_main);

// this function is the entry point, since the linker looks for a function
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use metal_os::allocator;
    use metal_os::memory::{self, BootInfoFrameAllocator};

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");
    metal_os::init();

    #[cfg(test)]
    test_main();
    metal_os::hlt_loop();
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    metal_os::hlt_loop();
}

// our panic handler in test mode
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    metal_os::test_panic_handler(info)
}
