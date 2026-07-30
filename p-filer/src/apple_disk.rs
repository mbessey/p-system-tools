use crate::disk_image::{DiskImage, WritableDiskImage};
use crate::error::Error;
use std::io::Write;

const TRACK_SIZE: usize = 16 * 256;
// Apple II .dsk files have interleaved sectors; this maps logical
// (de-interleaved) sector position -> physical sector number within a
// track. Used to un-shuffle on load and re-shuffle on save.
const SECTOR_MAP: [usize; 16] = [0, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 15];

fn validate_size(len: usize) -> Result<(), Error> {
    if len == 0 || !len.is_multiple_of(TRACK_SIZE) {
        return Err(Error::InvalidImageSize {
            len,
            track_size: TRACK_SIZE,
        });
    }
    Ok(())
}

// Walks every (logical_offset, physical_offset) 256-byte-sector pair implied
// by SECTOR_MAP across all tracks in a buffer of the given length. Shared by
// read_buffer (physical -> logical) and write_buffer (logical -> physical)
// so the un-shuffle and re-shuffle geometry can't drift out of sync.
fn for_each_sector(total_len: usize, mut f: impl FnMut(usize, usize)) {
    let num_tracks = total_len / TRACK_SIZE;
    for track in 0..num_tracks {
        let track_offset = track * TRACK_SIZE;
        for (i, &sector2) in SECTOR_MAP.iter().enumerate() {
            let logical_offset = track_offset + i * 256;
            let physical_offset = track_offset + sector2 * 256;
            f(logical_offset, physical_offset);
        }
    }
}

pub struct AppleDisk {
    blocks: Vec<u8>,
    source_path: String,
}

impl AppleDisk {
    pub fn from_file(name: &str, verbose: bool) -> Result<Self, Error> {
        Ok(Self {
            blocks: Self::read_buffer(name, verbose)?,
            source_path: name.to_string(),
        })
    }

    fn read_buffer(name: &str, verbose: bool) -> Result<Vec<u8>, Error> {
        let contents: Vec<u8> = std::fs::read(name)?;
        validate_size(contents.len())?;

        if verbose {
            let total_sectors = contents.len() / 256;
            println!(
                "{0} tracks of 16 sectors = {total_sectors} sectors, {1} blocks",
                total_sectors / 16,
                total_sectors / 2
            );
        }
        // Apple II .dsk files have interleaved sectors, so un-shuffle them.
        // The zero-fill below is fully overwritten by for_each_sector on any
        // validly-sized image (it covers every byte exactly once); it's kept
        // anyway as a defined fallback value rather than reaching for unsafe
        // uninitialized-buffer construction to skip a memset that's already
        // negligible at these disk-image sizes (a few hundred KB at most).
        let mut buffer = vec![0u8; contents.len()];
        for_each_sector(contents.len(), |logical_offset, physical_offset| {
            buffer[logical_offset..logical_offset + 256]
                .copy_from_slice(&contents[physical_offset..physical_offset + 256]);
        });
        Ok(buffer)
    }

    // Exact inverse of read_buffer's un-shuffle: re-interleaves de-interleaved
    // blocks back into physical Apple II sector order. See read_buffer's
    // comment on the zero-fill below being redundant-but-intentional.
    fn write_buffer(blocks: &[u8]) -> Vec<u8> {
        let mut contents = vec![0u8; blocks.len()];
        for_each_sector(blocks.len(), |logical_offset, physical_offset| {
            contents[physical_offset..physical_offset + 256]
                .copy_from_slice(&blocks[logical_offset..logical_offset + 256]);
        });
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

    fn save(&self) -> Result<(), Error> {
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
