# DEVELOPMENT — setup, workflow, debugging

## 1. One-time setup (Ubuntu / WSL2)

```bash
sudo apt update
sudo apt install -y build-essential git make gdb qemu-system-x86
rustup toolchain install nightly
rustup component add rust-src llvm-tools-preview --toolchain nightly
cargo install bootimage
```

Windows: use WSL2 (Ubuntu) — run QEMU inside WSL2. Native Linux is smoothest.

## 2. Project config files

`rust-toolchain.toml`
```toml
[toolchain]
channel = "nightly"        # once the build is green, pin an exact date: "nightly-YYYY-MM-DD"
components = ["rust-src", "llvm-tools-preview"]
```

`x86_64-kernova.json` — custom target. Start from the target file in phil-opp's `blog_os`
repository; the `data-layout` string must match the pinned nightly's LLVM, so if the compiler
ever complains about data layout after a toolchain change, copy the current string from that
repo rather than hand-editing it.
```json
{
  "llvm-target": "x86_64-unknown-none",
  "arch": "x86_64",
  "os": "none",
  "target-endian": "little",
  "target-pointer-width": "64",
  "target-c-int-width": "32",
  "executables": true,
  "linker-flavor": "ld.lld",
  "linker": "rust-lld",
  "panic-strategy": "abort",
  "disable-redzone": true,
  "features": "-mmx,-sse,+soft-float",
  "data-layout": "COPY-FROM-blog_os-FOR-YOUR-NIGHTLY"
}
```

`.cargo/config.toml`
```toml
[unstable]
build-std = ["core", "compiler_builtins"]       # add "alloc" at M8
build-std-features = ["compiler-builtins-mem"]

[build]
target = "x86_64-kernova.json"

[target.'cfg(target_os = "none")']
runner = "bootimage runner"
```

`Cargo.toml` (relevant extract — full crate list lives in TRD.md §3)
```toml
[dependencies]
bootloader = { version = "0.9", features = ["map_physical_memory"] }
# ... other approved crates as their milestone lands

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"

[package.metadata.bootimage]
test-args = [
  "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04",
  "-serial", "stdio",
  "-display", "none",
]
test-success-exit-code = 33          # (0x10 << 1) | 1
test-timeout = 300

# per-test harness overrides (M3+):
[[test]]
name = "should_panic"
harness = false

[[test]]
name = "stack_overflow"
harness = false
```

## 3. Daily loop

```bash
cargo run                              # build bootimage + boot QEMU window
cargo test                             # all tests, headless, serial-reported
cargo test --test heap_allocation      # a single integration test
cargo clippy && cargo fmt              # before every commit
```

Manual QEMU run — always keep serial attached; it's your primary console:
```bash
qemu-system-x86_64 \
  -drive format=raw,file=target/x86_64-kernova/debug/bootimage-kernova.bin \
  -serial stdio
```

## 4. Debugging

Serial first: sprinkle `serial_println!` — it keeps working when VGA or kernel state is broken,
and it's what the test harness reads.

GDB session:
```bash
# terminal 1 — QEMU waits for the debugger
qemu-system-x86_64 -drive format=raw,file=target/.../bootimage-kernova.bin \
  -serial stdio -s -S

# terminal 2 — note: symbols come from the ELF, not the .bin
gdb target/x86_64-kernova/debug/kernova \
  -ex "target remote :1234" \
  -ex "break kernel_main" \
  -ex "continue"
```
`rust-gdb` (ships with rustup) gives nicer Rust value printing.

QEMU introspection:

- `-d int,cpu_reset -no-reboot -no-shutdown` — logs every interrupt/exception; on a triple
  fault QEMU freezes instead of reboot-looping, so the last dumped state is readable.
- QEMU monitor (Ctrl+Alt+2 in the window): `info registers`, `info mem`, `info tlb`.
- `addr2line -e target/x86_64-kernova/debug/kernova <RIP>` → exact source line for a fault address.

### Triple-fault checklist (kernel instantly reboots / boot-loops)

1. Re-run with `-d int -no-reboot` and read the **last** exception: vector, error code, RIP.
2. Vector `0x0d` (GP) or `0x08` (DF) right after enabling something new? Suspect exactly that
   thing: missing IDT entry, wrong GDT selector, `sti` before the PIC was remapped, missing EOI.
3. Repeating page faults? Read CR2 — pattern near a stack top means stack overflow; an address
   in your new heap/mapping range means the mapping isn't actually there.
4. Just changed paging? Verify the kernel, its stack, and the VGA buffer are mapped in the
   *new* tables before switching CR3.
5. Map RIP to source with `addr2line`, then fix with knowledge, not guesses.

### Common errors → causes

| Symptom | Likely cause |
|---|---|
| linker: undefined `memset`/`memcpy`/`memcmp` | `build-std-features = ["compiler-builtins-mem"]` missing |
| `error[E0463]: can't find crate core` | `rust-src` component missing, or target JSON name/path typo |
| Deadlock the moment the timer works | ISR prints while mainline holds the writer lock → wrap mainline prints in `without_interrupts` (M6) |
| Works with `cargo run`, hangs in `cargo test` | isa-debug-exit args missing, or test never calls the exit function |
| Garbage characters on VGA | wrote non-CP437 bytes, or forgot `volatile` writes |
| Boots, then resets after ~seconds | timer IRQ firing with no handler/EOI once `sti` executed |

## 5. Git workflow

- Branch per milestone: `m7-paging`. Merge to `main` only when acceptance criteria pass and
  `cargo test` is green.
- Commits: `M7: map heap pages via OffsetPageTable` — prefix with the milestone.
- Tag each completed milestone (`m1` … `m13`) and the release (`v1.0`).
- Never commit `target/` or built bootimages. Do commit `Cargo.lock` (binary project —
  reproducible builds matter here).

## 6. Claude Code usage

- Claude Code auto-loads `CLAUDE.md` from the repo root — keep its "Current status" section
  true, it's how each session knows where the project stands.
- Good session opener: "Read CLAUDE.md and docs/MILESTONES.md. We are on M<n>. Implement it,
  then run cargo test and show me the serial output."
- Reference docs: https://docs.claude.com/en/docs/claude-code/overview
