#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kernova::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use kernova::println;
use kernova::task::{executor::Executor, keyboard, Task};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use kernova::memory::{self, BootInfoFrameAllocator};
    use x86_64::structures::paging::Translate;
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

    kernova::allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    // heap smoke test: Box, Vec, String, Rc
    {
        use alloc::{boxed::Box, rc::Rc, string::String, vec::Vec};
        let boxed = Box::new(41);
        let mut v = Vec::new();
        for i in 0..500 {
            v.push(i);
        }
        let s = String::from("heap works");
        let rc = Rc::new(*boxed + 1);
        println!("{}: box+1={} vec_sum={} rc_count={}", s, rc, v.iter().sum::<i32>(), Rc::strong_count(&rc));
    }

    #[cfg(test)]
    test_main();

    println!("It did not crash!");

    let mut executor = Executor::new();
    executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(interleave_task("A")));
    executor.spawn(Task::new(interleave_task("B")));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();
}

async fn async_number() -> u32 {
    42
}

async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
}

/// Prints five numbered lines, yielding between each — run two of these and
/// the outputs interleave, proving cooperative scheduling.
async fn interleave_task(name: &'static str) {
    for i in 0..5 {
        println!("task {} step {}", name, i);
        kernova::task::yield_now().await;
    }
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
