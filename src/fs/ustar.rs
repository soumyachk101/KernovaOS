//! Minimal read-only ustar (tar) parser. Header layout per REFERENCES.md §3.

const BLOCK: usize = 512;
const NAME_OFF: usize = 0;
const NAME_LEN: usize = 100;
const SIZE_OFF: usize = 124;
const SIZE_LEN: usize = 12;
const TYPE_OFF: usize = 156;
const MAGIC_OFF: usize = 257;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry<'a> {
    pub name: &'a str,
    pub data: &'a [u8],
    pub is_dir: bool,
}

/// Iterator over the regular files/dirs in a ustar archive.
pub struct Archive<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Archive<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Archive { bytes, offset: 0 }
    }

    /// Look up an entry by exact name.
    pub fn find(&self, path: &str) -> Option<Entry<'a>> {
        Archive::new(self.bytes).find_map(|e| if e.name == path { Some(e) } else { None })
    }
}

fn parse_octal(field: &[u8]) -> u64 {
    let mut n = 0u64;
    for &b in field {
        match b {
            b'0'..=b'9' => n = n * 8 + (b - b'0') as u64,
            _ => break, // stop at NUL, space, or any padding
        }
    }
    n
}

fn cstr(field: &[u8]) -> &str {
    let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    core::str::from_utf8(&field[..end]).unwrap_or("")
}

impl<'a> Iterator for Archive<'a> {
    type Item = Entry<'a>;

    fn next(&mut self) -> Option<Entry<'a>> {
        loop {
            let header = self.bytes.get(self.offset..self.offset + BLOCK)?;

            // two consecutive zero blocks end the archive; a single all-zero
            // header is the terminator we care about in practice
            if header.iter().all(|&b| b == 0) {
                return None;
            }

            // validate ustar magic; bail if it's garbage
            let magic = &header[MAGIC_OFF..MAGIC_OFF + 5];
            if magic != b"ustar" {
                return None;
            }

            let name = cstr(&header[NAME_OFF..NAME_OFF + NAME_LEN]);
            let size = parse_octal(&header[SIZE_OFF..SIZE_OFF + SIZE_LEN]) as usize;
            let typeflag = header[TYPE_OFF];
            let is_dir = typeflag == b'5';

            let data_start = self.offset + BLOCK;
            let data = self
                .bytes
                .get(data_start..data_start + size)
                .unwrap_or(&[]);

            // advance past header + data rounded up to a block boundary
            let data_blocks = size.div_ceil(BLOCK);
            self.offset = data_start + data_blocks * BLOCK;

            // skip directory entries for the file iterator's consumers, but
            // still report them (list() wants them); here just return
            return Some(Entry { name, data, is_dir });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Build a one-file ustar archive in a fixed buffer for unit testing.
    fn make(name: &str, data: &[u8], buf: &mut [u8; 1024]) {
        buf.iter_mut().for_each(|b| *b = 0);
        buf[..name.len()].copy_from_slice(name.as_bytes());
        // octal size at 124
        let mut size = data.len();
        let mut i = SIZE_OFF + 10; // write 11 octal digits, NUL at +11
        for _ in 0..11 {
            buf[i] = b'0' + (size % 8) as u8;
            size /= 8;
            i -= 1;
        }
        buf[TYPE_OFF] = b'0';
        buf[MAGIC_OFF..MAGIC_OFF + 5].copy_from_slice(b"ustar");
        buf[BLOCK..BLOCK + data.len()].copy_from_slice(data);
    }

    #[test_case]
    fn parses_single_file() {
        let mut buf = [0u8; 1024];
        make("greet.txt", b"hi", &mut buf);
        let archive = Archive::new(&buf);
        let e = archive.find("greet.txt").expect("entry not found");
        assert_eq!(e.name, "greet.txt");
        assert_eq!(e.data, b"hi");
        assert!(!e.is_dir);
    }

    #[test_case]
    fn octal_size_parsing() {
        // 512 decimal = 1000 octal
        assert_eq!(parse_octal(b"00000001000\0"), 512);
        assert_eq!(parse_octal(b"0"), 0);
    }

    #[test_case]
    fn missing_returns_none() {
        let mut buf = [0u8; 1024];
        make("a", b"x", &mut buf);
        assert!(Archive::new(&buf).find("nope").is_none());
    }
}
