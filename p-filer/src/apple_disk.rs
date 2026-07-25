use crate::disk_image::DiskImage;

const TRACK_SIZE: usize = 16 * 256;

fn validate_size(len: usize) -> anyhow::Result<()> {
    anyhow::ensure!(
        len != 0 && len % TRACK_SIZE == 0,
        "disk image size ({len} bytes) is not a whole number of tracks ({TRACK_SIZE} bytes each)"
    );
    Ok(())
}

pub struct AppleDisk {
    blocks: Vec<u8>,
}

impl AppleDisk {
    pub fn from_file(name: &str, verbose: bool) -> anyhow::Result<Self> {
        Ok(Self {
            blocks: Self::read_buffer(name, verbose)?,
        })
    }

    fn read_buffer(name: &str, verbose: bool) -> anyhow::Result<Vec<u8>> {
        let contents: Vec<u8> = std::fs::read(name)?;
        validate_size(contents.len())?;

        let mut buffer = Vec::with_capacity(contents.len());
        // Apple II .dsk files have interleaved sectors, so un-shuffle them
        let sector_map: [usize; 16] = [0, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 15];
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
            for &sector2 in &sector_map {
                let source_sector_offset = sector2 * 256 + track_offset;
                for byte in 0..256 {
                    buffer.push(contents[source_sector_offset + byte]);
                }
            }
        }
        debug_assert!(contents.len() == buffer.len());
        Ok(buffer)
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
}
