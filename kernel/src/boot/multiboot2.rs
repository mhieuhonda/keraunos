//! Multiboot2 information parser.
//!
//! Walks the tag list the bootloader drops at a known address and extracts
//! everything early boot needs today: the bootloader name, the kernel
//! command line and the BIOS memory map. Parsing is read-only and uses
//! unaligned loads so it is safe on any architecture multiboot2 targets.
//!
//! Reference: <https://www.gnu.org/software/grub/manual/multiboot2/>

/// Maximum number of memory regions retained from the boot memory map.
#[allow(dead_code)] // part of the parser contract even while the kernel uses fewer
const MAX_REGIONS: usize = 256;

/// Multiboot2 magic passed in EAX at hand-off.
pub const MULTIBOOT2_BOOT_MAGIC: u32 = 0x36d76289;

const HEADER_SIZE: usize = 8; // u32 total_size + u32 reserved

const TAG_TYPE_END: u32 = 0;
const TAG_TYPE_CMDLINE: u32 = 1;
const TAG_TYPE_BOOTLOADER_NAME: u32 = 2;
const TAG_TYPE_MEMORY_MAP: u32 = 6;

/// One entry of the multiboot2 memory map.
#[derive(Debug, Clone, Copy)]
pub struct Region {
    /// Physical base address of the region.
    pub base: u64,
    /// Length in bytes.
    pub len: u64,
    /// Multiboot2 region kind.
    pub kind: RegionKind,
}

#[allow(dead_code)] // full kind set decoded by the parser
/// Multiboot2 region kinds (subset).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RegionKind {
    /// Usable RAM.
    Usable = 1,
    /// Reserved by firmware or in use by the kernel image.
    Reserved = 2,
    /// ACPI data, reclaimable after table extraction.
    AcpiReclaimable = 3,
    /// ACPI NVS memory.
    AcpiNvs = 4,
    /// BIOS or bootloader scratch area — do not touch.
    Bad = 5,
    /// Anything the parser does not name explicitly.
    Other(u32),
}

impl From<u32> for RegionKind {
    fn from(raw: u32) -> Self {
        match raw {
            1 => RegionKind::Usable,
            2 => RegionKind::Reserved,
            3 => RegionKind::AcpiReclaimable,
            4 => RegionKind::AcpiNvs,
            5 => RegionKind::Bad,
            other => RegionKind::Other(other),
        }
    }
}

/// The parsed multiboot2 information block.
pub struct Info {
    bytes: &'static [u8],
}

impl Info {
    /// Reconstruct the information block from the physical address the
    /// bootloader passed in EBX.
    ///
    /// # Safety
    /// `addr` must point at a valid multiboot2 information structure that
    /// outlives boot (identity-mapped for the whole early-boot window).
    pub unsafe fn from_addr(addr: usize) -> Info {
        // SAFETY: guaranteed by the caller.
        let total_size = unsafe { core::ptr::read_unaligned(addr as *const u32) } as usize;
        // SAFETY: guaranteed by the caller; lifetime is 'static by contract.
        let bytes = unsafe { core::slice::from_raw_parts(addr as *const u8, total_size) };
        Info { bytes }
    }

    fn tags(&self) -> TagIter<'static> {
        // `Info::bytes` is `&'static [u8]` by boot-protocol contract, so the
        // iterator (and every string it yields) can borrow for 'static.
        TagIter {
            bytes: self.bytes,
            pos: HEADER_SIZE,
        }
    }

    /// Total size of the information block, in bytes. The PMM needs the
    /// span to keep its hands off the block.
    pub fn total_size(&self) -> usize {
        self.bytes.len()
    }

    /// Human-readable bootloader name (e.g. "GRUB 2.12").
    pub fn bootloader_name(&self) -> &'static str {
        for tag in self.tags() {
            if tag.kind == TAG_TYPE_BOOTLOADER_NAME {
                return tag.cstring();
            }
        }
        "unknown bootloader"
    }

    /// Kernel command line, if the bootloader supplied one.
    pub fn cmdline(&self) -> Option<&'static str> {
        for tag in self.tags() {
            if tag.kind == TAG_TYPE_CMDLINE {
                return Some(tag.cstring());
            }
        }
        None
    }

    /// BIOS-provided memory map (fixed-capacity copy of usable + reserved
    /// entries). Returns an empty map when the bootloader omitted the tag.
    pub fn memory_map(&self) -> MemoryMap {
        for tag in self.tags() {
            if tag.kind == TAG_TYPE_MEMORY_MAP {
                return MemoryMap::from_tag(tag.bytes);
            }
        }
        MemoryMap::empty()
    }
}

