//! Preemptive round-robin scheduler (M10, ARCHITECTURE §6 stage 2).
//!
//! Locking rule: scheduler state is only touched with interrupts disabled —
//! either from the timer ISR (interrupt gate clears IF) or inside
//! `without_interrupts` — so the spin locks can never deadlock against
//! preemption.

pub mod switch;
pub mod thread;

use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;
use switch::context_switch;
use thread::Thread;

pub use thread::ThreadId;

static RUNQUEUE: Mutex<VecDeque<Box<Thread>>> = Mutex::new(VecDeque::new());
static CURRENT: Mutex<Option<Box<Thread>>> = Mutex::new(None);
/// Threads that called `exit`; their stacks are freed on a later `schedule`
/// (never while still running on them).
static EXITED: Mutex<Vec<Box<Thread>>> = Mutex::new(Vec::new());
static SCHEDULER_READY: AtomicBool = AtomicBool::new(false);

/// Adopt the current boot flow as thread 0 and enable timer preemption.
pub fn init() {
    *CURRENT.lock() = Some(Box::new(Thread::bootstrap()));
    SCHEDULER_READY.store(true, Ordering::Release);
}

/// Add a new kernel thread running `entry`.
pub fn spawn(entry: extern "C" fn()) -> ThreadId {
    let thread = Box::new(Thread::new(entry));
    let id = thread.id;
    x86_64::instructions::interrupts::without_interrupts(|| {
        RUNQUEUE.lock().push_back(thread);
    });
    id
}

/// Voluntarily give up the CPU.
pub fn yield_now() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        schedule(false);
    });
}

/// Terminate the current thread; never returns.
pub fn exit() -> ! {
    x86_64::instructions::interrupts::without_interrupts(|| {
        schedule(true);
    });
    unreachable!("exited thread was scheduled again");
}

/// C-ABI wrapper so the trampoline can `call` it if an entry fn returns.
pub extern "C" fn thread_exit() -> ! {
    exit()
}

/// Called by the timer ISR (interrupts already off). No-op until `init`.
pub(crate) fn preempt() {
    if SCHEDULER_READY.load(Ordering::Acquire) {
        schedule(false);
    }
}

/// Core switch. Caller must have interrupts disabled.
///
/// `exiting`: current thread is done — don't requeue it, park it in EXITED
/// for a later thread to free.
fn schedule(exiting: bool) {
    // free stacks of previously exited threads (safe: they're switched out)
    if let Some(mut exited) = EXITED.try_lock() {
        exited.clear();
    }

    let (old_sp_ptr, new_sp) = {
        let mut runqueue = RUNQUEUE.lock();
        let next = match runqueue.pop_front() {
            Some(t) => t,
            None if exiting => panic!("last runnable thread called exit()"),
            None => return, // nothing else to run; keep going
        };

        let mut current = CURRENT.lock();
        let mut old = current.take().expect("schedule with no current thread");
        // raw pointer into the Box — stable even as the Box moves between
        // containers, since the Thread itself stays put on the heap
        let old_sp_ptr: *mut usize = &mut old.saved_rsp;
        if exiting {
            EXITED.lock().push(old);
        } else {
            runqueue.push_back(old);
        }

        let new_sp = next.saved_rsp;
        *current = Some(next);
        (old_sp_ptr, new_sp)
    }; // all locks dropped before switching

    // SAFETY: old_sp_ptr points into a live heap-allocated Thread; new_sp is
    // a frame built by Thread::new or saved by a previous switch; interrupts
    // are off and no locks are held (contract of this fn).
    unsafe { context_switch(old_sp_ptr, new_sp) };
    // execution resumes here when this thread is scheduled again
}
