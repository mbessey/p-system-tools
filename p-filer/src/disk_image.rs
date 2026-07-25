// Generic interface for reading a disk image as a sequence of fixed-size
// 512-byte blocks, independent of the underlying physical format (sector
// interleaving, track layout, container headers, etc). p_system_fs's
// directory/volume logic is written against this trait so it works with any
// disk format that can produce a block buffer this way.

pub trait DiskImage {
    fn read_blocks(&self, index: usize, count: usize) -> &[u8];
    fn num_blocks(&self) -> usize;
}
