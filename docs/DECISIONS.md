# DECISIONS — Architecture Decision Records (ADR log)

One entry per non-obvious technical choice: new crate, subsystem design, ordering change,
scope call. Claude Code: when you make such a choice mid-task, append an ADR here in the same
commit. Format: Status · Context · Decision · Consequences.

---

## ADR-001 — Rust (nightly, no_std) instead of C
Status: Accepted.
Context: ring-0 bugs become triple faults; we want maximum learning with survivable debugging.
Decision: Rust nightly, `#![no_std]`, following the phil-opp series — the best-documented
modern from-scratch path. Nightly is needed for `build-std`, `custom_test_frameworks`, and the
`x86-interrupt` ABI.
Consequences: pin the nightly; toolchain upgrades are dedicated, fully-tested tasks.

## ADR-002 — `bootloader` crate instead of GRUB/Multiboot or a hand-rolled bootloader
Status: Accepted.
Context: the roadmap's #1 pitfall is the "bootloader trap" — weeks of assembly before any
kernel learning happens.
Decision: use the Rust `bootloader` crate (0.9.x line) with `bootimage`; it delivers us into
long mode with all physical memory mapped and a structured `BootInfo`.
Consequences: BIOS boot only; less early-boot control. If UEFI or Multiboot specifics are ever
needed, migration paths are `bootloader` 0.11+ or Limine — that would be a new ADR.

## ADR-003 — x86-64 only, single core
Status: Accepted.
Context: mixing 32-bit and 64-bit tutorial material is a classic failure mode; SMP multiplies
every concurrency problem.
Decision: one architecture end-to-end (x86-64), bootstrap processor only through v1.0.
Consequences: no ARM/RISC-V ports; APIC/SMP deferred; simpler locking story (ADR-007).

## ADR-004 — QEMU as the only supported target
Status: Accepted.
Context: real-hardware reboots kill iteration speed; emulators give GDB stubs and exit devices.
Decision: QEMU is the platform. Bochs allowed as a second opinion on boot bugs. Real hardware
is out of scope and forbidden by CLAUDE.md rule 3.
Consequences: instant iterate + headless testing via `isa-debug-exit`; possible QEMU-specific
assumptions are an accepted risk for this project's goals.

## ADR-005 — Milestone order deviates from the roadmap PDF
Status: Accepted.
Context: the PDF orders GDT → IDT → … → serial (classic from-scratch sequence). Our stack
changes the economics: the bootloader supplies a working GDT, and the Rust testing workflow
needs serial immediately.
Decision: serial + test harness at M3; our own GDT/TSS at M5, motivated by the double-fault
IST stack — matching phil-opp's sequence.
Consequences: identical end state, earlier safety net; readers of the PDF should read this ADR
to understand the reshuffle.

## ADR-006 — Cooperative async executor (M9) before preemptive threads (M10)
Status: Accepted.
Context: schedulers involve two hard things at once — scheduling logic and context-switch asm.
Decision: learn wakers/polling first with an async executor (zero asm), turning the keyboard
path into ISR → queue → task; add preemptive threads afterwards.
Consequences: two task systems coexist for a while; the executor later runs as one kernel thread.

## ADR-007 — Sync primitives: `spin::Mutex` + lock-free `ArrayQueue` at the ISR boundary
Status: Accepted.
Context: single core, but ISRs still preempt mainline code — that's a real race.
Decision: anything an ISR touches goes through a lock-free queue; everything else uses spin
mutexes, taken with interrupts disabled whenever an ISR could contend for the same lock.
Consequences: the "handlers never block, never allocate" rule (CLAUDE.md rule 7) is enforced
in review; if SMP ever lands, this ADR gets superseded wholesale.

## ADR-008 — initrd = ustar archive via `include_bytes!` (no disk driver in v1.0)
Status: Accepted.
Context: filesystem/VFS concepts don't require yak-shaving an ATA/AHCI driver first.
Decision: pack `initrd/` into a ustar (tar) archive at build time and embed it in the kernel;
parse it behind a small VFS trait. Read-only.
Consequences: content changes rebuild the kernel; no persistence. Disk + FAT32 is the M14
stretch and its own ADR.

## ADR-009 — Syscall entry: `int 0x80` first, `syscall`/`sysret` later
Status: Accepted.
Context: `syscall`/`sysret` needs MSR setup (EFER.SCE, STAR, LSTAR, SFMASK) and careful
stack/GS handling; `int 0x80` reuses the already-working IDT machinery.
Decision: ship the syscall ABI over an `int 0x80` gate (DPL 3); treat `syscall`/`sysret` as an
optional optimization on top of a proven ABI.
Consequences: slower syscalls initially (irrelevant at our scale); a clean upgrade path.

---

## Template

```
## ADR-XXX — <title>
Status: Proposed | Accepted | Superseded by ADR-YYY
Context: <what forced a choice>
Decision: <what we chose>
Consequences: <what we gained, what we pay, what would trigger revisiting>
```
