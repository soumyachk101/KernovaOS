//! Embedded ring-3 test programs: tiny position-independent machine-code
//! blobs assembled into the kernel image and copied into user pages by
//! `usermode::run`. Real program loading from the initrd arrives at M12/M13.

use core::arch::global_asm;

global_asm!(
    ".section .rodata",
    // --- hello: sys_write a message, sys_exit(42) ---------------------
    ".global user_prog_hello_start",
    ".global user_prog_hello_end",
    "user_prog_hello_start:",
    "    mov rax, 1",                       // SYS_WRITE
    "    mov rdi, 1",                       // fd = stdout
    "    lea rsi, [rip + 2f]",              // buf (rip-relative → PIC)
    "    mov rdx, 32",                      // len (bytes in the message below)
    "    int 0x80",
    "    mov rax, 2",                       // SYS_EXIT
    "    mov rdi, 42",
    "    int 0x80",
    "1:  jmp 1b",                           // unreachable safety net
    "2:  .ascii \"hello from ring 3 via sys_write\\n\"",  // 32 bytes",
    "3:",
    "user_prog_hello_end:",
    // --- getpid: sys_getpid, exit with the pid ------------------------
    ".global user_prog_getpid_start",
    ".global user_prog_getpid_end",
    "user_prog_getpid_start:",
    "    mov rax, 3",                       // SYS_GETPID
    "    int 0x80",
    "    mov rdi, rax",                      // exit code = pid
    "    mov rax, 2",                        // SYS_EXIT
    "    int 0x80",
    "user_prog_getpid_end:",
    // --- fault: dereference a garbage pointer -------------------------
    ".global user_prog_fault_start",
    ".global user_prog_fault_end",
    "user_prog_fault_start:",
    "    mov qword ptr [0xdead], 1",        // unmapped/user-forbidden → #PF
    "    mov rax, 2",
    "    mov rdi, 0",
    "    int 0x80",                         // never reached
    "user_prog_fault_end:",
    // --- privileged: execute hlt in ring 3 ----------------------------
    ".global user_prog_priv_start",
    ".global user_prog_priv_end",
    "user_prog_priv_start:",
    "    hlt",                              // privileged → #GP
    "    mov rax, 2",
    "    mov rdi, 0",
    "    int 0x80",                         // never reached
    "user_prog_priv_end:",
);

extern "C" {
    static user_prog_hello_start: u8;
    static user_prog_hello_end: u8;
    static user_prog_getpid_start: u8;
    static user_prog_getpid_end: u8;
    static user_prog_fault_start: u8;
    static user_prog_fault_end: u8;
    static user_prog_priv_start: u8;
    static user_prog_priv_end: u8;
}

fn blob(start: *const u8, end: *const u8) -> &'static [u8] {
    let len = end as usize - start as usize;
    // SAFETY: start/end delimit contiguous bytes in the kernel's .rodata.
    unsafe { core::slice::from_raw_parts(start, len) }
}

pub fn hello() -> &'static [u8] {
    // SAFETY: symbols defined in the global_asm block above.
    unsafe { blob(&raw const user_prog_hello_start, &raw const user_prog_hello_end) }
}

pub fn getpid() -> &'static [u8] {
    // SAFETY: as above.
    unsafe { blob(&raw const user_prog_getpid_start, &raw const user_prog_getpid_end) }
}

/// Look up a runnable program by name (used by the shell's `run` builtin).
pub fn by_name(name: &str) -> Option<&'static [u8]> {
    match name {
        "hello" => Some(hello()),
        "getpid" => Some(getpid()),
        "fault" => Some(fault()),
        "priv" => Some(privileged()),
        _ => None,
    }
}

pub fn fault() -> &'static [u8] {
    // SAFETY: as above.
    unsafe { blob(&raw const user_prog_fault_start, &raw const user_prog_fault_end) }
}

pub fn privileged() -> &'static [u8] {
    // SAFETY: as above.
    unsafe { blob(&raw const user_prog_priv_start, &raw const user_prog_priv_end) }
}
