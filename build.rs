//! Packs `initrd/` into a ustar archive at build time (ADR-008). The kernel
//! embeds the result with `include_bytes!(concat!(env!("OUT_DIR"), ...))`.

use std::path::Path;
use std::process::Command;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let archive = Path::new(&out_dir).join("initrd.tar");

    // Rebuild whenever any initrd file changes.
    println!("cargo:rerun-if-changed=initrd");

    let initrd = Path::new("initrd");
    let mut files: Vec<String> = std::fs::read_dir(initrd)
        .expect("initrd/ directory missing")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    files.sort(); // deterministic archive order

    // BSD/GNU tar both take --format ustar; -C keeps paths flat (no initrd/ prefix)
    let status = Command::new("tar")
        .arg("--format")
        .arg("ustar")
        .arg("-cf")
        .arg(&archive)
        .arg("-C")
        .arg("initrd")
        .args(&files)
        .status()
        .expect("failed to run tar");
    assert!(status.success(), "tar failed to build initrd archive");
}
