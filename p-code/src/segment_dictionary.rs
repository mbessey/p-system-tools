#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CodeInfo {
    pub address: u16,
    pub length: u16,
}

#[repr(u16)]
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum SegmentKind {
    Linked,             // A ready-to-run program
    HostSegment,        // The outer block of a Pascal program, if it has unresolved references
    SegmentProcedure,   // Not used.
    UnitSegment,        // A Unit, ready to be linked
    SeparateSegment,    // Native-code segment
    UnlinkedIntrinsic,  // An Intrinsic unit with unresolved references
    LinkedIntrinsic,    // An Intrinsic unit
    DataSegment         // Data segment - data stored on the stack, used for some intrinsics
}

#[derive(Debug)]
#[repr(C)]
pub struct SegmentDictionary {
    pub code_info: [ CodeInfo; 16],     // one for each of 16 segments
    pub seg_name: [[u8; 8]; 16],        // 8 charcters, space-padded
    pub seg_kind: [SegmentKind; 16],    // one for each of 16 segments
    pub text_addr: [u16; 16],           // For Units, this points to the Interface section
    pub seg_info: [u16; 16],            // A bitfield for each segment
    pub intrinsic_segments: u32,        // One bit for each segment in System.Library
    // This is "library information", which is described by the Apple Pascal manual thus:
    // Library information of undefined format occupies most of the remainder of the segment dictionary block.
    // That's...great. I guess we'll figure that out when/if it comes up
    pub library_info: [u8; 140],
    pub copyright_string: [u8; 80],     // Copyright, as set by (*$C *), seems to be zero-terminated
}

impl SegmentDictionary {
    pub fn new(bytes: &[u8]) -> Self {
        let directory_ptr = bytes.as_ptr() as *const SegmentDictionary;
        unsafe { directory_ptr.read_unaligned() }
    }

    pub fn active_segments(&self) -> impl Iterator<Item = (usize, CodeInfo, String)> + '_ {
        (0..16).filter_map(move |s| {
            let code_info = self.code_info[s];
            if code_info.address == 0 {
                return None;
            }
            Some((s, code_info, p_system_format::pascal_string::from_space_padded(&self.seg_name[s])))
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
        _ => "Native code"
    };
    let version = (segment_info & 0xe000) >> 13;
    format!("[unit: {}, type: {}, version: {}]", unit, type_s, version)
}
