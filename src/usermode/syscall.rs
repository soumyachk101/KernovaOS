//! int 0x80 syscall gate (ADR-009). ABI: number in rax, args in rdi/rsi/rdx,
//! return in rax; all other registers preserved.
//! Table: 0 read · 1 write · 2 exit · 3 getpid (ARCHITECTURE §7).

use super::{return_to_kernel, set_exit_code, USER_SPAN};
use crate::print;
use core::arch::naked_asm;

pub const SYS_READ: u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_EXIT: u64 = 2;
pub const SYS_GETPID: u64 = 3;

/// Raw IDT entry target for vector 0x80. Registered with set_handler_addr
/// (the x86-interrupt convention can't read the syscall registers).
#[unsafe(naked)]
pub extern "C" fn syscall_entry() {
    // SAFETY (asm contract): entered via a DPL-3 interrupt gate — interrupts
    // are off, we're on the TSS rsp0 stack, an iretq frame is on the stack.
    // We save every register the ABI promises to preserve, shuffle the
    // syscall ABI (rax,rdi,rsi,rdx) into SysV argument order, call the Rust
    // dispatcher, restore, and iretq. One `sub rsp,8` keeps the call 16-byte
    // aligned (5-word iretq frame + 8 pushes = rsp ≡ 8 mod 16).
    naked_asm!(
        "push rdi",
        "push rsi",
        "push rdx",
        "push rcx",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "mov rcx, rdx", // arg3
        "mov rdx, rsi", // arg2
        "mov rsi, rdi", // arg1
        "mov rdi, rax", // syscall number
        "sub rsp, 8",
        "call {dispatch}",
        "add rsp, 8",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rcx",
        "pop rdx",
        "pop rsi",
        "pop rdi",
        "iretq",
        dispatch = sym dispatch,
    )
}

extern "C" fn dispatch(num: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    match num {
        // read: no user-visible input source yet — always EOF
        SYS_READ => 0,
        SYS_WRITE => sys_write(a1, a2, a3),
        SYS_EXIT => {
            set_exit_code(a1 as i64);
            return_to_kernel();
        }
        // single user process at a time → constant pid
        SYS_GETPID => 1,
        _ => u64::MAX, // unknown syscall → -1
    }
}

fn sys_write(fd: u64, buf: u64, len: u64) -> u64 {
    if fd != 1 {
        return u64::MAX;
    }
    // validate the whole range lies inside the user region before touching it
    let end = match buf.checked_add(len) {
        Some(e) => e,
        None => return u64::MAX,
    };
    if !(USER_SPAN.contains(&buf) && end <= USER_SPAN.end) {
        return u64::MAX;
    }

    // SAFETY: range-checked above; user pages stay mapped for the duration
    // of the syscall (we are still inside the user's address space).
    let bytes = unsafe { core::slice::from_raw_parts(buf as *const u8, len as usize) };
    for &b in bytes {
        // printable passthrough; anything non-ASCII becomes '?'
        let c = if b == b'\n' || (0x20..=0x7e).contains(&b) {
            b as char
        } else {
            '?'
        };
        print!("{}", c);
    }
    len
}
