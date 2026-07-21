# TRD — Kernova (Technical Requirements Document)

Maps the PRD to concrete technical decisions and testable requirements.
Design details → ARCHITECTURE.md · Build order → MILESTONES.md · Rationale → DECISIONS.md

## 1. Target platform

- TR-1 CPU/arch: x86-64 in long mode. Single core (bootstrap processor only). No 32-bit code paths.
- TR-2 Execution environment: QEMU `qemu-system-x86_64` (default machine type). Bochs optional
  as a second opinion for tricky boot bugs. Real hardware is unsupported by policy.
- TR-3 Boot: the Rust `bootloader` crate, packaged into a BIOS-bootable disk image by
  `bootimage`. GRUB/Multiboot, Limine, and hand-rolled bootloaders are deliberately not used
  for v1.0 (ADR-002).
- TR-4 Kernel binary: ELF linked by `rust-lld`; `bootimage` combines it with the bootloader
  into a flat `.bin` disk image.

## 2. Language and toolchain

- TR-5 Rust nightly, pinned in `rust-toolchain.toml`; components `rust-src` and
  `llvm-tools-preview`. Nightly is required for `build-std`, `custom_test_frameworks`,
  and the `x86-interrupt` calling convention.
- TR-6 `#![no_std]` + `#![no_main]`; panic strategy = `abort` in both dev and release profiles.
- TR-7 Custom target `x86_64-kernova.json`: `os = none`, linker `rust-lld`, red zone disabled,
  MMX/SSE disabled with soft-float — the kernel must be interrupt-safe without saving FPU/SIMD
  state (consequence: no floating point in kernel code).
- TR-8 `-Z build-std = core, compiler_builtins` (plus `alloc` from M8) with
  `build-std-features = ["compiler-builtins-mem"]` to provide `memset`/`memcpy`/`memcmp`.
- TR-9 Host: Linux (native or WSL2) with `build-essential`, `qemu-system-x86`, `gdb`.

## 3. Approved dependencies

Keep this list short; adding a crate requires an ADR in DECISIONS.md. Pin versions to match
the phil-opp posts being followed — API drift across these crates is a real hazard, so prefer
the exact versions from the blog's Cargo.toml at each milestone and record them here once green.

| Crate | Purpose | Enters at |
|---|---|---|
| `bootloader` 0.9.x, feature `map_physical_memory` | boot + `BootInfo` (memory map, physical memory offset) | M1 |
| `volatile` | VGA buffer writes the optimizer cannot elide | M2 |
| `spin` | spinlock `Mutex` for `no_std` | M2 |
| `lazy_static` (feature `spin_no_std`) | one-time init of statics (writer, IDT, GDT) | M2 |
| `uart_16550` | COM1 serial driver | M3 |
| `x86_64` | typed IDT/GDT/TSS, paging structures, port I/O, control registers | M3 |
| `pic8259` | chained 8259 PIC driver | M6 |
| `pc-keyboard` | scancode set 1 → keycode decoding | M6 |
| `linked_list_allocator` | kernel heap allocator | M8 |
| `crossbeam-queue` (`default-features = false`, `alloc`) | lock-free `ArrayQueue` at the ISR boundary | M9 |
| `conquer-once` | interrupt-safe `OnceCell` for queue init | M9 |
| `futures-util` (`default-features = false`, `alloc`) | async executor plumbing | M9 |

- TR-10 No crate that requires `std`, OS threads, or syscalls. Use `default-features = false`
  wherever applicable and audit transitive deps stay `no_std`-clean.

## 4. Functional requirements

- FR-1 Boot: bootloader hands off to `_start(boot_info: &'static BootInfo)` (type-checked via
  the `entry_point!` macro); kernel banner visible within ~1 s of QEMU start.
- FR-2 Output: `print!`/`println!` to VGA text mode (80×25 buffer at `0xB8000`) with newline
  handling, scrolling, and color; `serial_print!`/`serial_println!` to COM1 (`0x3F8`); both
  callable from the panic handler.
- FR-3 Exceptions: IDT installed; breakpoint handler logs and resumes; page-fault handler
  prints the accessed address (CR2) and error code; double-fault handler runs on a dedicated
  IST stack so a kernel stack overflow can never escalate to a triple fault.
- FR-4 Hardware IRQs: both PICs remapped to vectors 32–47; timer (IRQ0) increments a tick
  counter; keyboard (IRQ1) reads port `0x60` and yields decoded keypresses; correct EOI after
  every IRQ; idle loop uses `hlt`, not a busy spin.
- FR-5 Memory: parse the BootInfo memory map; 4 KiB frame allocator over usable regions;
  inspect and create page-table mappings through an `OffsetPageTable`; kernel heap of at least
  100 KiB such that `Box`, `Vec`, `String`, `Rc` work; clean panic (not corruption) on OOM.
- FR-6 Multitasking, two stages: (a) cooperative async executor with proper wakers and an
  async keyboard task consuming a scancode queue; (b) preemptive round-robin kernel threads
  with timer-driven context switching and `spawn` / `yield_now` / `exit`.
- FR-7 User mode: ring-3 code and data segments, TSS ring-0 stack for privilege transitions;
  kernel can enter a user program; syscall path via `int 0x80` first (`syscall`/`sysret`
  optional later, ADR-009). ABI: syscall number in `rax`, args in `rdi`, `rsi`, `rdx`, return
  in `rax`. Minimum set: `read`, `write`, `exit`, `getpid`. User pointers are validated before
  the kernel dereferences them. A faulting or privilege-violating user program is terminated
  without harming the kernel.
- FR-8 Filesystem: read-only initrd — a ustar archive embedded via `include_bytes!` — behind a
  minimal VFS interface: `open(path)`, `read`, `list(dir)`.
- FR-9 Shell: line input with backspace over the keyboard stream; builtins `help`, `echo`,
  `ls`, `cat`, `clear`, `uptime`; `run <prog>` loads a program from the initrd into user mode
  and reports its exit code.

## 5. Non-functional requirements

- NFR-1 Testability: `cargo test` runs unit + integration tests headless in QEMU using the
  `isa-debug-exit` device (success maps to host exit code 33) with results over serial;
  includes at least one should-panic test and a stack-overflow/double-fault test. See TESTING.md.
- NFR-2 Reliability: no milestone merges with a known triple fault, deadlock, or happy-path
  panic; ISRs never allocate and never take locks that non-interrupt code holds without
  interrupts disabled.
- NFR-3 Unsafe hygiene: every `unsafe` block and `asm!` carries a `// SAFETY:` comment;
  `cargo clippy` clean (any allow-listed lints documented); `rustfmt` enforced.
- NFR-4 Iteration speed: clean debug build in ≲1 min on the dev machine; QEMU boot-to-banner
  ≲2 s. Kernel runtime performance is explicitly not a v1.0 goal.
- NFR-5 Reproducibility: a fresh Ubuntu/WSL2 machine can go from clone to green `cargo test`
  using only DEVELOPMENT.md.

## 6. Technical out-of-scope (v1.0)

SMP and APIC bring-up, ACPI table parsing beyond necessity, PCI(e) enumeration (until stretch),
power management, storage drivers (until stretch), KASLR/SMEP/SMAP-style mitigations —
listed here so "later" is deliberate, not accidental.
