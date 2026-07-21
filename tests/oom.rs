//! Allocating beyond HEAP_SIZE must panic cleanly (not corrupt or hang).

#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;
use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use kernova::allocator::{self, HEAP_SIZE};
use kernova::{exit_qemu, serial_print, serial_println, QemuExitCode};

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    use kernova::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    serial_print!("oom::alloc_beyond_heap_panics...\t");

    kernova::init();
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    // SAFETY: same contracts as in kernel_main (bootloader-provided).
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    let mut v: Vec<u8> = Vec::new();
    v.reserve(HEAP_SIZE * 2); // must fail: heap is only HEAP_SIZE

    serial_println!("[test did not panic]");
    exit_qemu(QemuExitCode::Failed);
    kernova::hlt_loop();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    kernova::hlt_loop();
}
