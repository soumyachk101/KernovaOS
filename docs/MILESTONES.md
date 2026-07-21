# MILESTONES — Kernova build plan

**Source of truth for execution order.** One milestone at a time; a milestone is DONE only
when every "Done when" line passes — automated where possible (TESTING.md), manually verified
in QEMU otherwise. Update the status checklist at the bottom and CLAUDE.md "Current status"
after each completion.

Ordering note vs the roadmap PDF: adapted to the Rust / phil-opp flow. (1) Serial logging
moves early to M3 because the entire test harness depends on it. (2) Our own GDT/TSS lands
*after* the IDT, at M5 — the bootloader already provides a working GDT, and ours exists mainly
for the double-fault IST stack and later user segments. Rationale recorded in ADR-005.

⏱ = rough part-time estimate · 📚 = primary references

---

## M0 — Environment ready ⏱ ~1 day
Build: rustup nightly + `rust-src` + `llvm-tools-preview`; apt `build-essential gdb qemu-system-x86`;
`cargo install bootimage`; repo skeleton (`rust-toolchain.toml`, target JSON, `.cargo/config.toml`,
this docs/ folder) committed.
Done when: `rustc +nightly -V` and `qemu-system-x86_64 --version` both work; initial commit pushed.
📚 DEVELOPMENT.md §1–2

## M1 — Freestanding kernel boots ⏱ 2–4 days
Build: `#![no_std] #![no_main]` binary with a panic handler; custom target; `bootloader` dep;
`bootimage` runner wired so `cargo run` boots QEMU. Goal: the CPU is executing *our* code.
Done when: `cargo run` opens QEMU and stays up — no crash, no reboot loop. Tag `m1`.
📚 phil-opp: "A Freestanding Rust Binary", "A Minimal Rust Kernel"

## M2 — VGA text output ⏱ 2–4 days
Build: `vga_buffer` module — `Writer` over `0xB8000` using `volatile`, color codes, newline +
scrolling; `print!`/`println!` macros behind `spin::Mutex` + `lazy_static`; panic handler
prints message + location to screen.
Done when: boot banner prints; a deliberate `panic!` shows file/line on screen; nested prints
don't deadlock.
📚 phil-opp: "VGA Text Mode"

## M3 — Serial + test harness ⏱ 2–4 days
Build: `serial` module (uart_16550, COM1); `custom_test_frameworks` runner; `QemuExitCode`
via `isa-debug-exit` (Success = 0x10 → host exit 33); bootimage `test-args` +
`test-success-exit-code`; split shared code into `lib.rs`; `tests/basic_boot.rs`,
`tests/should_panic.rs`.
Done when: `cargo test` runs fully headless, prints `[ok]` per test over serial, exits green;
a deliberately failing assert exits red.
📚 phil-opp: "Testing" · TESTING.md

## M4 — CPU exceptions (IDT) ⏱ 3–5 days
Build: static IDT via the `x86_64` crate; breakpoint handler (log + resume); page-fault
handler printing CR2 + error code; `init()` ordering established in `lib.rs`.
Done when: `int3` test resumes execution and passes; writing through a bad pointer produces a
readable page-fault report instead of an instant reboot.
📚 phil-opp: "CPU Exceptions"

## M5 — GDT/TSS + double fault ⏱ 3–5 days
Build: our own GDT (kernel code segment), TSS with IST slot 0 → dedicated double-fault stack;
double-fault handler registered with that IST.
Done when: `tests/stack_overflow.rs` (infinite recursion, custom test IDT) lands in the
double-fault handler — not a triple-fault reboot; normal boot unaffected.
📚 phil-opp: "Double Faults"

## M6 — Hardware interrupts ⏱ 1–2 weeks
Build: `pic8259` init remapped to 32/40; `sti`; timer handler (tick counter); keyboard handler
(port `0x60` → `pc-keyboard` decode → echo); EOIs everywhere; `hlt` idle loop; wrap shared-lock
prints in `without_interrupts` (the deadlock fix).
Done when: timer visibly ticks; typing echoes characters live; a 5-minute soak with held-down
keys produces no deadlock and no fault.
📚 phil-opp: "Hardware Interrupts"

## M7 — Paging + frame allocator ⏱ 2–4 weeks
Build: bootloader `map_physical_memory` feature; init an `OffsetPageTable` from CR3 + physical
memory offset; `BootInfoFrameAllocator` over usable regions; demo code translating virtual →
physical and creating a brand-new mapping.
Done when: translation demo prints sane physical addresses for kernel/VGA addresses; writing
through a newly created mapping to the VGA frame shows on screen; all prior tests still green.
📚 phil-opp: "Introduction to Paging", "Paging Implementation"

