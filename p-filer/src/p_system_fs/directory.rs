use crate::disk_image::DiskImage;
use p_system_format::bytes::{read_array, read_u16_le};
use p_system_format::error::FormatError;

const VOLUME_INFO_SIZE: usize = 26;
const DIRECTORY_ENTRY_SIZE: usize = 26;
const NUM_ENTRIES: usize = 77;
const DIRECTORY_SIZE: usize = VOLUME_INFO_SIZE + NUM_ENTRIES * DIRECTORY_ENTRY_SIZE;

// Directory entries are each 26 bytes. The first is a bit special, and contains information about the volume itself.
// The rest are the files on the volume. Directory entries occupy blocks 2 through 5 on the disk.
#[derive(Debug)]
pub struct Directory {
    pub(crate) volume: VolumeInfo,
    pub(crate) entries: [DirectoryEntry; NUM_ENTRIES],
}

impl Directory {
    pub fn parse(disk: &impl DiskImage) -> Result<Self, FormatError> {
        let bytes = disk.read_blocks(2, 4);
        if bytes.len() < DIRECTORY_SIZE {
            return Err(FormatError::Truncated {
                needed: DIRECTORY_SIZE,
                available: bytes.len(),
            });
        }

        let volume = VolumeInfo::from_bytes(bytes);
        let entries = std::array::from_fn(|i| {
            let offset = VOLUME_INFO_SIZE + i * DIRECTORY_ENTRY_SIZE;
            DirectoryEntry::from_bytes(&bytes[offset..offset + DIRECTORY_ENTRY_SIZE])
        });

        Ok(Self { volume, entries })
    }
}

#[derive(Debug)]
pub struct VolumeInfo {
    pub(crate) first_system_block: u16,          // always zero
    pub(crate) first_block_after_directory: u16, // always 6
    pub(crate) file_type: u16,                   // always zero
    pub(crate) volume_name: [u8; 8],             // Pascal string - length is first byte
    pub(crate) num_blocks: u16,                  // number of blocks in volume
    pub(crate) num_files: u16,                   // number of files in directory
    pub(crate) last_access_time: u16,            // last access time - always zero?
    pub(crate) date: u16,                        // date set by user
    pub(crate) reserved: [u8; 4],                // reserved for future use
}

impl VolumeInfo {
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            first_system_block: read_u16_le(bytes, 0).expect("size checked above"),
            first_block_after_directory: read_u16_le(bytes, 2).expect("size checked above"),
            file_type: read_u16_le(bytes, 4).expect("size checked above"),
            volume_name: read_array::<8>(bytes, 6).expect("size checked above"),
            num_blocks: read_u16_le(bytes, 14).expect("size checked above"),
            num_files: read_u16_le(bytes, 16).expect("size checked above"),
            last_access_time: read_u16_le(bytes, 18).expect("size checked above"),
            date: read_u16_le(bytes, 20).expect("size checked above"),
            reserved: read_array::<4>(bytes, 22).expect("size checked above"),
        }
    }
}

#[derive(Debug)]
pub struct DirectoryEntry {
    pub(crate) first_block: u16,         // first block of file
    pub(crate) first_after_block: u16,   // first block after file (last block + 1)
    pub(crate) file_type: u16,           // type of file ()
    pub(crate) name: [u8; 16],           // Pascal string - length is first byte
    pub(crate) bytes_in_last_block: u16, // number of bytes in last block
    pub(crate) date: u16,                // modified date
}

impl DirectoryEntry {
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            first_block: read_u16_le(bytes, 0).expect("size checked above"),
            first_after_block: read_u16_le(bytes, 2).expect("size checked above"),
            file_type: read_u16_le(bytes, 4).expect("size checked above"),
            name: read_array::<16>(bytes, 6).expect("size checked above"),
            bytes_in_last_block: read_u16_le(bytes, 22).expect("size checked above"),
            date: read_u16_le(bytes, 24).expect("size checked above"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apple_disk::AppleDisk;
    use p_system_format::pascal_string::from_length_prefixed;
    use p_system_format::pdate::pdate_to_string;

    fn fixture_path(name: &str) -> String {
        format!("{}/../tests/AppleDsks/{name}", env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn empty_disk_directory() {
        let disk = AppleDisk::from_file(&fixture_path("empty.dsk"), false).unwrap();
        let directory = Directory::parse(&disk).unwrap();
        assert_eq!(directory.volume.num_files, 0);
        assert_eq!(from_length_prefixed(&directory.volume.volume_name), "WORK");
        assert_eq!(directory.volume.num_blocks, 280);
        assert_eq!(pdate_to_string(directory.volume.date), "1984-11-07");
        assert_eq!(disk.num_blocks(), 280);
    }

    #[test]
    fn manyfiles_disk_directory() {
        let disk = AppleDisk::from_file(&fixture_path("manyfiles.dsk"), false).unwrap();
        let directory = Directory::parse(&disk).unwrap();
        assert_eq!(directory.volume.num_files, 76);

        let entry0 = &directory.entries[0];
        assert_eq!(from_length_prefixed(&entry0.name), "DATAFILE01.DATA");
        assert_eq!(entry0.first_block, 6);
        assert_eq!(entry0.first_after_block, 9);

        // DATAFILE11 was deleted, so entry 10 is DATAFILE12 -- this checks
        // parsing doesn't assume contiguous file numbering.
        let entry10 = &directory.entries[10];
        assert_eq!(from_length_prefixed(&entry10.name), "DATAFILE12.DATA");
        assert_eq!(entry10.first_block, 39);

        let entry75 = &directory.entries[75];
        assert_eq!(from_length_prefixed(&entry75.name), "DATAFILE77.DATA");
        assert_eq!(entry75.first_block, 234);
        assert_eq!(entry75.first_after_block, 237);
    }

    #[test]
    fn blog_disk_directory() {
        let disk = AppleDisk::from_file(&fixture_path("blog.dsk"), false).unwrap();
        let directory = Directory::parse(&disk).unwrap();
        assert_eq!(directory.volume.num_files, 8);
        let expected_names = [
            "WORK.TEXT",
            "MAKEFILES.TEXT",
            "FILESYSTEM.TEXT",
            "EDITOR.TEXT",
            "SHORT.TEXT",
            "SHORT2.TEXT",
            "INDENTS.TEXT",
            "INDENT.TEXT",
        ];
        for (i, expected_name) in expected_names.iter().enumerate() {
            let entry = &directory.entries[i];
            assert_eq!(from_length_prefixed(&entry.name), *expected_name);
            assert_eq!(entry.file_type, 3);
        }
    }

    #[test]
    fn parse_rejects_truncated_disk() {
        struct TinyDisk;
        impl DiskImage for TinyDisk {
            fn read_blocks(&self, _index: usize, _count: usize) -> &[u8] {
                &[0u8; 10]
            }
            fn num_blocks(&self) -> usize {
                1
            }
        }
        assert!(Directory::parse(&TinyDisk).is_err());
    }
}
