//! The context switch: save callee-saved registers + rsp, swap stacks.
//! Everything else (caller-saved regs) is already saved by the calling
//! convention or the interrupt entry path.

use core::arch::naked_asm;

/// Saved-register frame layout on a switched-out stack, low → high:
/// `[r15][r14][r13][r12][rbx][rbp][return address]`
/// (must mirror the push/pop order below; thread.rs builds this frame by hand
/// for brand-new threads).
pub const SWITCH_FRAME_QWORDS: usize = 7;

/// Switch from the current context to `new_sp`.
///
/// Saves callee-saved registers on the current stack, stores the resulting
/// rsp through `old_sp`, then restores the frame at `new_sp` and returns
/// into the new context.
///
/// # Safety
/// - `old_sp` must be a valid place to store the outgoing rsp.
/// - `new_sp` must point at a frame with the exact layout above (either
///   created by a previous switch, or crafted by `Thread::new`).
/// - Caller must hold no locks that the incoming context might take
///   (interrupts are expected to be disabled across the call).
#[unsafe(naked)]
pub unsafe extern "C" fn context_switch(old_sp: *mut usize, new_sp: usize) {
    // SAFETY (asm contract): SysV args — rdi = old_sp, rsi = new_sp; we touch
    // only callee-saved registers and rsp, matching the documented frame.
    naked_asm!(
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "mov [rdi], rsp", // *old_sp = current stack top
        "mov rsp, rsi",   // switch to the new stack
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",
        "ret", // pops the new context's return address
    )
}

/// First-run entry of every thread. `Thread::new` places the real entry
/// function in the saved-r12 slot; interrupts are re-enabled (we arrive here
/// with IF clear, from the timer ISR or a yield's interrupt-free section).
#[unsafe(naked)]
pub extern "C" fn thread_trampoline() {
    // SAFETY (asm contract): r12 holds the thread entry `extern "C" fn()`;
    // rsp is 16-aligned here (frame built by Thread::new), so the call leaves
    // the callee with standard SysV alignment. The entry never returns here
    // without hitting `thread_exit` (fn() -> ! not enforced, so we call
    // exit explicitly after).
    naked_asm!(
        "sti",
        "call r12",
        "call {exit}", // if the entry fn ever returns, terminate the thread
        exit = sym crate::sched::thread_exit,
    )
}
