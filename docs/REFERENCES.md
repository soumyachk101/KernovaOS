# REFERENCES — canonical sources & ground-truth constants

Rule (from VERIFICATION.md R2/R10): constants used in code come **only** from the table below
or from a freshly fetched source listed here. When you verify a new fact, add it to the table
with its source — this file is the repo's accumulated ground truth.

## 1. Primary source per milestone (fetch the page, don't recall it)

| Milestone | Source |
|---|---|
| M1 | https://os.phil-opp.com/freestanding-rust-binary/ · https://os.phil-opp.com/minimal-rust-kernel/ |
| M2 | https://os.phil-opp.com/vga-text-mode/ |
| M3 | https://os.phil-opp.com/testing/ |
| M4 | https://os.phil-opp.com/cpu-exceptions/ |
| M5 | https://os.phil-opp.com/double-fault-exceptions/ |
| M6 | https://os.phil-opp.com/hardware-interrupts/ |
| M7 | https://os.phil-opp.com/paging-introduction/ · https://os.phil-opp.com/paging-implementation/ |
| M8 | https://os.phil-opp.com/heap-allocation/ · https://os.phil-opp.com/allocator-designs/ |
| M9 | https://os.phil-opp.com/async-await/ |
| M10 | OSDev Wiki: "Context Switching", "Kernel Multitasking" · xv6 source (`proc.c`, `swtch.S`) |
| M11 | OSDev Wiki: "Getting to Ring 3", "System Calls", "SYSCALL/SYSRET" |
| M12 | OSDev Wiki: "USTAR" · GNU tar "Basic Tar Format" spec |
| M13 | builds on all above |
| M14 | OSDev Wiki: "ATA PIO Mode", "AHCI", "FAT", e1000/8254x pages |

Reference code: phil-opp's repo https://github.com/phil-opp/blog_os (per-chapter branches —
also the source of truth for the target JSON `data-layout` string).

## 2. Standing references

| What | Where |
|---|---|
| OS theory | OSTEP — https://pages.cs.wisc.edu/~remzi/OSTEP/ |
| OS-dev how-tos | https://wiki.osdev.org (start: /Bare_Bones, /Expanded_Main_Page) |
| Small real Unix to read | xv6 / MIT 6.1810 — https://pdos.csail.mit.edu/6.1810/ |
| CPU ground truth | Intel SDM — https://www.intel.com/sdm (Vol. 3 for system programming) |
| Crate docs (always with version!) | https://docs.rs/<crate>/<pinned-version> — e.g. docs.rs/x86_64, docs.rs/bootloader — or via Context7 |
| QEMU | https://www.qemu.org/docs/master/ |
| Claude Code | https://code.claude.com/docs |

## 3. Verified ground-truth constants

Legend: src = where it was verified. These are stable architectural facts; anything not here
must be looked up before use.

### I/O ports
| Thing | Value | src |
|---|---|---|
| VGA text buffer (physical) | `0xB8000` | OSDev "Text UI" / phil-opp M2 |
| COM1 serial base | `0x3F8` (COM2 `0x2F8`) | OSDev "Serial Ports" |
| PIC1 command / data | `0x20` / `0x21` | OSDev "8259 PIC" |
| PIC2 command / data | `0xA0` / `0xA1` | OSDev "8259 PIC" |
| PIC end-of-interrupt command byte | `0x20` | OSDev "8259 PIC" |
| PS/2 data / status-command | `0x60` / `0x64` | OSDev "8042 PS/2 Controller" |
| PIT channel-0 data / mode-command | `0x40` / `0x43`; base clock 1 193 182 Hz | OSDev "PIT" |
| QEMU `isa-debug-exit` (as we configure it) | `0xF4`, iosize 4 | phil-opp M3 / our Cargo.toml |

### Interrupt vectors (our configuration)
| Thing | Value | src |
|---|---|---|
| PIC remap offsets | master → 32, slave → 40 | our ADR/phil-opp M6 |
| Timer (IRQ0) / Keyboard (IRQ1) vectors | 32 / 33 | follows from remap |
| Breakpoint `#BP` | vector 3 | SDM |
| Double fault `#DF` | vector 8 — **error code always 0**, does not `iret`-return | SDM / phil-opp M5 |
| General protection `#GP` | vector 13 | SDM |
| Page fault `#PF` | vector 14 — faulting address in **CR2** | SDM |
| Our syscall gate | `int 0x80` = vector 128, gate DPL 3 | our ADR-009 |

### Registers / MSRs (x86-64)
| Thing | Value | src |
|---|---|---|
| CR3 | physical address of active PML4 (+ flags) | SDM |
| IA32_EFER | MSR `0xC000_0080` (SCE bit 0 enables syscall/sysret) | SDM |
| IA32_STAR / LSTAR / FMASK | MSR `0xC000_0081` / `0xC000_0082` / `0xC000_0084` | SDM |
| FS.base / GS.base / KernelGSbase | MSR `0xC000_0100` / `0xC000_0101` / `0xC000_0102` | SDM |

### Paging & memory (x86-64, our setup)
| Thing | Value | src |
|---|---|---|
| Page size / levels | 4 KiB pages, 4-level (PML4→PDPT→PD→PT), 512 × 8-byte entries per table | SDM / phil-opp M7 |
| Huge pages | 2 MiB (PD level), 1 GiB (PDPT level) | SDM |
| Kernel heap start / min size | `0x_4444_4444_0000` / 100 KiB | our choice, ARCHITECTURE.md |
| Test success host exit code | 33 = `(0x10 << 1) \| 1` | phil-opp M3 / our Cargo.toml |

### ustar (tar) header — 512-byte blocks
| Field | Offset / len | Notes | src |
|---|---|---|---|
| name | 0 / 100 | NUL-padded | GNU tar spec / OSDev "USTAR" |
| mode | 100 / 8 | octal ASCII | same |
| size | 124 / 12 | **octal ASCII** | same |
| typeflag | 156 / 1 | '0'/NUL file, '5' dir | same |
| magic | 257 / 6 | `"ustar"` | same |
| data | follows header | padded to 512-byte boundary | same |

## 4. Pinned crate versions (fill as each lands, then treat as ground truth)

| Crate | Pinned | docs link |
|---|---|---|
| bootloader | _tbd at M1_ | docs.rs/bootloader/<ver> |
| x86_64 | _tbd at M3/M4_ | docs.rs/x86_64/<ver> |
| … | | |

Keep this table current — it's what makes rule R1 checkable.
