#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kernova::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use kernova::println;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use kernova::memory::{self, BootInfoFrameAllocator};
    use x86_64::structures::paging::{Page, Translate};
    use x86_64::VirtAddr;

    println!("Kernova: a kernel born like a new star");

    kernova::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    // SAFETY: bootloader maps all physical memory at physical_memory_offset;
    // called exactly once.
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    // SAFETY: memory map comes straight from the bootloader.
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    // translation demo: VGA, a kernel code addr, the physical mapping base
    let addresses = [
        0xb8000,
        0x201008,
        boot_info.physical_memory_offset,
    ];
    for &address in &addresses {
        let virt = VirtAddr::new(address);
        let phys = mapper.translate_addr(virt);
        println!("{:?} -> {:?}", virt, phys);
    }

    // map an unused page to the VGA frame and write through it
    let page = Page::containing_address(VirtAddr::new(0));
    memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);
    let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    // SAFETY: `page` was just mapped to the VGA frame; offset 400 is row 5,
    // column 0 — inside the 80×25 buffer. Volatile write, no reference kept.
    unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e) }; // "New!"

    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    kernova::hlt_loop();
}

/// Prints the panic message + location to the VGA buffer, then halts.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    kernova::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernova::test_panic_handler(info)
}
