#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kernova::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use kernova::{print, println};
use kernova::task::{executor::Executor, keyboard, Task};

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
        kernova::allocator::init_heap(
            mapper.as_mut().unwrap(),
            frame_allocator.as_mut().unwrap(),
        )
        .expect("heap initialization failed");
    }

    #[cfg(test)]
    test_main();

    println!("It did not crash!");

    // initrd demo (M12): kernel-side ls + cat over the ustar VFS
    {
        use kernova::fs::{Initrd, Vfs};
        let fs = Initrd::new();
        print!("initrd ls:");
        for name in fs.list() {
            print!(" {}", name);
        }
        println!();
        if let Some(data) = fs.read("motd.txt") {
            print!("cat motd.txt: {}", core::str::from_utf8(data).unwrap_or("<binary>"));
        }
    }

    // ring 3 demos (M11): a well-behaved program, a wild pointer, and a
    // privileged instruction — the kernel must survive all three
    use kernova::usermode::{self, programs};
    let code = usermode::run(programs::hello());
    println!("user program exited with code {}", code);
    let code = usermode::run(programs::fault());
    println!("faulting user program reported code {}", code);
    let code = usermode::run(programs::privileged());
    println!("privileged user program reported code {}", code);

    // preemptive threads (M10): two CPU-bound loops with no yields, plus a
    // short-lived thread proving the exit path
    kernova::sched::init();
    kernova::sched::spawn(cpu_bound_one);
    kernova::sched::spawn(cpu_bound_two);
    kernova::sched::spawn(short_lived);

    // the async executor runs as thread 0 among the others (ADR-006)
    let mut executor = Executor::new();
    executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(interleave_task("A")));
    executor.spawn(Task::new(interleave_task("B")));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();
}

fn busy_work(units: u64) {
    for i in 0..units * 100_000 {
        core::hint::black_box(i); // keep the loop from being optimized away
    }
}

extern "C" fn cpu_bound_one() {
    let mut n = 0u64;
    loop {
        busy_work(5);
        n += 1;
        println!("thread ONE tick {}", n);
    }
}

extern "C" fn cpu_bound_two() {
    let mut n = 0u64;
    loop {
        busy_work(5);
        n += 1;
        println!("thread TWO tick {}", n);
    }
}

extern "C" fn short_lived() {
    println!("short-lived thread ran and exits");
    // returning drops into thread_exit via the trampoline
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
