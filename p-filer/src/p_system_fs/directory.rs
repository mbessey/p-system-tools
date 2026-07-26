use crate::disk_image::DiskImage;
use p_system_format::bytes::{read_array, read_u16_le, write_array, write_u16_le};
use p_system_format::error::FormatError;
use p_system_format::pascal_string::from_length_prefixed;

const VOLUME_INFO_SIZE: usize = 26;
const DIRECTORY_ENTRY_SIZE: usize = 26;
const NUM_ENTRIES: usize = 77;
const DIRECTORY_SIZE: usize = VOLUME_INFO_SIZE + NUM_ENTRIES * DIRECTORY_ENTRY_SIZE;
pub(crate) const BLOCK_SIZE: usize = 512;
// The directory occupies a whole number of 512-byte blocks (2, through 5).
pub(crate) const DIRECTORY_BLOCKS_SIZE: usize = 4 * BLOCK_SIZE;
// Size of DirectoryEntry's length-prefixed name field (so 15 characters,
// since byte 0 is the length). The single source of truth for both the
// field's own size and any length check performed before encoding a name.
pub(crate) const ENTRY_NAME_SIZE: usize = 16;

// Standard UCSD p-System directory file type codes.
pub(crate) const FILE_TYPE_TEXTFILE: u16 = 3;
pub(crate) const FILE_TYPE_DATAFILE: u16 = 5;

