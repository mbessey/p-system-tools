use p_system_format::pascal_string::{from_length_prefixed, to_length_prefixed};
use p_system_format::pdate::{now_to_pdate, pdate_to_string, pdate_to_systime, systime_to_pdate};
use std::fs::File;
use std::io::prelude::*;

use super::directory::{Directory, DirectoryEntry, FILE_TYPE_DATAFILE, FILE_TYPE_TEXTFILE};
use super::text::{text_from_blocks, text_to_blocks};
use crate::disk_image::{DiskImage, WritableDiskImage};

pub struct Volume<D: DiskImage> {
    disk: D,
    image_name: String,
    directory: Directory,
}

impl<D: DiskImage> Volume<D> {
    pub fn new(disk: D, image_name: String) -> anyhow::Result<Self> {
        let directory = Directory::parse(&disk)?;
        Ok(Self {
            disk,
            image_name,
            directory,
        })
    }

    pub fn list(&self) -> anyhow::Result<()> {
        println!("Listing files on {0}", self.image_name);
        println!(
            "First block (should be 0): {}",
            self.directory.volume.first_system_block
        );
        println!(
            "First block after directory (should be 6): {}",
            self.directory.volume.first_block_after_directory
        );
        println!(
            "File type (should be 0): {}",
            self.directory.volume.file_type
        );
        println!(
            "Volume name:      {}",
            from_length_prefixed(&self.directory.volume.volume_name)
        );
        println!("Number of blocks: {}", self.directory.volume.num_blocks);
        println!("Number of files:  {}", self.directory.volume.num_files);
        println!(
            "Last access time: {}",
            self.directory.volume.last_access_time
        );
        println!(
            "Date:             {}",
            pdate_to_string(self.directory.volume.date)
        );
        println!("Reserved:         {:?}", self.directory.volume.reserved);
        for index in 0..self.directory.volume.num_files {
            let entry = &self.directory.entries[index as usize];
            println!("Entry {index}:");
            println!("  First block:         {}", entry.first_block);
            println!("  First block after:   {}", entry.first_after_block);
            println!("  File type:           {}", entry.file_type);
            println!(
                "  Name:                {}",
                from_length_prefixed(&entry.name)
            );
            println!("  Bytes in last block: {}", entry.bytes_in_last_block);
            println!("  Date:                {}", pdate_to_string(entry.date));
        }
        Ok(())
    }

    pub fn dump(&self, from: usize, to: usize) -> anyhow::Result<()> {
        anyhow::ensure!(from <= to, "from ({from}) must be less than to ({to})");
        anyhow::ensure!(
            to <= self.disk.num_blocks(),
            "to ({to}) must be less than {0} blocks",
            self.disk.num_blocks()
        );
        anyhow::ensure!(
            from <= self.disk.num_blocks(),
            "from ({from}) must be less than {0} blocks",
            self.disk.num_blocks()
        );
        println!(
            "Dumping contexts of {0} from block {1} to {2}",
            self.image_name, from, to
        );
        let line_len = 16;
        for block_no in from..=to {
            let block = self.disk.read_blocks(block_no, 1);
            for line in 0..512 / line_len {
                let offset: usize = block_no * 512 + line * line_len;
                print!("{:06x}  ", offset);
                for byte in 0..line_len {
                    let val = block[byte + line * line_len];
                    print!("{:02x} ", val);
                }
                print!("  |");
                for byte in 0..line_len {
                    let mut c = block[byte + line * line_len];
                    if !(32..=126).contains(&c) {
                        c = 46;
                    }
                    print!("{}", char::from(c));
                }
                println!("|");
            }
            println!()
        }
        Ok(())
    }
}

impl<D: WritableDiskImage> Volume<D> {
    pub fn save(&self) -> anyhow::Result<()> {
        self.disk.save()
    }

    pub fn remove(&self, name: &str) -> anyhow::Result<()> {
        println!("Removing {name} on {0}", self.image_name);
        Ok(())
    }