/// Cursor over the tag list.
struct TagIter<'a> {
    bytes: &'a [u8],
    pos: usize,
}

/// One decoded tag.
struct Tag<'a> {
    kind: u32,
    bytes: &'a [u8],
}

impl<'a> Iterator for TagIter<'a> {
    type Item = Tag<'a>;

    fn next(&mut self) -> Option<Tag<'a>> {
        if self.pos + 8 > self.bytes.len() {
            return None;
        }
        let kind = read_u32(self.bytes, self.pos);
        let size = read_u32(self.bytes, self.pos + 4) as usize;
        if size < 8 || self.pos + size > self.bytes.len() {
            return None; // corrupt tag list; stop walking
        }
        let tag = Tag {
            kind,
            bytes: &self.bytes[self.pos..self.pos + size],
        };
        // Tags are 8-byte aligned.
        self.pos = (self.pos + size + 7) & !7;
        if kind == TAG_TYPE_END {
            return None;
        }
        Some(tag)
    }
}

impl<'a> Tag<'a> {
    /// The tag payload interpreted as a NUL-terminated string. The returned
    /// borrow is tied to the tag's payload (which is boot-lifetime data).
    fn cstring(&self) -> &'a str {
        let payload = &self.bytes[8..];
        let end = payload
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(payload.len());
        let bytes = &payload[..end];
        // SAFETY: boot loaders pass ASCII/UTF-8; fall back to lossy slicing
        // only when the payload is valid UTF-8, else "invalid".
        match core::str::from_utf8(bytes) {
            Ok(s) => s,
            Err(_) => "invalid",
        }
    }
}

/// Fixed-capacity copy of the boot memory map.
pub struct MemoryMap {
    regions: [Region; MAX_REGIONS],
    count: usize,
}

impl MemoryMap {
    fn empty() -> Self {
        MemoryMap {
            regions: [Region {
                base: 0,
                len: 0,
                kind: RegionKind::Other(0),
            }; MAX_REGIONS],
            count: 0,
        }
    }

    fn from_tag(tag: &[u8]) -> Self {
        let mut map = MemoryMap::empty();
        // Memory-map tag payload: u32 entry_size, u32 entry_version, entries...
        if tag.len() < 16 {
            return map;
        }
        let entry_size = read_u32(tag, 8) as usize;
        if entry_size < 24 {
            return map;
        }
        let mut pos = 16;
        while pos + entry_size <= tag.len() && map.count < MAX_REGIONS {
            let base = read_u64(tag, pos);
            let len = read_u64(tag, pos + 8);
            let kind = read_u32(tag, pos + 16);
            map.regions[map.count] = Region {
                base,
                len,
                kind: RegionKind::from(kind),
            };
            map.count += 1;
            pos += entry_size;
        }
        map
    }

    /// The regions retained from the boot memory map.
    pub fn regions(&self) -> &[Region] {
        &self.regions[..self.count]
    }

    /// Total bytes of `Usable` RAM reported by the bootloader.
    pub fn usable_bytes(&self) -> u64 {
        self.regions()
            .iter()
            .filter(|r| r.kind == RegionKind::Usable)
            .map(|r| r.len)
            .sum()
    }
}

fn read_u32(bytes: &[u8], pos: usize) -> u32 {
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&bytes[pos..pos + 4]);
    u32::from_le_bytes(buf)
}

fn read_u64(bytes: &[u8], pos: usize) -> u64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&bytes[pos..pos + 8]);
    u64::from_le_bytes(buf)
}
