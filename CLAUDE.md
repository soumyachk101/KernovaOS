# CLAUDE.md — Kernova

Project memory for Claude Code. Read this first, every session. Name is final: **Kernova** — kernel + supernova: a kernel born like a new star.

## What this project is

Kernova is a hobby operating system kernel for **x86-64, written from scratch in Rust (`no_std`)**.
It boots via the `bootloader` crate, runs **only inside QEMU**, and is built milestone-by-milestone.
Primary learning references: "Writing an OS in Rust" (os.phil-opp.com), OSDev Wiki, OSTEP.

Document map:

| Doc | Purpose |
|---|---|
| `docs/PRD.md` | What we're building and why; scope, non-goals, release criteria |
| `docs/TRD.md` | Technical requirements: toolchain, approved crates, FR/NFR lists |
| `docs/ARCHITECTURE.md` | How subsystems fit: boot, memory, interrupts, tasks, syscalls, fs |
| `docs/MILESTONES.md` | **Source of truth for what to build next** — order + acceptance criteria |
| `docs/DEVELOPMENT.md` | Environment setup, config files, build/run/debug workflow |
| `docs/TESTING.md` | QEMU-based test framework and per-milestone test requirements |
| `docs/DECISIONS.md` | ADR log — why key choices were made; add entries for new choices |
| `docs/TOOLING.md` | MCP servers (Serena, Context7) + task→tool routing table |
| `docs/VERIFICATION.md` | **Anti-hallucination protocol** — mandatory rules R1–R10 |
| `docs/REFERENCES.md` | Source URLs per milestone + verified ground-truth constants table |

## Golden rules (never break these)

1. **One milestone at a time.** Check "Current status" below, open `docs/MILESTONES.md`,
   implement only that milestone. Do not start the next until acceptance criteria pass.
2. **Every change must boot.** After any code change, `cargo run` must reach the kernel and
   `cargo test` must pass in QEMU. A commit that triple-faults is worse than no commit.
3. **QEMU only.** Never produce commands or scripts targeting real hardware or real disks
   (no `dd` to `/dev/sd*`, no USB flashing instructions).
4. **Every `unsafe` block and `asm!` needs a `// SAFETY:` comment** explaining why the
   invariants hold. No exceptions.
5. **No `std`.** Freestanding binary: `#![no_std]`, `#![no_main]`. Only `core`, `alloc`
   (after M8), and the approved crates listed in `docs/TRD.md`. A new crate requires an ADR.
6. **Don't bump the toolchain or dependencies casually.** Nightly + crate versions are pinned
   (`rust-toolchain.toml`, `Cargo.toml`). Upgrades are a dedicated task with a full test run.
7. **Interrupt handlers do minimum work.** Never allocate, never take a blocking lock inside
   an ISR; push to lock-free queues and return. See ARCHITECTURE.md → Concurrency.
8. **x86-64 only.** Never mix in 32-bit / protected-mode tutorial code (different asm,
   different memory setup).
9. **Never guess — verify.** Crate APIs via Context7/docs.rs for the *pinned* version;
   hardware constants only from `docs/REFERENCES.md` or a freshly fetched OSDev/Intel SDM
   page. Full protocol: `docs/VERIFICATION.md` (rules R1–R10). A guessed constant here is a
   triple fault, not a bug.
10. **Claims require evidence.** Never state "it boots" or "tests pass" without running the
    command in this session and showing the real output.

## Commands

```bash
cargo build                 # compile kernel
cargo run                   # build bootimage + launch QEMU
cargo test                  # unit + integration tests in QEMU (headless, serial output)
cargo test --test <name>    # single integration test binary
```

Manual QEMU / debugging (full flows in `docs/DEVELOPMENT.md`):

```bash
qemu-system-x86_64 \
  -drive format=raw,file=target/x86_64-kernova/debug/bootimage-kernova.bin \
  -serial stdio
# GDB: add `-s -S` to QEMU, then: gdb target/.../kernova -ex "target remote :1234"
```

## Repo layout (target state — files appear as their milestone lands; no empty placeholders)

```
CLAUDE.md               ← you are here
.mcp.json               project-scoped MCP servers: Serena + Context7 (see docs/TOOLING.md)
.claude/commands/       /milestone and /check slash commands
rust-toolchain.toml     pinned nightly + components
x86_64-kernova.json     custom target spec
.cargo/config.toml      build-std, default target, bootimage runner
Cargo.toml
src/
  main.rs               entry_point! / kernel_main, panic handlers
  lib.rs                init(), test framework, re-exports (shared with integration tests)
  vga_buffer.rs         VGA text output → print!/println!
  serial.rs             COM1 logging → serial_print!/serial_println!
  gdt.rs                GDT + TSS + IST stacks
  interrupts.rs         IDT, exception + IRQ handlers
  memory.rs             frame allocator + paging
  allocator.rs          kernel heap
  task/                 async executor + keyboard task (M9+)
  sched/                threads, context switch, round-robin (M10+)
  usermode/  fs/  shell.rs                    (M11–M13)
tests/                  integration tests (each is its own QEMU-run binary)
initrd/                 files packed into the ustar initrd (M12+)
docs/                   all project documents
```

## Working style for Claude Code

- Before coding, state which milestone and which acceptance criteria you're targeting.
  (Shortcut: run `/milestone`; run `/check` before every commit.)
- Route work per `docs/TOOLING.md`: Serena for symbol-level navigation/edits, Context7 before
  writing against any crate API, WebFetch for the milestone's source chapter.
- Small steps: make it boot → make it right → make it clean. Verify in QEMU between steps.
- Suspected triple fault / boot loop → follow the debugging checklist in `docs/DEVELOPMENT.md`
  (QEMU `-d int -no-reboot`) before changing code speculatively.
- An architectural choice not covered by existing docs → add an ADR to `docs/DECISIONS.md`.
- After a milestone passes: update "Current status" below, tick it in `docs/MILESTONES.md`,
  commit + tag (`m4`, `m5`, …).
- Commit messages: `M6: remap PIC and handle timer IRQ`.
- If docs and code conflict, code that passes tests wins — then fix the doc in the same commit.

## Current status

- Active milestone: **M0 — Environment ready** (see `docs/MILESTONES.md`)
- Last verified boot: none yet
- Known issues: none yet
