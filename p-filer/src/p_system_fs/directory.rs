use crate::disk_image::DiskImage;

// Directory entries are each 26 bytes. The first is a bit special, and contains information about the volume itself.
// The rest are the files on the volume. Directory entries occupy blocks 2 through 5 on the disk.
#[derive(Debug)]
#[repr(C)]
pub struct Directory {
    pub(crate) volume: VolumeInfo,
    pub(crate) entries: [DirectoryEntry; 77],
}

impl Directory {
    pub fn read(disk: &impl DiskImage) -> Self {
        let bytes = disk.read_blocks(2, 4);
        let directory_ptr = bytes.as_ptr() as *const Directory;
        unsafe { directory_ptr.read_unaligned() }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct VolumeInfo {
    pub(crate) first_system_block: u16, // always zero
    pub(crate) first_block_after_directory: u16, // always 6
    pub(crate) file_type: u16, // always zero
    pub(crate) volume_name: [u8; 8], // Pascal string - length is first byte
    pub(crate) num_blocks: u16, // number of blocks in volume
    pub(crate) num_files: u16, // number of files in directory
    pub(crate) last_access_time: u16, // last access time - always zero?
    pub(crate) date: u16, // date set by user
    pub(crate) reserved: [u8; 4], // reserved for future use
}

#[derive(Debug)]
#[repr(C)]
pub struct DirectoryEntry {
    pub(crate) first_block: u16, // first block of file
    pub(crate) first_after_block: u16, // first block after file (last block + 1)
    pub(crate) file_type: u16, // type of file ()
    pub(crate) name: [u8; 16], // Pascal string - length is first byte
    pub(crate) bytes_in_last_block: u16, // number of bytes in last block
    pub(crate) date: u16, // modified date
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
        let disk = AppleDisk::from_file(&fixture_path("empty.dsk"));
        let directory = Directory::read(&disk);
        assert_eq!(directory.volume.num_files, 0);
        assert_eq!(from_length_prefixed(&directory.volume.volume_name), "WORK");
        assert_eq!(directory.volume.num_blocks, 280);
        assert_eq!(pdate_to_string(directory.volume.date), "1984-11-07");
        assert_eq!(disk.num_blocks(), 280);
    }

    #[test]
    fn manyfiles_disk_directory() {
        let disk = AppleDisk::from_file(&fixture_path("manyfiles.dsk"));
        let directory = Directory::read(&disk);
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
        let disk = AppleDisk::from_file(&fixture_path("blog.dsk"));
        let directory = Directory::read(&disk);
        assert_eq!(directory.volume.num_files, 8);
        let expected_names = [
            "WORK.TEXT", "MAKEFILES.TEXT", "FILESYSTEM.TEXT", "EDITOR.TEXT",
            "SHORT.TEXT", "SHORT2.TEXT", "INDENTS.TEXT", "INDENT.TEXT",
        ];
        for (i, expected_name) in expected_names.iter().enumerate() {
            let entry = &directory.entries[i];
            assert_eq!(from_length_prefixed(&entry.name), *expected_name);
            assert_eq!(entry.file_type, 3);
        }
    }
}
