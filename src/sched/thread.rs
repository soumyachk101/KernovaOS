//! Kernel thread: an owned stack plus the saved rsp when switched out.

use super::switch::{thread_trampoline, SWITCH_FRAME_QWORDS};
use alloc::boxed::Box;
use alloc::vec;
use core::sync::atomic::{AtomicU64, Ordering};

const STACK_SIZE: usize = 16 * 1024; // 16 KiB per kernel thread

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadId(pub u64);

impl ThreadId {
    fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1); // 0 = bootstrap thread
        ThreadId(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

pub struct Thread {
    pub id: ThreadId,
    /// Saved stack pointer while the thread is not running. Only meaningful
    /// when the thread is parked in the run queue.
    pub saved_rsp: usize,
    /// Owned stack backing store. `None` for the bootstrap thread, which runs
    /// on the boot stack the bootloader gave us.
    _stack: Option<Box<[u8]>>,
}

impl Thread {
    /// Wraps the currently-running boot flow into a schedulable thread.
    pub fn bootstrap() -> Self {
        Thread {
            id: ThreadId(0),
            saved_rsp: 0, // filled by the first context_switch away from us
            _stack: None,
        }
    }

    /// New thread that will start in `thread_trampoline` and call `entry`.
    pub fn new(entry: extern "C" fn()) -> Self {
        let stack = vec![0u8; STACK_SIZE].into_boxed_slice();

        let stack_bottom = stack.as_ptr() as usize;
        let stack_top = (stack_bottom + STACK_SIZE) & !0xf; // 16-align

        // Build the initial switch frame (see switch.rs layout):
        // [r15][r14][r13][r12=entry][rbx][rbp][ret=trampoline]  ← stack_top
        let frame_words = SWITCH_FRAME_QWORDS;
        let rsp = stack_top - frame_words * 8;
        let frame = rsp as *mut usize;
        // SAFETY: rsp..stack_top lies inside the freshly allocated stack; we
        // write exactly SWITCH_FRAME_QWORDS words.
        unsafe {
            frame.add(0).write(0); // r15
            frame.add(1).write(0); // r14
            frame.add(2).write(0); // r13
            frame.add(3).write(entry as usize); // r12 → trampoline calls it
            frame.add(4).write(0); // rbx
            frame.add(5).write(0); // rbp
            frame.add(6).write(thread_trampoline as usize); // return address
        }

        Thread {
            id: ThreadId::next(),
            saved_rsp: rsp,
            _stack: Some(stack),
        }
    }
}
