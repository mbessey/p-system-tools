// Bounds-checked little-endian byte readers, shared by both crates' on-disk
// format parsers so neither needs to reach for an unsafe struct-overlay cast.

pub fn read_u16_le(bytes: &[u8], offset: usize) -> Option<u16> {
    let b = bytes.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([b[0], b[1]]))
}

pub fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    let b = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

pub fn read_array<const N: usize>(bytes: &[u8], offset: usize) -> Option<[u8; N]> {
    bytes.get(offset..offset + N)?.try_into().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_u16_le_round_trip() {
        assert_eq!(read_u16_le(&[0x34, 0x12], 0), Some(0x1234));
        assert_eq!(read_u16_le(&[0xff, 0x00, 0x34, 0x12], 2), Some(0x1234));
    }

    #[test]
    fn read_u16_le_truncated() {
        assert_eq!(read_u16_le(&[0x12], 0), None);
        assert_eq!(read_u16_le(&[0x12, 0x34], 1), None);
        assert_eq!(read_u16_le(&[], 0), None);
    }

    #[test]
    fn read_u32_le_round_trip() {
        assert_eq!(read_u32_le(&[0x78, 0x56, 0x34, 0x12], 0), Some(0x12345678));
    }

    #[test]
    fn read_u32_le_truncated() {
        assert_eq!(read_u32_le(&[0x12, 0x34, 0x56], 0), None);
    }

    #[test]
    fn read_array_round_trip() {
        assert_eq!(read_array::<3>(&[1, 2, 3, 4, 5], 1), Some([2, 3, 4]));
    }

    #[test]
    fn read_array_truncated() {
        assert_eq!(read_array::<3>(&[1, 2], 0), None);
    }
}
