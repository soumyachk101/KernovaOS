# VERIFICATION — anti-hallucination protocol

This is a ring-0 project: a hallucinated constant doesn't throw an exception, it triple-faults
the machine. So the cost of guessing is maximal and this protocol is **mandatory** for Claude
Code (and humans). Tool routing lives in `docs/TOOLING.md`; ground-truth facts live in
`docs/REFERENCES.md`.

## Trust order (when sources disagree)

1. Code in this repo that compiles and passes `cargo test` — observed behavior wins.
2. This repo's `docs/` (especially the REFERENCES.md constants table and DECISIONS.md).
3. The **pinned version's** crate documentation (Context7 / docs.rs with explicit version).
4. The actual phil-opp chapter, freshly fetched — not remembered.
5. OSDev Wiki.
6. Intel SDM / AMD manuals — final word on CPU behavior.

## Hard rules

- **R1 — Crate APIs are looked up, not recalled.** Before writing a call into `bootloader`,
  `x86_64`, `pic8259`, `pc-keyboard`, `linked_list_allocator`, etc., verify the item exists
  with that name and signature **in our pinned version** (Context7 or docs.rs). Version drift
  is this project's #1 hallucination source.
- **R2 — Hardware constants come only from REFERENCES.md or a fetched source.** Never invent
  port numbers, MSR indices, interrupt vectors, bit layouts, or header offsets. If it's not in
  the table and not fetched, it doesn't go in the code.
- **R3 — Every `asm!` block is cross-checked** against a source, and its `// SAFETY:` comment
  names that source (e.g. "per OSDev Context_Switching" or "SDM Vol.3 §…").
- **R4 — Tutorials are fetched, not reconstructed.** When a milestone follows phil-opp, fetch
  the actual chapter first and follow its current text. Memory of the tutorial mixes editions.
- **R5 — No invented flags or paths.** QEMU/cargo/gdb flags verified via `--help` or docs
  before use; repo paths and symbol names confirmed via Serena/grep before being referenced.
- **R6 — Claims require evidence.** Never state "it boots" / "tests pass" without running the
  command in this session and showing the real output. If a command can't be run, say exactly
  that instead.
- **R7 — "I don't know" is a valid and required answer** when a fact can't be verified right
  now. Flag it, look it up, or leave a `// TODO(verify: …)` — never silently guess.
- **R8 — Dependency compile errors (E0599/E0425/E0433) → suspect version drift first.** Check
  the pinned version's docs before "fixing" code toward an API from a different version.
- **R9 — Don't trust your own summaries of long files.** Before editing a file discussed
  earlier, re-read the relevant region; state before editing which acceptance criterion the
  edit serves.
- **R10 — Captured knowledge beats re-derived knowledge.** A newly verified constant → add to
  REFERENCES.md (with source). A design choice → ADR in DECISIONS.md. Milestone learnings
  ("gotcha: EOI before…") → one line in the milestone's commit message. Future sessions must
  not have to re-guess.

## High-risk zones (extra care; assume your memory is wrong here)

Crate APIs across versions · GDT/IDT descriptor bit layouts · MSR numbers and WRMSR semantics
· PIC/PIT programming sequences · `syscall`/`sysret` + swapgs details · paging entry flags ·
tar/ustar header offsets · linker/target-JSON details · QEMU device flags.

## Pre-commit checklist (also run via `/check`)

1. `cargo fmt --all -- --check` clean.
2. `cargo clippy -- -D warnings` clean (allow-listed lints documented).
3. `cargo test` — full serial output pasted, exit green.
4. `cargo run` boots to the expected state for the current milestone.
5. Every new/changed `unsafe` and `asm!` has a `// SAFETY:` naming its source.
6. No new magic number entered code without a REFERENCES.md entry or inline source comment.
7. No prints/allocations/blocking locks added inside interrupt handlers.
8. Docs updated: CLAUDE.md status, MILESTONES checklist, ADRs if any.

## For the human reviewer

Spot-check one thing per session: pick any constant or API call Claude wrote and ask
"source?" — the answer must be a doc, not a shrug. That habit keeps the whole system honest.
