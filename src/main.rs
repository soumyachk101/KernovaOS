#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kernova::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use kernova::println;
use kernova::task::{executor::Executor, Task};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use kernova::memory;
    use x86_64::VirtAddr;

    println!("Kernova: a kernel born like a new star");

    kernova::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    // SAFETY: bootloader maps all physical memory at physical_memory_offset;
    // called exactly once.
    unsafe { memory::init_globals(phys_mem_offset, &boot_info.memory_map) };
    {
        let mut mapper = memory::MAPPER.lock();
        let mut frame_allocator = memory::FRAME_ALLOCATOR.lock();
        kernova::allocator::init_heap(mapper.as_mut().unwrap(), frame_allocator.as_mut().unwrap())
            .expect("heap initialization failed");
    }

    #[cfg(test)]
    test_main();

    // v1.0: boot straight into the interactive shell (async task on the
    // executor). usermode::run for `run <prog>` works without the preemptive
    // scheduler — one user program at a time (ADR-012).
    let mut executor = Executor::new();
    executor.spawn(Task::new(kernova::shell::run_shell()));
    executor.run();
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
