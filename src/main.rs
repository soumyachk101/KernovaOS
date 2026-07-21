#![no_std]
#![no_main]

use core::panic::PanicInfo;

static HELLO: &[u8] = b"Kernova: a kernel born like a new star";

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let vga_buffer = 0xb8000 as *mut u8;

    for (i, &byte) in HELLO.iter().enumerate() {
        // SAFETY: 0xB8000 is the VGA text buffer, identity-mapped by the
        // bootloader; we write within the first row (80 cells), byte pairs of
        // ASCII + color attribute, which is the format the hardware expects.
        unsafe {
            *vga_buffer.offset(i as isize * 2) = byte;
            *vga_buffer.offset(i as isize * 2 + 1) = 0xb;
        }
    }

    loop {}
}

/// Called on panic. No unwinding (panic = "abort"); nothing to do yet but spin.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
