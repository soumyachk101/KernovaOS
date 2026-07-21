# TESTING — strategy

A `no_std` kernel can't use Rust's default test harness (it needs `std` and an OS — which is
us). So every test runs **inside QEMU**, reports over **serial**, and terminates QEMU with a
**status code** the host asserts on. The mechanics follow phil-opp's "Testing" post.

## 1. Mechanics

- Crate attributes: `#![feature(custom_test_frameworks)]`,
  `#![test_runner(kernova::test_runner)]`, `#![reexport_test_harness_main = "test_main"]`
  — in `lib.rs`, `main.rs`, and each `tests/*.rs`.
- Exit device: QEMU `isa-debug-exit` on port `0xf4`.
  `enum QemuExitCode { Success = 0x10, Failed = 0x11 }`; writing the code makes QEMU exit with
  `(code << 1) | 1`, so success is host exit **33** (matches `test-success-exit-code` in
  Cargo.toml, see DEVELOPMENT.md §2).
- All test output goes through `serial_print!`; QEMU runs with `-display none -serial stdio`
  so `cargo test` is fully headless and CI-able.
- A `Testable` trait wraps each test fn to print `module::name...    [ok]`.
- Test-mode panic handler (`#[cfg(test)]`): print `[failed]` + panic info to serial → exit
  with `Failed`. The lib exposes `test_panic_handler` so integration tests reuse it.

## 2. Test types

| Type | Lives in | Notes | Examples |
|---|---|---|---|
| Unit | `#[cfg(test)] mod tests` inside a module | run within the lib test binary | VGA writer puts a line in the right buffer row; ustar header parse |
| Integration | `tests/*.rs` — each is its own kernel binary with its own `_start` | boots fresh, calls `init()` as needed | `basic_boot`, `heap_allocation` |
| No-harness / should-panic | `tests/*.rs` with `harness = false` in Cargo.toml | hand-rolled flow; inverted success | `should_panic`, `stack_overflow` |

## 3. Cornerstone tests by milestone

| Milestone | Test | Asserts |
|---|---|---|
| M3 | `basic_boot` | `println!` works right after boot without init |
| M3 | `should_panic` | a failing assertion panics → reported `[ok]` via inverted handler |
| M4 | breakpoint unit test | `int3` fires the handler and execution resumes |
| M5 | `stack_overflow` | infinite recursion → **double-fault handler** (custom test IDT with IST) → success exit from inside the handler |
| M8 | `heap_allocation` | box roundtrip; 1000 sequential allocs; large `Vec` sum; memory reused after drop (bounded heap) |
| M11 | user-mode smoke | wild user pointer / privileged instruction kills the *process*, kernel logs and continues (manual first, automated once process exit codes exist) |
| M12 | ustar unit tests | header fields, octal sizes, multi-block file content, path lookup |

## 4. Manual verification

Automation can't see the VGA screen or "feel" interactivity. Each milestone's "Done when"
lines in MILESTONES.md include manual checks — run `cargo run` and physically verify (typing
echoes, counters interleave, 5–10 minute soak without deadlock). Record the result in the
milestone's final commit message.

## 5. Discipline

- New feature ⇒ new or updated test in the same milestone. No green `cargo test`, no merge.
- Tests must be deterministic: no timing assertions tighter than QEMU jitter allows.
- Keep integration tests minimal-boot: only initialize what the test needs, so failures point
  at the subsystem under test.
- Later (nice-to-have): GitHub Actions job — install nightly + bootimage + qemu, run
  `cargo test` headless on every push.
