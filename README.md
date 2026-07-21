# KernovaOS

**Kernova** — a kernel born like a new star. An x86-64 operating system kernel written from scratch in Rust (`no_std`).

## Overview

Kernova is an x86-64 bare-metal operating system kernel built using Rust. It boots via the `bootloader` crate and is designed to run in QEMU.

## Tech Stack

- **Language:** Rust (`no_std`)
- **Target Architecture:** x86-64 (`x86_64-kernova.json`)
- **Boot Machine / Virtualization:** QEMU

## Prerequisites

- Rust nightly toolchain
- `bootloader` tools
- QEMU (`qemu-system-x86_64`)

## Building and Running

To build and run the kernel in QEMU:

```bash
cargo run
```

To run unit and integration tests:

```bash
cargo test
```
