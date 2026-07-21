Work the current milestone properly.

1. Read CLAUDE.md "Current status" and docs/MILESTONES.md; identify the active milestone and
   restate its acceptance criteria.
2. Fetch the milestone's primary source from docs/REFERENCES.md §1 (WebFetch the actual page —
   do not work from memory; docs/VERIFICATION.md R4).
3. Implement in small steps. Between steps, verify with `cargo run` / `cargo test` and show
   real output. Follow docs/VERIFICATION.md for every crate API (Context7) and every hardware
   constant (REFERENCES.md table), and docs/TOOLING.md for tool routing.
4. When every acceptance criterion passes with evidence shown: update CLAUDE.md "Current
   status", tick the milestone in docs/MILESTONES.md, add any ADRs to docs/DECISIONS.md and
   any newly verified constants to docs/REFERENCES.md, then propose the commit message
   (`M<n>: …`) and tag.

Extra instructions for this run: $ARGUMENTS
