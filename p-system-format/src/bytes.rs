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

pub fn write_u16_le(bytes: &mut [u8], offset: usize, value: u16) -> Option<()> {
    let b = bytes.get_mut(offset..offset + 2)?;
    b.copy_from_slice(&value.to_le_bytes());
    Some(())
}

pub fn write_u32_le(bytes: &mut [u8], offset: usize, value: u32) -> Option<()> {
    let b = bytes.get_mut(offset..offset + 4)?;
    b.copy_from_slice(&value.to_le_bytes());
    Some(())
}

pub fn write_array<const N: usize>(bytes: &mut [u8], offset: usize, value: &[u8; N]) -> Option<()> {
    let b = bytes.get_mut(offset..offset + N)?;
    b.copy_from_slice(value);
    Some(())
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

    #[test]
    fn write_u16_le_round_trip() {
        let mut buf = [0u8; 4];
        assert_eq!(write_u16_le(&mut buf, 1, 0x1234), Some(()));
        assert_eq!(read_u16_le(&buf, 1), Some(0x1234));
    }

    #[test]
    fn write_u16_le_truncated() {
        let mut buf = [0u8; 1];
        assert_eq!(write_u16_le(&mut buf, 0, 0x1234), None);
    }

    #[test]
    fn write_u32_le_round_trip() {
        let mut buf = [0u8; 4];
        assert_eq!(write_u32_le(&mut buf, 0, 0x12345678), Some(()));
        assert_eq!(read_u32_le(&buf, 0), Some(0x12345678));
    }

    #[test]
    fn write_array_round_trip() {
        let mut buf = [0u8; 5];
        assert_eq!(write_array(&mut buf, 1, &[2, 3, 4]), Some(()));
        assert_eq!(read_array::<3>(&buf, 1), Some([2, 3, 4]));
    }
}
