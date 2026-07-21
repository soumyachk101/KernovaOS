Run the full quality gate. Show the real output of every step — never summarize away a failure
(docs/VERIFICATION.md R6).

1. `cargo fmt --all -- --check`
2. `cargo clippy -- -D warnings`
3. `cargo test` — paste the serial output
4. `cargo build` and confirm the bootimage file exists
5. Scan the current diff for: `unsafe`/`asm!` without a `// SAFETY:` comment; new numeric
   constants not present in docs/REFERENCES.md §3 and without an inline source comment;
   prints/allocations/blocking locks inside interrupt handlers.

Report pass/fail per item. If anything fails, fix it and re-run before claiming done.
