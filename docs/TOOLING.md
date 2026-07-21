# TOOLING — MCP servers & tools Claude Code must use

Purpose: replace guessing with lookup. Claude Code hallucinates most when it writes crate APIs
or hardware constants from stale training memory. The tools below give it live, verifiable
sources. The protocol for *when verification is mandatory* is `docs/VERIFICATION.md`;
this file covers *what tools exist and how to route work to them*.

## 1. The two MCP servers (configured in `.mcp.json`, committed to the repo)

| Server | What it is | Why this project needs it |
|---|---|---|
| **Serena** (oraios/serena) | Language-server-backed semantic code toolkit: find symbol, find references, symbol-level insert/replace, project memory. Free & open source. | The kernel grows to dozens of interlinked modules (gdt ↔ interrupts ↔ memory ↔ sched). Symbol-level navigation/edits beat regex-grepping, save tokens, and avoid "edited the wrong lookalike function" errors. |
| **Context7** (upstash/context7) | Serves fresh, **version-specific** library documentation into context (`resolve-library-id` → `get-library-docs`). Free tier works without a key. | Our crates (`x86_64`, `bootloader`, `pc-keyboard`, …) changed APIs across versions and we pin *old* versions. Training memory mixes versions — Context7 answers "what does THIS version's API look like", which kills invented-API bugs. |

## 2. Setup

Prerequisites (one-time, on the host):

```bash
# Node.js (for npx / Context7) — any recent LTS
# uv (for uvx / Serena):
curl -LsSf https://astral.sh/uv/install.sh | sh
# Rust language server (Serena uses it for this repo):
rustup component add rust-analyzer
```

Option A — nothing else needed: `.mcp.json` is already in the repo root. On first launch in
this project, Claude Code detects it and asks for approval — approve both servers.

Option B — CLI (writes the same project-scoped config):

```bash
claude mcp add --scope project serena -- uvx --from git+https://github.com/oraios/serena \
  serena start-mcp-server --context ide-assistant --project "$(pwd)"
claude mcp add --scope project context7 -- npx -y @upstash/context7-mcp
```

Verify: `claude mcp list` in the terminal, or `/mcp` inside a session — both servers should
show as connected with tools listed.

Notes:
- Serena's first run does a short onboarding and creates a `.serena/` folder
  (`project.yml` is fine to commit). If a newer Serena warns that the `ide-assistant` context
  is deprecated, switch the arg to `--context claude-code`.
- Context7 without an API key uses the free tier; if rate-limited, get a key at
  context7.com/dashboard and append `--api-key YOUR_KEY` to its args.
- If Serena doesn't auto-activate the project (because `.mcp.json` uses `"."`), re-add it via
  Option B with the absolute `$(pwd)` path.

## 3. Task → tool routing (Claude Code: follow this table)

| Task | Use | Not |
|---|---|---|
| "What's the signature / how do I use X from `x86_64` / `bootloader` / any dep?" | **Context7** (resolve id → get docs for the *pinned* version); fallback `WebFetch` of `docs.rs/<crate>/<version>` | memory |
| Find where a symbol is defined / all its callers; rename; insert a method into a type | **Serena** (`find_symbol`, `find_referencing_symbols`, symbol-level edits) | blind regex edits |
| Does function/const/path Y exist in this repo? | **Serena** / grep — confirm before referencing | assuming it exists |
| Hardware fact: port, MSR, bit layout, vector, header offset | `docs/REFERENCES.md` ground-truth table first; else **WebFetch** OSDev/Intel SDM (URLs in REFERENCES.md) | memory — a wrong constant = triple fault |
| Implementing a milestone that follows phil-opp | **WebFetch the actual chapter** (URL map in REFERENCES.md), then code | reconstructing the tutorial from memory |
| Build / run / test / debug | **Bash**: `cargo …`, QEMU, GDB per DEVELOPMENT.md | claiming results without running |
| Anything ambiguous about Claude Code itself | WebFetch https://code.claude.com/docs | guessing product behavior |

## 4. Slash commands shipped in `.claude/commands/`

| Command | Does |
|---|---|
| `/milestone` | Loads current milestone from CLAUDE.md + MILESTONES.md, fetches its primary reference, implements with verification, updates status when done |
| `/check` | Full quality gate: fmt → clippy → `cargo test` (real serial output) → boot check → SAFETY/constants scan |

## 5. MCP vs "Skills" (naming clarity)

- **MCP servers** = external tools with live capabilities (Serena, Context7). Configured in
  `.mcp.json`. This is what this file is about.
- **Claude Code Agent Skills** = folders of instructions (`SKILL.md`) Claude loads on demand.
  Our `docs/` set already plays that role for this repo; converting docs into formal skills is
  optional later polish, not needed now.

## 6. Keep the tool list lean

Every extra MCP server costs context and attention. Add more (e.g. a GitHub MCP for PR review,
a memory server) **only** when a concrete recurring need appears — and record it here plus an
ADR in `docs/DECISIONS.md`. Two well-used tools beat eight idle ones.
