#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kernova::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use kernova::println;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Kernova: a kernel born like a new star");

    kernova::init();

    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    kernova::hlt_loop();
}

/// Prints the panic message + location to the VGA buffer, then spins.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernova::test_panic_handler(info)
}
