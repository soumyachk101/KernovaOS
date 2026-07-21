//! Ring 3 execution (M11, ARCHITECTURE §7).
//!
//! Model: one user program at a time, run synchronously — `run()` builds a
//! fresh address space (kernel entries shared, user subtree at USER_BASE),
//! iretq's into ring 3, and returns when the program exits or is killed.
//! Preemption is paused while a user program runs (single rsp0 stack).

pub mod programs;
pub mod syscall;

use crate::gdt;
use crate::memory;
use core::arch::naked_asm;
use core::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use x86_64::registers::control::{Cr3, Cr3Flags};
use x86_64::structures::paging::{
    FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame,
};
use x86_64::VirtAddr;

/// Start of the user region: PML4 slot 32 (0x1000_0000_0000), far away from
/// every kernel mapping so the whole subtree can be user-accessible.
pub const USER_BASE: u64 = 0x1000_0000_0000;
/// User stack page start (stack grows down from USER_STACK_TOP).
pub const USER_STACK: u64 = USER_BASE + 0x10_0000;
pub const USER_STACK_TOP: u64 = USER_STACK + 0x1000;
/// Everything a user pointer may reference.
pub const USER_SPAN: core::ops::Range<u64> = USER_BASE..USER_STACK_TOP;

static EXIT_CODE: AtomicI64 = AtomicI64::new(0);
static IN_USER: AtomicBool = AtomicBool::new(false);
static mut KERNEL_SAVED_RSP: usize = 0;

/// True while a ring-3 program is running (gates preemption, see sched).
pub fn in_user() -> bool {
    IN_USER.load(Ordering::Acquire)
}

pub(crate) fn set_exit_code(code: i64) {
    EXIT_CODE.store(code, Ordering::Release);
}

/// Build an address space for `program`, enter ring 3, come back with its
/// exit code (or the kill code if it faulted).
pub fn run(program: &[u8]) -> i64 {
    let (pml4_frame, entry) = build_address_space(program);

    set_exit_code(0);
    IN_USER.store(true, Ordering::Release);

    let (old_frame, old_flags) = Cr3::read();
    let cs = gdt::GDT.1.user_code_selector.0 as u64;
    let ss = gdt::GDT.1.user_data_selector.0 as u64;

    // SAFETY: pml4_frame contains a copy of every kernel PML4 entry plus the
    // user subtree, so kernel code keeps executing across the switch; iretq
    // targets are valid mapped user pages; return path is return_to_kernel.
    unsafe {
        Cr3::write(pml4_frame, Cr3Flags::empty());
        enter_user(entry, USER_STACK_TOP, cs, ss);
        Cr3::write(old_frame, old_flags);
    }

    IN_USER.store(false, Ordering::Release);
    EXIT_CODE.load(Ordering::Acquire)
    // ponytail: user frames + PML4 are leaked (~5 pages/run); add frame
    // deallocation if program runs ever number in the thousands
}

/// Fresh PML4: all current (kernel) entries copied, user code + stack mapped
/// under PML4 slot 32 with USER_ACCESSIBLE at every level.
fn build_address_space(program: &[u8]) -> (PhysFrame, u64) {
    assert!(program.len() <= 4096, "user program larger than one page");

    let phys_offset = (*memory::PHYS_OFFSET.lock()).expect("memory globals not initialized");
    let mut alloc_guard = memory::FRAME_ALLOCATOR.lock();
    let frame_allocator = alloc_guard.as_mut().expect("memory globals not initialized");

    let phys_to_virt = |f: PhysFrame| -> *mut u8 {
        (phys_offset + f.start_address().as_u64()).as_mut_ptr()
    };

    // new PML4 = copy of the active one
    let pml4_frame = frame_allocator.allocate_frame().expect("out of frames");
    let (active_pml4, _) = Cr3::read();
    // SAFETY: both frames are mapped through the phys offset window; the new
    // frame is exclusively ours, the active one is only read.
    unsafe {
        core::ptr::copy_nonoverlapping(
            phys_to_virt(active_pml4),
            phys_to_virt(pml4_frame),
            4096,
        );
    }

    // code frame: copy the program in
    let code_frame = frame_allocator.allocate_frame().expect("out of frames");
    // SAFETY: fresh frame via the phys window; program fits (assert above).
    unsafe {
        let dst = phys_to_virt(code_frame);
        core::ptr::write_bytes(dst, 0, 4096);
        core::ptr::copy_nonoverlapping(program.as_ptr(), dst, program.len());
    }
    let stack_frame = frame_allocator.allocate_frame().expect("out of frames");
    // SAFETY: fresh frame, zeroed for hygiene.
    unsafe { core::ptr::write_bytes(phys_to_virt(stack_frame), 0, 4096) };

    // map user pages inside the NEW address space
    // SAFETY: pml4_frame was fully initialized above; phys offset window is
    // valid for all frames.
    let new_pml4: &mut PageTable = unsafe { &mut *(phys_to_virt(pml4_frame) as *mut PageTable) };
    let mut user_mapper = unsafe { OffsetPageTable::new(new_pml4, phys_offset) };

    let user_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
    let table_flags =
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;

    // SAFETY: USER_BASE / USER_STACK pages are unmapped in the fresh PML4
    // (slot 32 is untouched by the kernel), frames are freshly allocated.
    unsafe {
        user_mapper
            .map_to_with_table_flags(
                Page::containing_address(VirtAddr::new(USER_BASE)),
                code_frame,
                user_flags,
                table_flags,
                frame_allocator,
            )
            .expect("mapping user code failed")
            .ignore(); // not the active address space — nothing to flush
        user_mapper
            .map_to_with_table_flags(
                Page::containing_address(VirtAddr::new(USER_STACK)),
                stack_frame,
                user_flags | PageTableFlags::WRITABLE,
                table_flags,
                frame_allocator,
            )
            .expect("mapping user stack failed")
            .ignore();
    }

    (pml4_frame, USER_BASE)
}

/// Save kernel context, then iretq into ring 3.
/// Args: rdi = user rip, rsi = user rsp, rdx = user cs, rcx = user ss.
#[unsafe(naked)]
unsafe extern "C" fn enter_user(entry: u64, user_rsp: u64, cs: u64, ss: u64) {
    // SAFETY (asm contract): saves callee-saved regs + rsp exactly like
    // context_switch, then builds the SDM iretq frame (SS, RSP, RFLAGS, CS,
    // RIP pushed in that order). return_to_kernel undoes it.
    naked_asm!(
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "mov qword ptr [rip + {saved}], rsp",
        "push rcx",        // SS (RPL 3)
        "push rsi",        // user RSP
        "push 0x202",      // RFLAGS: IF | reserved bit 1
        "push rdx",        // CS (RPL 3)
        "push rdi",        // RIP
        "iretq",
        saved = sym KERNEL_SAVED_RSP,
    )
}

/// Abandon the current (user or syscall) context and resume the kernel right
/// after `enter_user`. Used by sys_exit and the fault-kill path.
#[unsafe(naked)]
pub(crate) extern "C" fn return_to_kernel() -> ! {
    // SAFETY (asm contract): KERNEL_SAVED_RSP was written by enter_user and
    // the frame it points to is still intact (the kernel thread is parked
    // inside run() while the user program executes).
    naked_asm!(
        "mov rsp, qword ptr [rip + {saved}]",
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",
        "ret",
        saved = sym KERNEL_SAVED_RSP,
    )
}
