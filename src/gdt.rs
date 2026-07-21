//! GDT + TSS. Exists mainly for the double-fault IST stack (and, later, user
//! segments — ADR-005).

use lazy_static::lazy_static;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            // SAFETY: taking the address only (no reference to mutable static
            // contents); the CPU uses this range solely as the double-fault
            // stack, which is never active twice concurrently (no nesting:
            // a fault inside the DF handler is a triple fault anyway).
            let stack_start = VirtAddr::from_ptr(&raw const STACK);
            // stack grows downward: pass the top (exclusive end)
            stack_start + STACK_SIZE as u64
        };
        // ring-0 stack the CPU switches to on any ring-3 → ring-0 transition
        // (int 0x80, IRQs and faults raised while in user mode)
        tss.privilege_stack_table[0] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            // SAFETY: address-of only, same argument as the IST stack above;
            // only one user program runs at a time (usermode.rs), so this
            // stack is never in use by two transitions at once.
            let stack_start = VirtAddr::from_ptr(&raw const STACK);
            stack_start + STACK_SIZE as u64
        };
        tss
    };
}

pub struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
    pub user_code_selector: SegmentSelector,
    pub user_data_selector: SegmentSelector,
}

lazy_static! {
    pub static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.append(Descriptor::kernel_code_segment());
        let tss_selector = gdt.append(Descriptor::tss_segment(&TSS));
        // DPL-3 segments; append returns selectors with RPL already = 3
        let user_data_selector = gdt.append(Descriptor::user_data_segment());
        let user_code_selector = gdt.append(Descriptor::user_code_segment());
        (
            gdt,
            Selectors {
                code_selector,
                tss_selector,
                user_code_selector,
                user_data_selector,
            },
        )
    };
}

pub fn init() {
    use x86_64::instructions::segmentation::{Segment, CS};
    use x86_64::instructions::tables::load_tss;

    GDT.0.load();
    // SAFETY: the selectors point into the GDT we just loaded, which lives in
    // a 'static lazy_static; the code selector is a valid kernel code segment
    // and the TSS selector references our 'static TSS.
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
}