## M8 — Kernel heap ⏱ 1–2 weeks
Build: map the heap range (`0x_4444_4444_0000`, ≥100 KiB); `linked_list_allocator` as
`#[global_allocator]`; add `alloc` to build-std; `tests/heap_allocation.rs`
(box roundtrip, big `Vec` sum, many allocations, reuse-after-drop).
Done when: `Box`/`Vec`/`String`/`Rc` work in kernel code; heap tests green; allocation beyond
heap size panics cleanly.
📚 phil-opp: "Heap Allocation" (+ "Allocator Designs" as optional reading)

## M9 — Cooperative multitasking (async) ⏱ 1–2 weeks
Build: `Task` + executor with real wakers (`ArrayQueue` ready queue), `hlt` when idle;
keyboard ISR shrinks to push-scancode + wake; async keyboard task does decoding/printing;
a second demo task to show interleaving.
Done when: two async tasks interleave output; typing still echoes (now via the task); QEMU no
longer pegs a host core when idle.
📚 phil-opp: "Async/Await"

## M10 — Preemptive scheduler ⏱ 2–5 weeks
Build: kernel `Thread` with its own stack; context-switch `asm!` (callee-saved regs + `rsp`);
round-robin run queue; timer-driven preemption via resched-on-interrupt-return;
`spawn` / `yield_now` / `exit`; executor becomes one thread.
Done when: two CPU-bound loops with **no** yields visibly interleave counters on screen;
10-minute soak stable; keyboard and tests unaffected.
📚 OSDev Wiki: "Context Switching", "Kernel Multitasking"; xv6 `proc.c`/`swtch.S` as reading

## M11 — User mode + syscalls ⏱ 3–6 weeks
Build: user code/data GDT entries + TSS `rsp0`; `iretq` into an embedded ring-3 test program;
`int 0x80` gate (DPL 3) → dispatcher; syscalls `read`/`write`/`exit`/`getpid`; user-pointer
validation; per-process address space (fresh PML4, kernel half shared).
Done when: the user program prints via `sys_write` and exits via `sys_exit` with its code
reported; a user program that dereferences garbage or executes a privileged instruction is
killed while the kernel keeps running.
📚 OSDev Wiki: "Getting to Ring 3", "System Calls", "SYSCALL/SYSRET"; `x86_64` crate docs

## M12 — initrd filesystem ⏱ 1–3 weeks
Build: build step packs `initrd/` into a ustar archive; `include_bytes!` it; ustar parser with
unit tests; `Vfs` trait (`open`/`read`/`list`); user-program loading + `sys_read` served
through the VFS.
Done when: kernel-side `ls` and `cat` list and read initrd files correctly (including a file
>512 bytes spanning multiple tar blocks); parser unit tests green.
📚 OSDev Wiki: "USTAR", "Initrd"

## M13 — Shell → v1.0 ⏱ 2–4 weeks
Build: async shell task — prompt, line editing with backspace, tokenizer; builtins `help`,
`echo`, `clear`, `uptime`, `ls`, `cat`; `run <prog>` = initrd → user process → print exit code.
Done when: the scripted 10-minute demo (boot → every builtin → run two user programs → no
panic) passes twice in a row. Tag `v1.0`. 🎉
📚 everything above

## M14 — Stretch (post-v1.0, pick freely)
ATA PIO (then AHCI) disk reads → FAT32 read-only mount → proper ELF loader → e1000 NIC +
minimal ARP/ICMP. Each stretch item gets its own mini acceptance criteria + ADR before starting.
📚 OSDev Wiki respective pages

---

## Timeline sanity (mirrors the roadmap PDF, part-time)

| Phase | Milestones | Rough time |
|---|---|---|
| Boot + print + tests | M0–M3 | ~1–2 weeks |
| Interrupts + keyboard | M4–M6 | 2–4 weeks |
| Memory management | M7–M8 | 1–2.5 months |
| Multitasking | M9–M10 | 1–2 months |
| User mode + fs + shell | M11–M13 | 2–4 months |

## Status checklist

- [ ] M0 environment
- [ ] M1 boot
- [ ] M2 VGA
- [ ] M3 serial + tests
- [ ] M4 exceptions
- [ ] M5 GDT + double fault
- [ ] M6 hardware IRQs
- [ ] M7 paging
- [ ] M8 heap
- [ ] M9 async executor
- [ ] M10 preemptive scheduler
- [ ] M11 user mode + syscalls
- [ ] M12 initrd fs
- [ ] M13 shell / v1.0
