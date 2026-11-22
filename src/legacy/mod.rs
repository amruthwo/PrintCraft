//! Readers for older file formats: saved projects (.sig and siblings) and
//! .CDT art/photo collections. See docs/FORMATS.md for the layouts.

pub mod collection;
pub mod sig;
pub mod wmf;

pub(crate) fn le16(d: &[u8], at: usize) -> Option<u16> {
    d.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]]))
}

pub(crate) fn le32(d: &[u8], at: usize) -> Option<u32> {
    d.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

pub(crate) fn lei32(d: &[u8], at: usize) -> Option<i32> {
    le32(d, at).map(|v| v as i32)
}
