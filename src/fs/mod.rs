//! Read-only initrd filesystem (M12, ADR-008): a ustar archive embedded at
//! build time, exposed behind a tiny VFS.

pub mod ustar;

use alloc::vec::Vec;
use ustar::Archive;

/// The initrd image, packed from `initrd/` by build.rs.
static INITRD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/initrd.tar"));

/// Minimal read-only filesystem interface (ARCHITECTURE §8).
pub trait Vfs {
    /// Read the full contents of a file, or `None` if absent.
    fn read(&self, path: &str) -> Option<&[u8]>;
    /// List file names in the archive.
    fn list(&self) -> Vec<&str>;
    /// True if the file exists.
    fn exists(&self, path: &str) -> bool {
        self.read(path).is_some()
    }
}

pub struct Initrd {
    bytes: &'static [u8],
}

impl Initrd {
    pub fn new() -> Self {
        Initrd { bytes: INITRD }
    }
}

impl Default for Initrd {
    fn default() -> Self {
        Self::new()
    }
}

impl Vfs for Initrd {
    fn read(&self, path: &str) -> Option<&[u8]> {
        Archive::new(self.bytes)
            .find(path)
            .filter(|e| !e.is_dir)
            .map(|e| e.data)
    }

    fn list(&self) -> Vec<&str> {
        Archive::new(self.bytes)
            .filter(|e| !e.is_dir)
            .map(|e| e.name)
            .collect()
    }
}
