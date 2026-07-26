use crate::disk_image::{DiskImage, WritableDiskImage};
use std::io::Write;

const TRACK_SIZE: usize = 16 * 256;
// Apple II .dsk files have interleaved sectors; this maps logical
// (de-interleaved) sector position -> physical sector number within a
// track. Used to un-shuffle on load and re-shuffle on save.
const SECTOR_MAP: [usize; 16] = [0, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 15];

fn validate_size(len: usize) -> anyhow::Result<()> {
    anyhow::ensure!(
        len != 0 && len % TRACK_SIZE == 0,
        "disk image size ({len} bytes) is not a whole number of tracks ({TRACK_SIZE} bytes each)"
    );
    Ok(())
}

pub struct AppleDisk {
    blocks: Vec<u8>,
    source_path: String,
}

impl AppleDisk {
    pub fn from_file(name: &str, verbose: bool) -> anyhow::Result<Self> {
        Ok(Self {
            blocks: Self::read_buffer(name, verbose)?,
            source_path: name.to_string(),
        })
    }

    fn read_buffer(name: &str, verbose: bool) -> anyhow::Result<Vec<u8>> {
        let contents: Vec<u8> = std::fs::read(name)?;
        validate_size(contents.len())?;

        let mut buffer = Vec::with_capacity(contents.len());
        // Apple II .dsk files have interleaved sectors, so un-shuffle them
        let total_sectors = contents.len() / 256;
        let num_tracks = total_sectors / 16;
        if verbose {
            println!(
                "{num_tracks} tracks of 16 sectors = {total_sectors} sectors, {0} blocks",
                total_sectors / 2
            );
        }
        for track in 0..num_tracks {
            let track_offset = track * 16 * 256;
            for &sector2 in &SECTOR_MAP {
                let source_sector_offset = sector2 * 256 + track_offset;
                for byte in 0..256 {
                    buffer.push(contents[source_sector_offset + byte]);
                }
            }
        }
        debug_assert!(contents.len() == buffer.len());
        Ok(buffer)
    }

    // Exact inverse of read_buffer's un-shuffle: re-interleaves de-interleaved
    // blocks back into physical Apple II sector order.
    fn write_buffer(blocks: &[u8]) -> Vec<u8> {
        let mut contents = vec![0u8; blocks.len()];
        let total_sectors = blocks.len() / 256;
        let num_tracks = total_sectors / 16;
        for track in 0..num_tracks {
            let track_offset = track * 16 * 256;
            for (i, &sector2) in SECTOR_MAP.iter().enumerate() {
                let logical_offset = track_offset + i * 256;
                let physical_offset = track_offset + sector2 * 256;
                contents[physical_offset..physical_offset + 256]
                    .copy_from_slice(&blocks[logical_offset..logical_offset + 256]);
            }
        }
        contents
    }
}

impl DiskImage for AppleDisk {
    fn read_blocks(&self, index: usize, count: usize) -> &[u8] {
        let start: usize = index * 512;
        let end: usize = (index + count) * 512;
        &self.blocks[start..end]
    }

    fn num_blocks(&self) -> usize {
        self.blocks.len() / 512
    }
}

impl WritableDiskImage for AppleDisk {
    fn write_blocks(&mut self, index: usize, data: &[u8]) {
        let start = index * 512;
        let end = start + data.len();
        self.blocks[start..end].copy_from_slice(data);
    }

    fn save(&self) -> anyhow::Result<()> {
        let backup_path = format!("{}.bak", self.source_path);
        std::fs::copy(&self.source_path, &backup_path)?;

        let contents = Self::write_buffer(&self.blocks);
        let tmp_path = format!("{}.tmp", self.source_path);
        {
            let mut f = std::fs::File::create(&tmp_path)?;
            f.write_all(&contents)?;
            f.sync_all()?;
        }
        std::fs::rename(&tmp_path, &self.source_path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_size_rejects_empty_and_partial_tracks() {
        assert!(validate_size(0).is_err());
        assert!(validate_size(500).is_err());
        assert!(validate_size(TRACK_SIZE - 1).is_err());
    }

    #[test]
    fn validate_size_accepts_whole_tracks() {
        assert!(validate_size(TRACK_SIZE).is_ok());
        assert!(validate_size(TRACK_SIZE * 35).is_ok()); // a full 35-track disk
    }

    fn fixture_path(name: &str) -> String {
        format!("{}/../tests/AppleDsks/{name}", env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn write_blocks_round_trip() {
        let mut disk = AppleDisk::from_file(&fixture_path("empty.dsk"), false).unwrap();
        let data = vec![0xABu8; 512];
        disk.write_blocks(10, &data);
        assert_eq!(disk.read_blocks(10, 1), data.as_slice());
    }

    #[test]
    fn save_reinterleave_is_inverse_of_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.dsk");
        std::fs::copy(fixture_path("empty.dsk"), &path).unwrap();
        let original = std::fs::read(&path).unwrap();

        let disk = AppleDisk::from_file(path.to_str().unwrap(), false).unwrap();
        disk.save().unwrap();

        let saved = std::fs::read(&path).unwrap();
        assert_eq!(saved, original);
    }

    #[test]
    fn save_refreshes_backup_to_pre_write_state() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.dsk");
        std::fs::copy(fixture_path("empty.dsk"), &path).unwrap();

        let mut disk = AppleDisk::from_file(path.to_str().unwrap(), false).unwrap();
        disk.write_blocks(10, &[0x11u8; 512]);
        disk.save().unwrap();
        let state_after_first_save = std::fs::read(&path).unwrap();

        disk.write_blocks(20, &[0x22u8; 512]);
        disk.save().unwrap();

        let backup_path = format!("{}.bak", path.to_str().unwrap());
        let backup = std::fs::read(backup_path).unwrap();
        assert_eq!(backup, state_after_first_save);
    }
}