    pub fn transfer(
        &mut self,
        name: &str,
        to_image: bool,
        is_text: bool,
        preserve_date: bool,
    ) -> anyhow::Result<()> {
        if to_image {
            println!("Copying {name} to {0}", self.image_name);
            let host_bytes = std::fs::read(name)?;
            let host_len = host_bytes.len();
            let (block_bytes, file_type) = if is_text {
                (text_to_blocks(&host_bytes)?, FILE_TYPE_TEXTFILE)
            } else {
                let mut b = host_bytes;
                let padded_len = b.len().div_ceil(512) * 512;
                b.resize(padded_len, 0);
                (b, FILE_TYPE_DATAFILE)
            };
            let blocks_needed_total = block_bytes.len() / 512;
            anyhow::ensure!(
                blocks_needed_total <= self.directory.volume.num_blocks as usize,
                "{name} needs {blocks_needed_total} blocks, but the volume only has {0} blocks total",
                self.directory.volume.num_blocks
            );
            let blocks_needed = blocks_needed_total as u16;
            let start_block = self
                .directory
                .find_free_range(blocks_needed)
                .ok_or_else(|| anyhow::anyhow!("not enough contiguous free space for {name}"))?;
            let date = if preserve_date {
                systime_to_pdate(std::fs::metadata(name)?.modified()?)?
            } else {
                now_to_pdate()?
            };
            let bytes_in_last_block = if is_text {
                512
            } else if host_len == 0 {
                0
            } else {
                let rem = (host_len % 512) as u16;
                if rem == 0 { 512 } else { rem }
            };
            self.directory.add_entry(DirectoryEntry {
                first_block: start_block,
                first_after_block: start_block + blocks_needed,
                file_type,
                name: to_length_prefixed::<16>(name)?,
                bytes_in_last_block,
                date,
            })?;
            self.disk.write_blocks(start_block as usize, &block_bytes);
            self.disk.write_blocks(2, &self.directory.to_bytes());
            self.save()?;
            println!("Wrote {name} to {0}", self.image_name);
            Ok(())
        } else {
            println!("Copying {name} from {0}", self.image_name);
            for entry in &self.directory.entries {
                let entry_name = from_length_prefixed(&entry.name);
                if entry_name == name {
                    println!("Found {name} at block {0}", entry.first_block);
                    let file_buffer = self.disk.read_blocks(
                        entry.first_block as usize,
                        entry.first_after_block as usize - entry.first_block as usize,
                    );
                    let file_name = name.to_string();
                    // Because we want to possibly use set_times, we'll
                    // have to use more conventional File:: methods.
                    let mut filedesc = File::create(file_name)?;
                    if is_text {
                        let text_buffer = text_from_blocks(file_buffer);
                        let _ = filedesc.write(text_buffer.as_slice());
                    } else {
                        // Only the last block is partially used; trim to the
                        // real content length instead of writing the whole
                        // block-aligned buffer (with .min() as a defensive
                        // guard against a corrupt/implausible metadata value).
                        let num_blocks_in_file = (entry.first_after_block
                            - entry.first_block) as usize;
                        let trimmed_len = if num_blocks_in_file == 0 {
                            0
                        } else {
                            (num_blocks_in_file - 1) * 512 + entry.bytes_in_last_block as usize
                        }
                        .min(file_buffer.len());
                        let _ = filedesc.write(&file_buffer[..trimmed_len]);
                    }
                    println!("Wrote {name} to disk");
                    if preserve_date {
                        let _ = filedesc.set_modified(pdate_to_systime(entry.date));
                    }
                    filedesc.sync_all()?;
                    return Ok(());
                }
            }
            Ok(())
        }
    }

    pub fn change(&self, from: &str, to: &str) -> anyhow::Result<()> {
        println!("Renaming {from} to {to} on {0}", self.image_name);
        Ok(())
    }

    pub fn krunch(&self) -> anyhow::Result<()> {
        println!("Consolidating free space on {0}", self.image_name);
        Ok(())
    }

