#![no_std]
#![no_main]

mod vga_buffer;

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Kernova: a kernel born like a new star");
    println!("nested fmt: {} + {} = {}", 20, 22, 20 + 22);
    print!("multi");
    print!("-part ");
    println!("line");

    loop {}
}

/// Prints the panic message + location to the VGA buffer, then spins.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
