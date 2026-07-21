//! IDT setup, CPU exception handlers, and hardware IRQ handlers.

use crate::println;
use core::sync::atomic::{AtomicU64, Ordering};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

// SAFETY: 32/40 are the canonical remap offsets (REFERENCES.md) — above the
// 32 CPU exception vectors and non-overlapping.
pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Timer ticks since interrupts were enabled (~18.2 Hz PIT default).
pub static TICKS: AtomicU64 = AtomicU64::new(0);

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        // SAFETY: DOUBLE_FAULT_IST_INDEX refers to a valid IST slot whose
        // stack is set up in gdt.rs and is used by no other handler.
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(crate::gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.general_protection_fault
            .set_handler_fn(general_protection_fault_handler);
        idt[InterruptIndex::Timer.as_u8()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_u8()].set_handler_fn(keyboard_interrupt_handler);
        // SAFETY: syscall_entry is a naked handler that preserves the full
        // register state and ends in iretq; DPL 3 lets ring-3 code invoke it.
        unsafe {
            idt[0x80]
                .set_handler_addr(x86_64::VirtAddr::new(
                    crate::usermode::syscall::syscall_entry as usize as u64,
                ))
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);
        }
        idt
    };
}

/// A fault raised from ring 3 kills the user program, not the kernel.
/// Returns only if the fault did NOT come from user mode.
fn kill_user_if_ring3(stack_frame: &InterruptStackFrame, what: &str) {
    use x86_64::PrivilegeLevel;

    if stack_frame.code_segment.rpl() == PrivilegeLevel::Ring3 {
        println!(
            "user program killed: {} at {:?}",
            what, stack_frame.instruction_pointer
        );
        crate::usermode::set_exit_code(139); // 128 + SIGSEGV, unix-flavored
        crate::usermode::return_to_kernel();
    }
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    kill_user_if_ring3(&stack_frame, "general protection fault");
    panic!(
        "EXCEPTION: GENERAL PROTECTION FAULT (error code {})\n{:#?}",
        error_code, stack_frame
    );
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    TICKS.fetch_add(1, Ordering::Relaxed);

    // SAFETY: EOI for the timer IRQ we are currently servicing; required or
    // the PIC never delivers another interrupt. Sent BEFORE preempting so the
    // PIC can deliver the next tick to whichever thread runs next.
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }

    // Preemption point: switches stacks and returns here later; the iretq at
    // the end of this handler then resumes the interrupted thread
    // (ARCHITECTURE §6 — resched on the interrupt-return path).
    crate::sched::preempt();
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    // SAFETY: 0x60 is the PS/2 data port (REFERENCES.md); reading it is how
    // the keyboard IRQ is acknowledged at the device level.
    let scancode: u8 = unsafe { port.read() };
    // minimum ISR work (rule 7): push + wake, decode happens in the task
    crate::task::keyboard::add_scancode(scancode);

    // SAFETY: EOI for the keyboard IRQ we are currently servicing.
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64, // always 0 for #DF (REFERENCES.md)
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    kill_user_if_ring3(&stack_frame, "page fault");

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    crate::hlt_loop();
}

#[test_case]
fn test_breakpoint_exception() {
    // invoke int3; the handler logs and returns, so execution must continue
    x86_64::instructions::interrupts::int3();
}