    pub fn zero(&self) -> anyhow::Result<()> {
        println!("Clearing directory on {0}", self.image_name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apple_disk::AppleDisk;
    use std::sync::Mutex;

    fn fixture_path(name: &str) -> String {
        format!("{}/../tests/AppleDsks/{name}", env!("CARGO_MANIFEST_DIR"))
    }

    // transfer(--to-image) reads/writes the host file relative to the
    // process's current directory, so these tests need a private cwd; guard
    // it with a lock so parallel test threads don't race on process-global
    // state (nothing else in the suite depends on cwd).
    static CWD_LOCK: Mutex<()> = Mutex::new(());

    fn in_temp_dir(f: impl FnOnce(&std::path::Path)) {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        f(dir.path());
        std::env::set_current_dir(original_cwd).unwrap();
    }

    fn open_volume(image_path: &std::path::Path) -> Volume<AppleDisk> {
        Volume::new(
            AppleDisk::from_file(image_path.to_str().unwrap(), false).unwrap(),
            image_path.to_str().unwrap().to_string(),
        )
        .unwrap()
    }

    #[test]
    fn transfer_to_image_text_round_trip() {
        in_temp_dir(|dir| {
            let image_path = dir.join("scratch.dsk");
            std::fs::copy(fixture_path("empty.dsk"), &image_path).unwrap();
            std::fs::write("HELLO.TEXT", "hello p-system\nsecond line\n").unwrap();

            let mut volume = open_volume(&image_path);
            volume.transfer("HELLO.TEXT", true, true, false).unwrap();

            let mut reopened = open_volume(&image_path);
            assert_eq!(reopened.directory.volume.num_files, 1);
            let entry = &reopened.directory.entries[0];
            assert_eq!(from_length_prefixed(&entry.name), "HELLO.TEXT");
            assert_eq!(entry.file_type, FILE_TYPE_TEXTFILE);

            std::fs::remove_file("HELLO.TEXT").unwrap();
            reopened.transfer("HELLO.TEXT", false, true, false).unwrap();
            let round_tripped = std::fs::read_to_string("HELLO.TEXT").unwrap();
            assert_eq!(round_tripped, "hello p-system\nsecond line\n");
        });
    }

    #[test]
    fn transfer_to_image_binary_round_trip() {
        in_temp_dir(|dir| {
            let image_path = dir.join("scratch.dsk");
            std::fs::copy(fixture_path("empty.dsk"), &image_path).unwrap();
            let data: Vec<u8> = (0..1024u32).map(|b| (b % 256) as u8).collect();
            std::fs::write("DATA.DATA", &data).unwrap();

            let mut volume = open_volume(&image_path);
            volume.transfer("DATA.DATA", true, false, false).unwrap();

            let mut reopened = open_volume(&image_path);
            let entry = &reopened.directory.entries[0];
            assert_eq!(entry.file_type, FILE_TYPE_DATAFILE);
            assert_eq!(entry.first_after_block - entry.first_block, 2);

            std::fs::remove_file("DATA.DATA").unwrap();
            reopened.transfer("DATA.DATA", false, false, false).unwrap();
            let round_tripped = std::fs::read("DATA.DATA").unwrap();
            assert_eq!(round_tripped, data);
        });
    }

    #[test]
    fn transfer_to_image_binary_round_trip_non_block_aligned() {
        in_temp_dir(|dir| {
            let image_path = dir.join("scratch.dsk");
            std::fs::copy(fixture_path("empty.dsk"), &image_path).unwrap();
            // Deliberately not a multiple of 512, to exercise from-image
            // trimming to entry.bytes_in_last_block on extraction.
            let data: Vec<u8> = (0..700u32).map(|b| (b % 256) as u8).collect();
            std::fs::write("ODD.DATA", &data).unwrap();

            let mut volume = open_volume(&image_path);
            volume.transfer("ODD.DATA", true, false, false).unwrap();

            let mut reopened = open_volume(&image_path);
            let entry = &reopened.directory.entries[0];
            assert_eq!(entry.bytes_in_last_block, 700 % 512);

            std::fs::remove_file("ODD.DATA").unwrap();
            reopened.transfer("ODD.DATA", false, false, false).unwrap();
            let round_tripped = std::fs::read("ODD.DATA").unwrap();
            assert_eq!(round_tripped, data);
        });
    }

    #[test]
    fn transfer_to_image_empty_file_has_consistent_metadata() {
        in_temp_dir(|dir| {
            let image_path = dir.join("scratch.dsk");
            std::fs::copy(fixture_path("empty.dsk"), &image_path).unwrap();
            std::fs::write("EMPTY.DATA", b"").unwrap();

            let mut volume = open_volume(&image_path);
            volume.transfer("EMPTY.DATA", true, false, false).unwrap();

            let reopened = open_volume(&image_path);
            let entry = &reopened.directory.entries[0];
            assert_eq!(entry.first_block, entry.first_after_block);
            assert_eq!(entry.bytes_in_last_block, 0);
        });
    }

    #[test]
    fn transfer_to_image_errors_cleanly_when_file_exceeds_volume_capacity() {
        in_temp_dir(|dir| {
            let image_path = dir.join("scratch.dsk");
            std::fs::copy(fixture_path("empty.dsk"), &image_path).unwrap();
            // empty.dsk has 280 blocks (143360 bytes) total; this file needs
            // more blocks than the entire volume has, exercising the guard
            // that rejects oversized input before it could ever truncate a
            // block count through a u16 cast.
            let data = vec![0u8; 512 * 300];
            std::fs::write("HUGE.DATA", &data).unwrap();

            let mut volume = open_volume(&image_path);
            let err = volume.transfer("HUGE.DATA", true, false, false);
            assert!(err.is_err());
        });
    }

    #[test]
    fn transfer_to_image_rejects_duplicate_name() {
        in_temp_dir(|dir| {
            let image_path = dir.join("scratch.dsk");
            std::fs::copy(fixture_path("blog.dsk"), &image_path).unwrap();
            std::fs::write("WORK.TEXT", "hi\n").unwrap();

            let mut volume = open_volume(&image_path);
            assert!(volume.transfer("WORK.TEXT", true, true, false).is_err());
        });
    }

    #[test]
    fn transfer_to_image_errors_when_full() {
        in_temp_dir(|dir| {
            let image_path = dir.join("scratch.dsk");
            std::fs::copy(fixture_path("manyfiles.dsk"), &image_path).unwrap();
            // manyfiles.dsk's files nearly fill its 280 blocks; nothing this
            // large can fit in any remaining gap.
            let data = vec![0u8; 512 * 100];
            std::fs::write("BIGFILE.DATA", &data).unwrap();

            let mut volume = open_volume(&image_path);
            assert!(
                volume
                    .transfer("BIGFILE.DATA", true, false, false)
                    .is_err()
            );
        });
    }
}
