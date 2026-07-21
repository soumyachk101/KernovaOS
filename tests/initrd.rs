#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kernova::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use kernova::fs::{Initrd, Vfs};

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    use kernova::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    kernova::init();
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    // SAFETY: bootloader-provided; called once.
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };
    kernova::allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap init failed");

    test_main();
    kernova::hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernova::test_panic_handler(info)
}

#[test_case]
fn lists_expected_files() {
    let fs = Initrd::new();
    let files = fs.list();
    assert!(files.contains(&"hello.txt"));
    assert!(files.contains(&"motd.txt"));
    assert!(files.contains(&"big.txt"));
}

#[test_case]
fn reads_small_file() {
    let fs = Initrd::new();
    let data = fs.read("hello.txt").expect("hello.txt missing");
    assert_eq!(data, b"hello from the initrd filesystem\n");
}

#[test_case]
fn reads_multiblock_file() {
    // big.txt is 2160 bytes → spans 5 tar data blocks; verify boundaries
    let fs = Initrd::new();
    let data = fs.read("big.txt").expect("big.txt missing");
    assert_eq!(data.len(), 2160);
    assert!(data.starts_with(b"line 000:"));
    assert!(data.ends_with(b"lazy dog\n"));
    // spot-check bytes straddling the first block boundary (512)
    assert_eq!(&data[512..521], b"fox jumps");
}

#[test_case]
fn missing_file_is_none() {
    let fs = Initrd::new();
    assert!(fs.read("does_not_exist").is_none());
    assert!(!fs.exists("does_not_exist"));
}