// Rounds `buf` up to a whole number of BLOCK_SIZE-byte blocks, zero-padding
// the tail. Shared by the text and raw-data to-image encoding paths so the
// padding convention can't drift between them.
pub(crate) fn pad_to_block_boundary(buf: &mut Vec<u8>) {
    let padded_len = buf.len().div_ceil(BLOCK_SIZE) * BLOCK_SIZE;
    buf.resize(padded_len, 0);
}

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
        // Both fields are trusted everywhere else in this crate to slice
        // fixed-size arrays or bound block allocation, so validate them
        // once here rather than letting a corrupted image panic later.
        if volume.num_files as usize > NUM_ENTRIES {
            return Err(FormatError::InvalidValue {
                field: "num_files",
                value: volume.num_files as u32,
            });
        }
        if volume.num_blocks as usize > disk.num_blocks() {
            return Err(FormatError::InvalidValue {
                field: "num_blocks",
                value: volume.num_blocks as u32,
            });
        }
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
    // Enumerates every free (start, end) block range between existing files
    // (and the tail up to num_blocks), in ascending order. Shared building
    // block for both allocation (find_free_range) and, eventually,
    // consolidation (krunch, which needs every gap, not just the first).
    pub(crate) fn free_ranges(&self) -> Vec<(u16, u16)> {
        let num_files = self.volume.num_files as usize;
        let mut occupied: Vec<(u16, u16)> = self.entries[..num_files]
            .iter()
            .map(|e| (e.first_block, e.first_after_block))
            .collect();
        occupied.sort_by_key(|r| r.0);

        let mut free = Vec::new();
        let mut cursor = self.volume.first_block_after_directory;
        for (start, end) in occupied {
            if start > cursor {
                free.push((cursor, start));
            }
            cursor = cursor.max(end);
        }
        if self.volume.num_blocks > cursor {
            free.push((cursor, self.volume.num_blocks));
        }
        free
    }

    pub(crate) fn find_free_range(&self, blocks_needed: u16) -> Option<u16> {
        self.free_ranges()
            .into_iter()
            .find(|(start, end)| end - start >= blocks_needed)
            .map(|(start, _)| start)
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
        // Real p-System directories keep entries in ascending first_block
        // order -- other tools (including the real Filer) rely on that
        // ordering to walk the directory and compute free space, so insert
        // in place rather than always appending, which would otherwise
        // leave entries out of order whenever a file lands in a gap instead
        // of at the volume's tail.
        let insert_pos = self.entries[..num_files]
            .iter()
            .position(|e| e.first_block > entry.first_block)
            .unwrap_or(num_files);
        self.entries.copy_within(insert_pos..num_files, insert_pos + 1);
        self.entries[insert_pos] = entry;
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
    // On-disk field offsets, defined once and shared by from_bytes/to_bytes
    // so the read and write sides can't silently drift out of sync.
    const OFFSET_FIRST_SYSTEM_BLOCK: usize = 0;
    const OFFSET_FIRST_BLOCK_AFTER_DIRECTORY: usize = 2;
    const OFFSET_FILE_TYPE: usize = 4;
    const OFFSET_VOLUME_NAME: usize = 6;
    const OFFSET_NUM_BLOCKS: usize = 14;
    const OFFSET_NUM_FILES: usize = 16;
    const OFFSET_LAST_ACCESS_TIME: usize = 18;
    const OFFSET_DATE: usize = 20;
    const OFFSET_RESERVED: usize = 22;

    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            first_system_block: read_u16_le(bytes, Self::OFFSET_FIRST_SYSTEM_BLOCK)
                .expect("size checked above"),
            first_block_after_directory: read_u16_le(
                bytes,
                Self::OFFSET_FIRST_BLOCK_AFTER_DIRECTORY,
            )
            .expect("size checked above"),
            file_type: read_u16_le(bytes, Self::OFFSET_FILE_TYPE).expect("size checked above"),
            volume_name: read_array::<8>(bytes, Self::OFFSET_VOLUME_NAME)
                .expect("size checked above"),
            num_blocks: read_u16_le(bytes, Self::OFFSET_NUM_BLOCKS).expect("size checked above"),
            num_files: read_u16_le(bytes, Self::OFFSET_NUM_FILES).expect("size checked above"),
            last_access_time: read_u16_le(bytes, Self::OFFSET_LAST_ACCESS_TIME)
                .expect("size checked above"),
            date: read_u16_le(bytes, Self::OFFSET_DATE).expect("size checked above"),
            reserved: read_array::<4>(bytes, Self::OFFSET_RESERVED).expect("size checked above"),
        }
    }

    fn to_bytes(&self) -> [u8; VOLUME_INFO_SIZE] {
        let mut buf = [0u8; VOLUME_INFO_SIZE];
        write_u16_le(&mut buf, Self::OFFSET_FIRST_SYSTEM_BLOCK, self.first_system_block)
            .expect("size checked above");
        write_u16_le(
            &mut buf,
            Self::OFFSET_FIRST_BLOCK_AFTER_DIRECTORY,
            self.first_block_after_directory,
        )
        .expect("size checked above");
        write_u16_le(&mut buf, Self::OFFSET_FILE_TYPE, self.file_type).expect("size checked above");
        write_array(&mut buf, Self::OFFSET_VOLUME_NAME, &self.volume_name)
            .expect("size checked above");
        write_u16_le(&mut buf, Self::OFFSET_NUM_BLOCKS, self.num_blocks)
            .expect("size checked above");
        write_u16_le(&mut buf, Self::OFFSET_NUM_FILES, self.num_files).expect("size checked above");
        write_u16_le(&mut buf, Self::OFFSET_LAST_ACCESS_TIME, self.last_access_time)
            .expect("size checked above");
        write_u16_le(&mut buf, Self::OFFSET_DATE, self.date).expect("size checked above");
        write_array(&mut buf, Self::OFFSET_RESERVED, &self.reserved).expect("size checked above");
        buf
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DirectoryEntry {
    pub(crate) first_block: u16,         // first block of file
    pub(crate) first_after_block: u16,   // first block after file (last block + 1)
    pub(crate) file_type: u16,           // type of file ()
    pub(crate) name: [u8; ENTRY_NAME_SIZE], // Pascal string - length is first byte
    pub(crate) bytes_in_last_block: u16, // number of bytes in last block
    pub(crate) date: u16,                // modified date
}

impl DirectoryEntry {
    // On-disk field offsets, defined once and shared by from_bytes/to_bytes
    // so the read and write sides can't silently drift out of sync.
    const OFFSET_FIRST_BLOCK: usize = 0;
    const OFFSET_FIRST_AFTER_BLOCK: usize = 2;
    const OFFSET_FILE_TYPE: usize = 4;
    const OFFSET_NAME: usize = 6;
    const OFFSET_BYTES_IN_LAST_BLOCK: usize = 22;
    const OFFSET_DATE: usize = 24;

    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            first_block: read_u16_le(bytes, Self::OFFSET_FIRST_BLOCK).expect("size checked above"),
            first_after_block: read_u16_le(bytes, Self::OFFSET_FIRST_AFTER_BLOCK)
                .expect("size checked above"),
            file_type: read_u16_le(bytes, Self::OFFSET_FILE_TYPE).expect("size checked above"),
            name: read_array::<ENTRY_NAME_SIZE>(bytes, Self::OFFSET_NAME).expect("size checked above"),
            bytes_in_last_block: read_u16_le(bytes, Self::OFFSET_BYTES_IN_LAST_BLOCK)
                .expect("size checked above"),
            date: read_u16_le(bytes, Self::OFFSET_DATE).expect("size checked above"),
        }
    }

    pub(crate) fn empty() -> Self {
        Self {
            first_block: 0,
            first_after_block: 0,
            file_type: 0,
            name: [0u8; ENTRY_NAME_SIZE],
            bytes_in_last_block: 0,
            date: 0,
        }
    }

    // Number of blocks the file occupies. Saturating: a corrupted or
    // adversarial entry could have first_after_block < first_block, which
    // this treats as zero blocks rather than underflowing.
    pub(crate) fn block_count(&self) -> usize {
        self.first_after_block.saturating_sub(self.first_block) as usize
    }

    fn to_bytes(self) -> [u8; DIRECTORY_ENTRY_SIZE] {
        let mut buf = [0u8; DIRECTORY_ENTRY_SIZE];
        write_u16_le(&mut buf, Self::OFFSET_FIRST_BLOCK, self.first_block)
            .expect("size checked above");
        write_u16_le(&mut buf, Self::OFFSET_FIRST_AFTER_BLOCK, self.first_after_block)
            .expect("size checked above");
        write_u16_le(&mut buf, Self::OFFSET_FILE_TYPE, self.file_type).expect("size checked above");
        write_array(&mut buf, Self::OFFSET_NAME, &self.name).expect("size checked above");
        write_u16_le(&mut buf, Self::OFFSET_BYTES_IN_LAST_BLOCK, self.bytes_in_last_block)
            .expect("size checked above");
        write_u16_le(&mut buf, Self::OFFSET_DATE, self.date).expect("size checked above");
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
    // output straight back into Directory::parse(). num_blocks is reported
    // separately rather than derived from the buffer's own length, since
    // that buffer only ever holds the directory's 4 blocks regardless of
    // the volume's real (much larger) size.
    struct BytesDisk {
        directory_bytes: Vec<u8>,
        num_blocks: usize,
    }
    impl DiskImage for BytesDisk {
        fn read_blocks(&self, _index: usize, _count: usize) -> &[u8] {
            &self.directory_bytes
        }
        fn num_blocks(&self) -> usize {
            self.num_blocks
        }
    }

    fn round_trip_check(fixture: &str) {
        let disk = AppleDisk::from_file(&fixture_path(fixture), false).unwrap();
        let directory = Directory::parse(&disk).unwrap();
        let bytes = directory.to_bytes();
        let mock = BytesDisk {
            directory_bytes: bytes.to_vec(),
            num_blocks: directory.volume.num_blocks as usize,
        };
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
                    name: [0u8; ENTRY_NAME_SIZE],
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
    fn parse_rejects_num_files_exceeding_num_entries() {
        // A corrupted num_files claiming more files than the fixed 77-entry
        // array can hold must be rejected here, not left to panic later when
        // something slices entries[..num_files].
        let entries_data = vec![(6u16, 7u16); 200];
        let directory = make_directory(280, &entries_data);
        let bytes = directory.to_bytes().to_vec();
        let mock = BytesDisk {
            directory_bytes: bytes,
            num_blocks: 280,
        };
        assert!(Directory::parse(&mock).is_err());
    }

    #[test]
    fn parse_rejects_num_blocks_exceeding_physical_disk_size() {
        // A corrupted/truncated image whose on-disk num_blocks claims a
        // larger volume than the disk actually is must be rejected here,
        // not left to let allocation pick an out-of-bounds start_block.
        let directory = make_directory(280, &[]);
        let bytes = directory.to_bytes().to_vec();
        let mock = BytesDisk {
            directory_bytes: bytes,
            num_blocks: 4, // far smaller than the volume's claimed 280 blocks
        };
        assert!(Directory::parse(&mock).is_err());
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
    fn free_ranges_enumerates_every_gap() {
        // Mirrors find_free_range_middle_gap's layout, but checks that every
        // gap is reported, not just the first one big enough for a given
        // request -- the building block a future krunch will need.
        let directory = make_directory(280, &[(6, 20), (100, 110)]);
        assert_eq!(directory.free_ranges(), vec![(20, 100), (110, 280)]);
    }

    #[test]
    fn free_ranges_empty_volume_is_one_big_gap() {
        let directory = make_directory(280, &[]);
        assert_eq!(directory.free_ranges(), vec![(6, 280)]);
    }

    #[test]
    fn free_ranges_no_room_is_empty() {
        let directory = make_directory(280, &[(6, 280)]);
        assert_eq!(directory.free_ranges(), vec![]);
    }

    #[test]
    fn add_entry_rejects_duplicate() {
        let mut directory = make_directory(280, &[]);
        let name: [u8; ENTRY_NAME_SIZE] = to_length_prefixed("FOO.TEXT").unwrap();
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
            let name: [u8; ENTRY_NAME_SIZE] = to_length_prefixed(&format!("F{i}.TEXT")).unwrap();
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
        let name: [u8; ENTRY_NAME_SIZE] = to_length_prefixed("OVERFLOW.TEXT").unwrap();
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

    #[test]
    fn add_entry_inserts_in_first_block_order() {
        let mut directory = make_directory(280, &[(6, 20), (100, 110)]);
        let name: [u8; ENTRY_NAME_SIZE] = to_length_prefixed("MIDDLE.DATA").unwrap();
        let entry = DirectoryEntry {
            first_block: 50,
            first_after_block: 60,
            file_type: FILE_TYPE_DATAFILE,
            name,
            bytes_in_last_block: 512,
            date: 0,
        };
        directory.add_entry(entry).unwrap();

        assert_eq!(directory.volume.num_files, 3);
        let blocks: Vec<u16> = directory.entries[..3].iter().map(|e| e.first_block).collect();
        assert_eq!(blocks, vec![6, 50, 100]);
        assert_eq!(from_length_prefixed(&directory.entries[1].name), "MIDDLE.DATA");
    }

    #[test]
    fn add_entry_appends_when_it_belongs_at_the_tail() {
        let mut directory = make_directory(280, &[(6, 20), (100, 110)]);
        let name: [u8; ENTRY_NAME_SIZE] = to_length_prefixed("LAST.DATA").unwrap();
        let entry = DirectoryEntry {
            first_block: 200,
            first_after_block: 210,
            file_type: FILE_TYPE_DATAFILE,
            name,
            bytes_in_last_block: 512,
            date: 0,
        };
        directory.add_entry(entry).unwrap();

        let blocks: Vec<u16> = directory.entries[..3].iter().map(|e| e.first_block).collect();
        assert_eq!(blocks, vec![6, 100, 200]);
    }

    #[test]
    fn block_count_is_normal_difference_for_a_well_formed_entry() {
        let entry = DirectoryEntry {
            first_block: 10,
            first_after_block: 13,
            file_type: FILE_TYPE_DATAFILE,
            name: [0u8; ENTRY_NAME_SIZE],
            bytes_in_last_block: 512,
            date: 0,
        };
        assert_eq!(entry.block_count(), 3);
    }

    #[test]
    fn block_count_saturates_instead_of_underflowing_on_a_corrupt_entry() {
        let entry = DirectoryEntry {
            first_block: 13,
            first_after_block: 10, // corrupt: before first_block
            file_type: FILE_TYPE_DATAFILE,
            name: [0u8; ENTRY_NAME_SIZE],
            bytes_in_last_block: 512,
            date: 0,
        };
        assert_eq!(entry.block_count(), 0);
    }
}
