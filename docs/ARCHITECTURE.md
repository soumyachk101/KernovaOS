# ARCHITECTURE — Kernova

How the pieces fit together. Requirements → TRD.md · Build order → MILESTONES.md · Why → DECISIONS.md

## 1. Big picture

```
┌───────────────────────────── QEMU (x86-64) ─────────────────────────────┐
│ BIOS → bootloader crate → [long mode on, paging on, phys mem mapped]    │
│                  │                                                      │
│                  ▼  _start(&'static BootInfo)                           │
│  ┌───────────────────────────── kernel ─────────────────────────────┐   │
│  │ init: GDT+TSS → IDT → PIC remap → sti → paging/frames → heap     │   │
│  │                                                                  │   │
│  │  ISRs: timer ──┐                ┌── vga_buffer  (print!)         │   │
│  │     keyboard ──┤→ lock-free ────┤                                │   │
│  │  (minimal!)    │   queues       └── serial      (logs, tests)    │   │
│  │                ▼                                                 │   │
│  │  task layer: async executor  →  preemptive scheduler (M10)       │   │
│  │                ▼                                                 │   │
│  │  syscall boundary  (ring 3 ⇄ ring 0, int 0x80)                   │   │
│  │                ▼                                                 │   │
│  │  user programs ── shell ── VFS ── initrd (ustar, read-only)      │   │
│  └──────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

## 2. Boot flow

1. QEMU's BIOS loads the `bootimage`-built disk image.
2. The `bootloader` crate does the ugly part: real → protected → long mode, initial page
   tables, maps **all physical memory** at a fixed virtual offset, loads the kernel ELF, and
   jumps to `_start` with a `&'static BootInfo` (memory map + `physical_memory_offset`).
3. `_start` (declared through `entry_point!` for a type-checked signature) calls `init()`:
   GDT + TSS → IDT → PIC init → `sti` (enable interrupts) → frame allocator + mapper → heap →
   spawn initial tasks.
4. Idle behavior: `hlt` loop; from M9 the executor sleeps with interrupts-enabled `hlt` when
   its ready queue is empty.

Consequence of ADR-002: we skip 16/32-bit bring-up entirely and start life in a sane 64-bit
world. The trade is less control over early boot — acceptable for this project's goals.

## 3. Memory design

Physical memory:

- Ground truth is the BootInfo memory map (usable vs reserved regions).
- `BootInfoFrameAllocator`: an iterator over usable regions chunked into 4 KiB frames.
  v0.x does not free frames (kernel mappings are long-lived); replacing it with a
  bitmap/stack allocator that supports dealloc is a contained later refactor.

Virtual memory (x86-64 4-level paging, 4 KiB pages):

| Region | Where | Notes |
|---|---|---|
| Kernel image | wherever the bootloader linked/mapped it | code + statics |
| Complete physical memory | at `physical_memory_offset` | makes page-table walking simple pointer math |
| Kernel heap | `0x_4444_4444_0000`, ≥100 KiB | `linked_list_allocator::LockedHeap` as `#[global_allocator]`, frames mapped at init |
| IST double-fault stack | static 5-page (20 KiB) array | no guard page yet — documented, accepted risk |
| User space (M11+) | lower half, per-process | fresh PML4 per process; kernel mappings shared; `USER_ACCESSIBLE` flags on user pages |

## 4. Interrupts and exceptions

- One static IDT (`lazy_static`), handlers use the `x86-interrupt` calling convention.
- Exceptions: breakpoint → log and resume. Page fault → log CR2 + error code (panic in v0.x;
  becomes "kill the offending user process" at M11). Double fault → runs on IST stack 0, so a
  kernel stack overflow cannot cascade into a triple fault; prints state and halts.
- PIC 8259 pair remapped: master → vectors 32–39, slave → 40–47 (clear of CPU exceptions).
  EOI notified after every IRQ, correct chip order for slave lines.
- IRQ0 timer: increment tick counter; from M10 also drives preemption (see §6).
- IRQ1 keyboard: read scancode from port `0x60`, push into an `ArrayQueue`, wake the keyboard
  task. Decoding/printing happens in the task, not the ISR.

