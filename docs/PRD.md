# PRD — Kernova (Product Requirements Document)

Status: v1.0 draft · Owner: you · Last updated: 2026-07-21
Companion docs: TRD.md (technical requirements) · ARCHITECTURE.md · MILESTONES.md

## 1. Vision

Build a small but real operating system for x86-64 from scratch in Rust — one that boots,
handles interrupts, manages memory, runs multiple tasks, executes user-mode programs through
a syscall interface, and gives the user an interactive shell. The OS is the visible product;
the deeper product is the builder's systems-programming skill ("maine kernel likha" — for real).

## 2. Goals

- G1 — Learn OS internals hands-on: booting, interrupts, paging, allocation, scheduling,
  privilege levels, syscalls, filesystems.
- G2 — Produce a demo-able artifact: boot in QEMU → type commands into our own shell →
  run a user-mode program.
- G3 — Practice serious Rust: `no_std`, `unsafe` discipline, lock-free ISR patterns,
  async internals, custom test harnesses.
- G4 — Prove out a spec-driven workflow with Claude Code: docs → milestone → tests → code.

## 3. Users

- Primary: the developer (learning + portfolio piece).
- Secondary: other OS-dev learners reading the repo.

No end users and no production deployment — this framing drives every scope call below.

## 4. Product scope — what Kernova v1.0 does

Booted in QEMU, the v1.0 kernel must:

1. Boot via the `bootloader` crate and print to VGA text mode and the serial port.
2. Survive CPU exceptions: breakpoint resumes, page faults are reported with the faulting
   address, double faults are caught on a dedicated stack (never a triple-fault reboot).
3. Handle hardware interrupts: timer ticks and live keyboard input.
4. Manage memory: physical frame allocator, 4-level paging control, and a kernel heap
   (`Box`, `Vec`, `String` usable in kernel code).
5. Run multiple tasks: a cooperative async executor, then preemptive round-robin threads.
6. Enter user mode (ring 3) and service a small syscall set (write, read, exit, getpid).
7. Serve a read-only initrd filesystem and run a shell with basic commands
   (`help`, `echo`, `ls`, `cat`, `clear`, `uptime`, `run <prog>`).

## 5. Non-goals for v1.0

Explicitly out, so scope creep is a decision rather than an accident:

- Real-hardware support — QEMU (and optionally Bochs) only.
- GUI or any graphics beyond VGA text mode.
- Networking, USB, sound, or ACPI beyond the bare minimum.
- SMP / multi-core.
- Writable or persistent on-disk filesystems.
- POSIX compatibility or running existing Linux software.
- Security hardening beyond basic ring separation.

Stretch after v1.0 (optional, see MILESTONES.md M14): ATA/AHCI disk reads, FAT32 read-only
mount, a proper ELF loader, e1000 NIC with a toy network stack.

## 6. Releases and success criteria

| Release | Name | Definition of done |
|---|---|---|
| v0.1 | "It's alive" | M1–M3: boots our code, `println!` on VGA, serial logs, `cargo test` green |
| v0.2 | "Interactive" | M4–M6: exceptions handled, timer ticking, keypresses echo on screen |
| v0.3 | "Real memory" | M7–M8: paging under our control, heap working (`Vec`/`String` in kernel) |
| v0.4 | "Multitasking" | M9–M10: async executor + preemptive scheduler interleaving 2+ tasks |
| v0.5 | "Userland" | M11: a ring-3 program does syscalls and exits cleanly; a crashing user program does not take the kernel down |
| v1.0 | "Kernova" | M12–M13: shell over initrd fs; a scripted 10-minute demo runs twice without a panic |

Overall success metric: every milestone's acceptance criteria in MILESTONES.md pass, verified
by automated QEMU tests wherever automation is possible (TESTING.md).

## 7. Constraints and assumptions

- One part-time developer pairing with Claude Code; the timeline is elastic — the milestone
  estimates in MILESTONES.md (roughly 6–12+ months total) are guides, not deadlines.
- Toolchain is free: Rust nightly, Linux/WSL2 host, QEMU, GDB.
- Learning sources of truth: phil-opp's "Writing an OS in Rust", OSDev Wiki, OSTEP for theory,
  Intel SDM when hardware behavior is in question.

## 8. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Motivation dip during the long memory/scheduler stretch | Small milestones with a visible, working checkpoint at every step |
| Nightly or crate churn breaks the build | Pin toolchain + versions; upgrades are their own tested task (CLAUDE.md rule 6) |
| Debugging black holes (silent triple faults) | Serial-first logging, GDB workflow, triple-fault checklist in DEVELOPMENT.md |
| Scope creep toward networking/GUI/SMP | Non-goals list above; stretch items gated behind v1.0 |
| AI-generated code that "looks right" but corrupts state | Acceptance tests per milestone + soak checks; no merge without green `cargo test` |
