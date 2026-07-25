use crate::disk_image::DiskImage;

pub struct AppleDisk {
    blocks: Vec<u8>,
}

impl AppleDisk {
    pub fn from_file(name: &str) -> Self {
        Self { blocks: Self::read_buffer(name) }
    }

    fn read_buffer(name: &str) -> Vec<u8> {
        let contents: Vec<u8> = std::fs::read(name).expect("couldn't read file");
        let mut buffer = Vec::with_capacity(contents.len());
        // Apple II .dsk files have interleaved sectors, so un-shuffle them
        let sector_map: [usize; 16] = [
            0, 14, 13, 12, 11, 10, 9, 8,
            7, 6, 5, 4, 3, 2, 1, 15
        ];
        let total_sectors = contents.len() / 256;
        let num_tracks = total_sectors / 16;
        println!("{num_tracks} tracks of 16 sectors = {total_sectors} sectors, {0} blocks", total_sectors/2);
        for track in 0..num_tracks {
            let track_offset = track * 16 * 256;
            for sector in 0..16 as usize {
                let sector2 = sector_map[sector];
                let source_sector_offset = sector2 * 256 + track_offset;
                for byte in 0..256 as usize {
                    buffer.push(contents[source_sector_offset+byte]);
                }
            }
        }
        assert!(contents.len() == buffer.len());
        buffer
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
