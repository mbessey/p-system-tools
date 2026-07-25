use p_system_format::bytes::{read_array, read_u16_le, read_u32_le};
use p_system_format::error::FormatError;

const SIZE: usize = 512;

#[derive(Debug, Clone, Copy)]
pub struct CodeInfo {
    pub address: u16,
    pub length: u16,
}

#[derive(Debug, Clone, Copy)]
pub enum SegmentKind {
    Linked,            // A ready-to-run program
    HostSegment,       // The outer block of a Pascal program, if it has unresolved references
    SegmentProcedure,  // Not used.
    UnitSegment,       // A Unit, ready to be linked
    SeparateSegment,   // Native-code segment
    UnlinkedIntrinsic, // An Intrinsic unit with unresolved references
    LinkedIntrinsic,   // An Intrinsic unit
    DataSegment,       // Data segment - data stored on the stack, used for some intrinsics
}

impl TryFrom<u16> for SegmentKind {
    type Error = FormatError;

    fn try_from(value: u16) -> Result<Self, FormatError> {
        Ok(match value {
            0 => SegmentKind::Linked,
            1 => SegmentKind::HostSegment,
            2 => SegmentKind::SegmentProcedure,
            3 => SegmentKind::UnitSegment,
            4 => SegmentKind::SeparateSegment,
            5 => SegmentKind::UnlinkedIntrinsic,
            6 => SegmentKind::LinkedIntrinsic,
            7 => SegmentKind::DataSegment,
            _ => {
                return Err(FormatError::InvalidValue {
                    field: "seg_kind",
                    value: value as u32,
                });
            }
        })
    }
}

// intrinsic_segments/library_info mirror the on-disk format but aren't
// consumed by any command yet.
#[allow(dead_code)]
#[derive(Debug)]
pub struct SegmentDictionary {
    pub code_info: [CodeInfo; 16], // one for each of 16 segments
    pub seg_name: [[u8; 8]; 16],   // 8 charcters, space-padded
    seg_kind: [u16; 16],           // raw; decode on demand via segment_kind()
    pub text_addr: [u16; 16],      // For Units, this points to the Interface section
    pub seg_info: [u16; 16],       // A bitfield for each segment
    pub intrinsic_segments: u32,   // One bit for each segment in System.Library
    // This is "library information", which is described by the Apple Pascal manual thus:
    // Library information of undefined format occupies most of the remainder of the segment dictionary block.
    // That's...great. I guess we'll figure that out when/if it comes up
    pub library_info: [u8; 140],
    pub copyright_string: [u8; 80], // Copyright, as set by (*$C *), seems to be zero-terminated
}

impl SegmentDictionary {
    pub fn parse(bytes: &[u8]) -> Result<Self, FormatError> {
        if bytes.len() < SIZE {
            return Err(FormatError::Truncated {
                needed: SIZE,
                available: bytes.len(),
            });
        }

        let mut code_info = [CodeInfo {
            address: 0,
            length: 0,
        }; 16];
        for (i, slot) in code_info.iter_mut().enumerate() {
            let base = i * 4;
            slot.address = read_u16_le(bytes, base).expect("size checked above");
            slot.length = read_u16_le(bytes, base + 2).expect("size checked above");
        }

        let mut seg_name = [[0u8; 8]; 16];
        for (i, slot) in seg_name.iter_mut().enumerate() {
            *slot = read_array::<8>(bytes, 64 + i * 8).expect("size checked above");
        }

        let mut seg_kind = [0u16; 16];
        for (i, slot) in seg_kind.iter_mut().enumerate() {
            *slot = read_u16_le(bytes, 192 + i * 2).expect("size checked above");
        }

        let mut text_addr = [0u16; 16];
        for (i, slot) in text_addr.iter_mut().enumerate() {
            *slot = read_u16_le(bytes, 224 + i * 2).expect("size checked above");
        }

        let mut seg_info = [0u16; 16];
        for (i, slot) in seg_info.iter_mut().enumerate() {
            *slot = read_u16_le(bytes, 256 + i * 2).expect("size checked above");
        }

        let intrinsic_segments = read_u32_le(bytes, 288).expect("size checked above");
        let library_info = read_array::<140>(bytes, 292).expect("size checked above");
        let copyright_string = read_array::<80>(bytes, 432).expect("size checked above");

        Ok(Self {
            code_info,
            seg_name,
            seg_kind,
            text_addr,
            seg_info,
            intrinsic_segments,
            library_info,
            copyright_string,
        })
    }

    pub fn segment_kind(&self, slot: usize) -> Result<SegmentKind, FormatError> {
        SegmentKind::try_from(self.seg_kind[slot])
    }

    pub fn active_segments(&self) -> impl Iterator<Item = (usize, CodeInfo, String)> + '_ {
        (0..16).filter_map(move |s| {
            let code_info = self.code_info[s];
            if code_info.address == 0 {
                return None;
            }
            Some((
                s,
                code_info,
                p_system_format::pascal_string::from_space_padded(&self.seg_name[s]),
            ))
        })
    }
}

pub fn string_from_segment_info(segment_info: u16) -> String {
    let unit = segment_info & 0xff;
    let code_type = (segment_info & 0x0f00) >> 8;
    let type_s = match code_type {
        0 => "Unknown",
        1 => "Pcode Big-endian",
        2 => "Pcode Little-endian",
        _ => "Native code",
    };
    let version = (segment_info & 0xe000) >> 13;
    format!("[unit: {}, type: {}, version: {}]", unit, type_s, version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_kind_valid_range() {
        assert!(matches!(SegmentKind::try_from(0), Ok(SegmentKind::Linked)));
        assert!(matches!(
            SegmentKind::try_from(7),
            Ok(SegmentKind::DataSegment)
        ));
    }

    #[test]
    fn segment_kind_rejects_out_of_range() {
        assert!(SegmentKind::try_from(8).is_err());
        assert!(SegmentKind::try_from(u16::MAX).is_err());
    }

    #[test]
    fn parse_rejects_truncated_input() {
        assert!(SegmentDictionary::parse(&[]).is_err());
        assert!(SegmentDictionary::parse(&[0u8; 511]).is_err());
    }

    #[test]
    fn parse_accepts_minimum_size() {
        assert!(SegmentDictionary::parse(&[0u8; 512]).is_ok());
    }
}
