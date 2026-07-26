use crate::disk_image::DiskImage;
use p_system_format::bytes::{read_array, read_u16_le, write_array, write_u16_le};
use p_system_format::error::FormatError;
use p_system_format::pascal_string::from_length_prefixed;

const VOLUME_INFO_SIZE: usize = 26;
const DIRECTORY_ENTRY_SIZE: usize = 26;
const NUM_ENTRIES: usize = 77;
const DIRECTORY_SIZE: usize = VOLUME_INFO_SIZE + NUM_ENTRIES * DIRECTORY_ENTRY_SIZE;
// The directory occupies a whole number of 512-byte blocks (2, through 5).
pub(crate) const DIRECTORY_BLOCKS_SIZE: usize = 4 * 512;

// Standard UCSD p-System directory file type codes.
pub(crate) const FILE_TYPE_TEXTFILE: u16 = 3;
pub(crate) const FILE_TYPE_DATAFILE: u16 = 5;

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

    pub(crate) fn to_bytes(&self) -> [u8; DIRECTORY_BLOCKS_SIZE] {
        let mut buf = [0u8; DIRECTORY_BLOCKS_SIZE];
        buf[0..VOLUME_INFO_SIZE].copy_from_slice(&self.volume.to_bytes());
        let num_files = self.volume.num_files as usize;
        for (i, entry) in self.entries.iter().enumerate() {
            let offset = VOLUME_INFO_SIZE + i * DIRECTORY_ENTRY_SIZE;
            let entry_bytes = if i < num_files {
                entry.to_bytes()
            } else {
                DirectoryEntry::empty().to_bytes()
            };
            buf[offset..offset + DIRECTORY_ENTRY_SIZE].copy_from_slice(&entry_bytes);
        }
        buf
    }

    // Finds the first gap between existing files (or the tail of the volume)
    // large enough to hold `blocks_needed` contiguous blocks, mirroring how
    // real p-System allocation works.
    pub(crate) fn find_free_range(&self, blocks_needed: u16) -> Option<u16> {
        let num_files = self.volume.num_files as usize;
        let mut ranges: Vec<(u16, u16)> = self.entries[..num_files]
            .iter()
            .map(|e| (e.first_block, e.first_after_block))
            .collect();
        ranges.sort_by_key(|r| r.0);

        let mut cursor = self.volume.first_block_after_directory;
        for (start, end) in ranges {
            if start > cursor && start - cursor >= blocks_needed {
                return Some(cursor);
            }
            cursor = cursor.max(end);
        }
        if self.volume.num_blocks > cursor && self.volume.num_blocks - cursor >= blocks_needed {
            return Some(cursor);
        }
        None
    }

    pub(crate) fn add_entry(&mut self, entry: DirectoryEntry) -> anyhow::Result<()> {
        let num_files = self.volume.num_files as usize;
        anyhow::ensure!(num_files < NUM_ENTRIES, "directory is full");
        let new_name = from_length_prefixed(&entry.name);
        for existing in &self.entries[..num_files] {
            anyhow::ensure!(
                from_length_prefixed(&existing.name) != new_name,
                "{new_name} already exists on volume"
            );
        }
        self.entries[num_files] = entry;
        self.volume.num_files += 1;
        Ok(())
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

    fn to_bytes(&self) -> [u8; VOLUME_INFO_SIZE] {
        let mut buf = [0u8; VOLUME_INFO_SIZE];
        write_u16_le(&mut buf, 0, self.first_system_block).expect("size checked above");
        write_u16_le(&mut buf, 2, self.first_block_after_directory).expect("size checked above");
        write_u16_le(&mut buf, 4, self.file_type).expect("size checked above");
        write_array(&mut buf, 6, &self.volume_name).expect("size checked above");
        write_u16_le(&mut buf, 14, self.num_blocks).expect("size checked above");
        write_u16_le(&mut buf, 16, self.num_files).expect("size checked above");
        write_u16_le(&mut buf, 18, self.last_access_time).expect("size checked above");
        write_u16_le(&mut buf, 20, self.date).expect("size checked above");
        write_array(&mut buf, 22, &self.reserved).expect("size checked above");
        buf
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

    pub(crate) fn empty() -> Self {
        Self {
            first_block: 0,
            first_after_block: 0,
            file_type: 0,
            name: [0u8; 16],
            bytes_in_last_block: 0,
            date: 0,
        }
    }

    fn to_bytes(&self) -> [u8; DIRECTORY_ENTRY_SIZE] {
        let mut buf = [0u8; DIRECTORY_ENTRY_SIZE];
        write_u16_le(&mut buf, 0, self.first_block).expect("size checked above");
        write_u16_le(&mut buf, 2, self.first_after_block).expect("size checked above");
        write_u16_le(&mut buf, 4, self.file_type).expect("size checked above");
        write_array(&mut buf, 6, &self.name).expect("size checked above");
        write_u16_le(&mut buf, 22, self.bytes_in_last_block).expect("size checked above");
        write_u16_le(&mut buf, 24, self.date).expect("size checked above");
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apple_disk::AppleDisk;
    use p_system_format::pascal_string::{from_length_prefixed, to_length_prefixed};
    use p_system_format::pdate::pdate_to_string;

    fn fixture_path(name: &str) -> String {
        format!("{}/../tests/AppleDsks/{name}", env!("CARGO_MANIFEST_DIR"))
    }

    // Minimal DiskImage wrapping a raw block buffer, for feeding to_bytes()
    // output straight back into Directory::parse().
    struct BytesDisk(Vec<u8>);
    impl DiskImage for BytesDisk {
        fn read_blocks(&self, _index: usize, _count: usize) -> &[u8] {
            &self.0
        }
        fn num_blocks(&self) -> usize {
            self.0.len() / 512
        }
    }

    fn round_trip_check(fixture: &str) {
        let disk = AppleDisk::from_file(&fixture_path(fixture), false).unwrap();
        let directory = Directory::parse(&disk).unwrap();
        let bytes = directory.to_bytes();
        let mock = BytesDisk(bytes.to_vec());
        let round_tripped = Directory::parse(&mock).unwrap();

        assert_eq!(round_tripped.volume.num_files, directory.volume.num_files);
        assert_eq!(round_tripped.volume.volume_name, directory.volume.volume_name);
        assert_eq!(round_tripped.volume.num_blocks, directory.volume.num_blocks);
        assert_eq!(round_tripped.volume.date, directory.volume.date);
        for i in 0..directory.volume.num_files as usize {
            let a = &directory.entries[i];
            let b = &round_tripped.entries[i];
            assert_eq!(b.first_block, a.first_block);
            assert_eq!(b.first_after_block, a.first_after_block);
            assert_eq!(b.file_type, a.file_type);
            assert_eq!(from_length_prefixed(&b.name), from_length_prefixed(&a.name));
            assert_eq!(b.bytes_in_last_block, a.bytes_in_last_block);
            assert_eq!(b.date, a.date);
        }
    }

    fn make_directory(num_blocks: u16, entries_data: &[(u16, u16)]) -> Directory {
        let entries: [DirectoryEntry; NUM_ENTRIES] = std::array::from_fn(|i| {
            if i < entries_data.len() {
                let (first_block, first_after_block) = entries_data[i];
                DirectoryEntry {
                    first_block,
                    first_after_block,
                    file_type: FILE_TYPE_DATAFILE,
                    name: [0u8; 16],
                    bytes_in_last_block: 512,
                    date: 0,
                }
            } else {
                DirectoryEntry::empty()
            }
        });
        let volume = VolumeInfo {
            first_system_block: 0,
            first_block_after_directory: 6,
            file_type: 0,
            volume_name: [0u8; 8],
            num_blocks,
            num_files: entries_data.len() as u16,
            last_access_time: 0,
            date: 0,
            reserved: [0u8; 4],
        };
        Directory { volume, entries }
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

    #[test]
    fn directory_round_trip_empty() {
        round_trip_check("empty.dsk");
    }

    #[test]
    fn directory_round_trip_manyfiles() {
        round_trip_check("manyfiles.dsk");
    }

    #[test]
    fn directory_round_trip_blog() {
        round_trip_check("blog.dsk");
    }

    #[test]
    fn find_free_range_empty_volume() {
        let directory = make_directory(280, &[]);
        assert_eq!(directory.find_free_range(10), Some(6));
    }

    #[test]
    fn find_free_range_middle_gap() {
        let directory = make_directory(280, &[(6, 20), (100, 110)]);
        assert_eq!(directory.find_free_range(50), Some(20));
        assert_eq!(directory.find_free_range(170), Some(110));
    }

    #[test]
    fn find_free_range_no_room() {
        let directory = make_directory(280, &[(6, 280)]);
        assert_eq!(directory.find_free_range(1), None);
    }

    #[test]
    fn add_entry_rejects_duplicate() {
        let mut directory = make_directory(280, &[]);
        let name: [u8; 16] = to_length_prefixed("FOO.TEXT").unwrap();
        let entry1 = DirectoryEntry {
            first_block: 6,
            first_after_block: 8,
            file_type: FILE_TYPE_TEXTFILE,
            name,
            bytes_in_last_block: 512,
            date: 0,
        };
        directory.add_entry(entry1).unwrap();
        let entry2 = DirectoryEntry {
            first_block: 8,
            first_after_block: 10,
            file_type: FILE_TYPE_TEXTFILE,
            name,
            bytes_in_last_block: 512,
            date: 0,
        };
        assert!(directory.add_entry(entry2).is_err());
    }

    #[test]
    fn add_entry_rejects_when_full() {
        let mut directory = make_directory(280, &[]);
        for i in 0..NUM_ENTRIES {
            let name: [u8; 16] = to_length_prefixed(&format!("F{i}.TEXT")).unwrap();
            let entry = DirectoryEntry {
                first_block: 6,
                first_after_block: 7,
                file_type: FILE_TYPE_TEXTFILE,
                name,
                bytes_in_last_block: 512,
                date: 0,
            };
            directory.add_entry(entry).unwrap();
        }
        let name: [u8; 16] = to_length_prefixed("OVERFLOW.TEXT").unwrap();
        let entry = DirectoryEntry {
            first_block: 6,
            first_after_block: 7,
            file_type: FILE_TYPE_TEXTFILE,
            name,
            bytes_in_last_block: 512,
            date: 0,
        };
        assert!(directory.add_entry(entry).is_err());
    }
}