Rule: **ISRs do minimum work** — no allocation, no blocking locks. Shared writers that ISRs
also use (e.g. the VGA writer during early bring-up) are accessed by non-ISR code inside
`interrupts::without_interrupts` to prevent the classic print-deadlock.

## 5. Concurrency model

- Data shared with ISRs → lock-free queues only.
- Everything else → `spin::Mutex`, taken with interrupts disabled whenever any ISR could touch
  the same lock.
- Single core forever (v1.0), so no cross-CPU memory-model heroics — but we still code as if
  racing with ISRs, because we are.

## 6. Task layer — two deliberate stages (ADR-006)

Stage 1, cooperative (M9): `Task { id, future: Pin<Box<dyn Future<Output = ()>>> }`; executor
with a wake queue and a real `Waker` per task; `hlt`-if-idle. The keyboard pipeline becomes
ISR → scancode queue → async keyboard task (stream of decoded keys). This teaches
polling/wakers with zero context-switch assembly.

Stage 2, preemptive (M10): kernel `Thread`s, each with its own stack; context switch is a
small `asm!` routine saving callee-saved registers and swapping `rsp`; a round-robin run
queue; the timer tick sets a resched flag and the switch happens on the interrupt-return path
(keeps the ISR itself tiny). API: `spawn(fn)`, `yield_now()`, `exit()`. The async executor
survives as one thread among others.

## 7. User mode and syscalls (M11)

- GDT grows user code/data descriptors (DPL 3); TSS `privilege_stack_table` slot 0 holds the
  ring-0 stack used on ring-3 → ring-0 transitions.
- Entering ring 3: craft an `iretq` frame — user RIP, user CS (RPL 3), RFLAGS with IF set,
  user RSP, user SS (RPL 3) — and `iretq` into the program.
- Syscall path v1: an `int 0x80` IDT gate with DPL 3 → dispatcher. ABI: number in `rax`,
  args in `rdi/rsi/rdx`, return in `rax`. v2 (optional): `syscall`/`sysret` via
  EFER.SCE + STAR/LSTAR/SFMASK MSRs (ADR-009).
- v1 table: `0 read(fd, buf, len)` · `1 write(fd, buf, len)` · `2 exit(code)` · `3 getpid()`.
- Copy-in/copy-out: every user pointer is range-checked against the process's user region
  before the kernel touches it — minimal, but the habit matters.
- Isolation contract: a user program that dereferences garbage or runs a privileged
  instruction gets terminated; the kernel logs it and keeps running.

## 8. Filesystem and shell (M12–M13)

- initrd = a ustar (tar) archive produced from the `initrd/` directory at build time and
  embedded with `include_bytes!` — zero disk drivers needed (ADR-008).
- A small ustar parser exposes a `Vfs` trait: `open(path) -> File`, `read(&mut buf)`,
  `list(dir)`. Read-only by design.
- Shell = an async task: prompt, line editing (backspace), tokenizer; builtins hit the VFS;
  `run <prog>` loads a program from the initrd into a fresh user address space, schedules it,
  and waits for its exit code.

## 9. Error and panic policy

- `panic!` means an unrecoverable kernel bug: print message + location to VGA and serial,
  then `hlt` loop. In test builds, the panic handler reports `[failed]` and exits QEMU with
  the failure code instead (two `cfg`-gated handlers).
- Fallible operations return `Result` with small module-local error enums. `unwrap`/`expect`
  are acceptable only in `init()` paths and tests.

## 10. Source map (target state)

```
src/lib.rs          init(), test framework, re-exports — shared by main + tests/
src/main.rs         entry_point!, kernel_main, panic handlers
src/vga_buffer.rs   src/serial.rs
src/gdt.rs          src/interrupts.rs
src/memory.rs       src/allocator.rs
src/task/{mod,executor,keyboard}.rs        (M9)
src/sched/{mod,thread,switch}.rs           (M10)
src/usermode/{mod,syscall}.rs              (M11)
src/fs/{mod,ustar,vfs}.rs                  (M12)
src/shell.rs                               (M13)
tests/basic_boot.rs  tests/should_panic.rs  tests/stack_overflow.rs  tests/heap_allocation.rs
```
