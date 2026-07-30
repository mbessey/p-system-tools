use crate::error::Error;
use p_system_format::pascal_string::{from_length_prefixed, to_length_prefixed};
use p_system_format::pdate::{now_to_pdate, pdate_to_string, pdate_to_systime, systime_to_pdate};
use std::fs::File;
use std::io::prelude::*;

use super::directory::{
    BLOCK_SIZE, Directory, DirectoryEntry, ENTRY_NAME_SIZE, FILE_TYPE_DATAFILE, FILE_TYPE_TEXTFILE,
    VOLUME_NAME_SIZE, bytes_in_last_block, pad_to_block_boundary,
};
use super::text::{text_from_blocks, text_to_blocks};
use crate::disk_image::{DiskImage, WritableDiskImage};

pub struct Volume<D: DiskImage> {
    disk: D,
    image_name: String,
    directory: Directory,
}

impl<D: DiskImage> Volume<D> {
    pub fn new(disk: D, image_name: String) -> Result<Self, Error> {
        let directory = Directory::parse(&disk)?;
        Ok(Self {
            disk,
            image_name,
            directory,
        })
    }

    pub fn list(&self) -> Result<(), Error> {
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

    pub fn dump(&self, from: usize, to: usize) -> Result<(), Error> {
        if from > to {
            return Err(Error::DumpRangeInverted { from, to });
        }
        if to > self.disk.num_blocks() {
            return Err(Error::DumpBlockOutOfRange {
                which: "to",
                value: to,
                num_blocks: self.disk.num_blocks(),
            });
        }
        if from > self.disk.num_blocks() {
            return Err(Error::DumpBlockOutOfRange {
                which: "from",
                value: from,
                num_blocks: self.disk.num_blocks(),
            });
        }
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
    pub fn save(&self) -> Result<(), Error> {
        self.disk.save()
    }

    // Removes name's directory entry and persists the updated directory.
    // The file's old blocks aren't zeroed -- only the entry is spliced
    // out, matching the real Filer's Remove and the same "absent from
    // entries[..num_files] means free" convention the rest of this format
    // relies on (see Directory::remove_entry). Those blocks simply become
    // part of a gap the next transfer/krunch can reuse or close.
    pub fn remove(&mut self, name: &str) -> Result<(), Error> {
        println!("Removing {name} on {0}", self.image_name);
        self.directory.remove_entry(name)?;
        self.disk.write_blocks(2, &self.directory.to_bytes());
        self.save()?;
        Ok(())
    }

    pub fn transfer(
        &mut self,
        name: &str,
        to_image: bool,
        is_text: bool,
        preserve_date: bool,
    ) -> Result<(), Error> {
        if to_image {
            println!("Copying {name} to {0}", self.image_name);
            // `name` may be a full host path (e.g. run from another
            // directory); only its final component is meaningful as a
            // p-System volume filename, which is limited to 15 characters.
            // Uppercase it: the Filer can display a lowercase name, but the
            // Editor and other system tools don't handle them well, so
            // volume filenames are conventionally all-uppercase.
            let volume_name = std::path::Path::new(name)
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| Error::NoFileName {
                    name: name.to_string(),
                })?
                .to_ascii_uppercase();
            if volume_name.len() >= ENTRY_NAME_SIZE {
                let len = volume_name.len();
                return Err(Error::FileNameTooLong {
                    name: volume_name,
                    len,
                    limit: ENTRY_NAME_SIZE - 1,
                });
            }
            let host_bytes = std::fs::read(name)?;
            // bytes_in_last_block means different things for the two file
            // types. For a data file it's load-bearing: extraction trims to
            // it, so it must reflect the real pre-padding content length.
            // For a text file, real Apple Pascal never seems to compute a
            // real value here -- every text file on tests/AppleDsks/blog.dsk,
            // and both a real Editor-authored 0-byte and 89-byte text file
            // (experiments4.dsk), report a full block (512) regardless of
            // actual content length. Text files find their own end by
            // scanning for CR/RLE markers instead, so this field just isn't
            // used for them; match that rather than computing a real value
            // no real Filer/Editor ever writes.
            let (block_bytes, file_type, bytes_in_last_block) = if is_text {
                let blocks = text_to_blocks(&host_bytes)?;
                (blocks, FILE_TYPE_TEXTFILE, BLOCK_SIZE as u16)
            } else {
                let content_len = host_bytes.len();
                let mut b = host_bytes;
                pad_to_block_boundary(&mut b);
                (b, FILE_TYPE_DATAFILE, bytes_in_last_block(content_len))
            };
            let blocks_needed_total = block_bytes.len() / BLOCK_SIZE;
            if blocks_needed_total > self.directory.volume.num_blocks as usize {
                return Err(Error::NotEnoughSpaceTotal {
                    name: name.to_string(),
                    needed: blocks_needed_total,
                    available: self.directory.volume.num_blocks as usize,
                });
            }
            let blocks_needed = blocks_needed_total as u16;
            let start_block = self
                .directory
                .find_free_range(blocks_needed)
                .ok_or_else(|| Error::NoContiguousSpace {
                    name: name.to_string(),
                })?;
            let date = if preserve_date {
                systime_to_pdate(std::fs::metadata(name)?.modified()?)?
            } else {
                now_to_pdate()?
            };
            self.directory.add_entry(DirectoryEntry {
                first_block: start_block,
                first_after_block: start_block + blocks_needed,
                file_type,
                name: to_length_prefixed::<ENTRY_NAME_SIZE>(&volume_name)?,
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
            let num_files = self.directory.volume.num_files as usize;
            for entry in &self.directory.entries[..num_files] {
                let entry_name = from_length_prefixed(&entry.name);
                if entry_name == name {
                    println!("Found {name} at block {0}", entry.first_block);
                    let num_blocks_in_file = entry.block_count();
                    let file_buffer = self
                        .disk
                        .read_blocks(entry.first_block as usize, num_blocks_in_file);
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
                        // block-aligned buffer. .min() is a defensive guard
                        // against an implausible bytes_in_last_block value;
                        // num_blocks_in_file itself can't underflow since it
                        // was already saturated above.
                        let trimmed_len = if num_blocks_in_file == 0 {
                            0
                        } else {
                            (num_blocks_in_file - 1) * BLOCK_SIZE
                                + entry.bytes_in_last_block as usize
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
            Err(Error::FileNotFoundOnImage {
                name: name.to_string(),
                image_name: self.image_name.clone(),
            })
        }
    }

    // Renames from's entry to to. to is a volume filename (not a host
    // path), so it's uppercased and length-checked the same way
    // transfer's to-image path validates one (see the volume_name
    // handling above) -- both are subject to the same 15-character,
    // conventionally-uppercase p-System naming rule.
    pub fn change(&mut self, from: &str, to: &str) -> Result<(), Error> {
        println!("Renaming {from} to {to} on {0}", self.image_name);
        let to_upper = to.to_ascii_uppercase();
        if to_upper.len() >= ENTRY_NAME_SIZE {
            return Err(Error::RenameTargetTooLong {
                len: to_upper.len(),
                name: to_upper,
                limit: ENTRY_NAME_SIZE - 1,
            });
        }
        self.directory.rename_entry(from, &to_upper)?;
        self.disk.write_blocks(2, &self.directory.to_bytes());
        self.save()?;
        Ok(())
    }

    // Consolidates free space by sliding every file down to close any gap
    // before it, merging all free space into one region at the volume's
    // tail. Directory::compact decides the new block layout (a pure,
    // disk-free decision); this just carries out the physical block
    // moves it prescribes. Each move reads its source blocks into an
    // owned buffer before writing the destination, both because
    // read_blocks' borrow of self.disk must end before write_blocks can
    // borrow it mutably, and because compact's ordering guarantee (a
    // move's destination never reaches into a later entry's still-unread
    // source blocks) depends on each entry being fully read before any
    // subsequent write.
    pub fn krunch(&mut self) -> Result<(), Error> {
        println!("Consolidating free space on {0}", self.image_name);
        for mv in self.directory.compact() {
            let block_bytes = self
                .disk
                .read_blocks(mv.from as usize, mv.block_count as usize)
                .to_vec();
            self.disk.write_blocks(mv.to as usize, &block_bytes);
        }
        self.disk.write_blocks(2, &self.directory.to_bytes());
        self.save()?;
        Ok(())
    }

    // Clears the volume's catalog. Only num_files needs to change --
    // to_bytes() already writes every slot past num_files as an empty
    // entry regardless of what's sitting in the in-memory array -- so
    // this is sufficient to make the whole directory serialize as empty.
    // File blocks themselves aren't touched, matching the real Filer's
    // Zero-Directory command and the same "absence from
    // entries[..num_files] means free" convention remove relies on.
    //
    // The real Filer's Zero command also prompts for a new volume name
    // while it's at it, so new_name mirrors that: None leaves the
    // existing name alone, Some renames the volume the same way `change`
    // renames a file (uppercased, length-checked -- but against
    // VOLUME_NAME_SIZE, not ENTRY_NAME_SIZE, since the volume header's
    // name field is only 7 characters wide, shorter than a file's 15).
    pub fn zero(&mut self, new_name: Option<&str>) -> Result<(), Error> {
        println!("Clearing directory on {0}", self.image_name);
        if let Some(name) = new_name {
            let name_upper = name.to_ascii_uppercase();
            if name_upper.len() >= VOLUME_NAME_SIZE {
                return Err(Error::VolumeNameTooLong {
                    len: name_upper.len(),
                    name: name_upper,
                    limit: VOLUME_NAME_SIZE - 1,
                });
            }
            self.directory.set_volume_name(&name_upper)?;
        }
        self.directory.volume.num_files = 0;
        self.disk.write_blocks(2, &self.directory.to_bytes());
        self.save()?;
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

    // Restores the process's cwd on drop, including during a panic, so one
    // failing assertion inside `f` can't leave the cwd pointing at a
    // tempdir that's about to be deleted -- which would otherwise break
    // every other test in the process that also depends on cwd.
    struct RestoreCwd(std::path::PathBuf);
    impl Drop for RestoreCwd {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.0);
        }
    }

    fn in_temp_dir(f: impl FnOnce(&std::path::Path)) {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let _restore = RestoreCwd(original_cwd);
        f(dir.path());
    }

    fn open_volume(image_path: &std::path::Path) -> Volume<AppleDisk> {
        Volume::new(
            AppleDisk::from_file(image_path.to_str().unwrap(), false).unwrap(),
            image_path.to_str().unwrap().to_string(),
        )
        .unwrap()
    }

    #[test]
    fn transfer_to_image_uses_basename_of_a_full_host_path() {
        in_temp_dir(|dir| {
            let image_path = dir.join("scratch.dsk");
            std::fs::copy(fixture_path("empty.dsk"), &image_path).unwrap();
            let sub_dir = dir.join("sub");
            std::fs::create_dir(&sub_dir).unwrap();
            let host_path = sub_dir.join("NESTED.TEXT");
            std::fs::write(&host_path, "from a nested path\n").unwrap();

            let mut volume = open_volume(&image_path);
            // Pass a full path (not just a bare filename in cwd) -- only its
            // basename should end up as the volume's directory entry name,
            // since full paths routinely exceed the 15-character limit.
            volume
                .transfer(host_path.to_str().unwrap(), true, true, false)
                .unwrap();

            let reopened = open_volume(&image_path);
            let entry = &reopened.directory.entries[0];
            assert_eq!(from_length_prefixed(&entry.name), "NESTED.TEXT");
        });
    }

    #[test]
    fn transfer_to_image_uppercases_the_volume_name() {
        in_temp_dir(|dir| {
            let image_path = dir.join("scratch.dsk");
            std::fs::copy(fixture_path("empty.dsk"), &image_path).unwrap();
            std::fs::write("FeatureDemo.pas", "lowercase host filename\n").unwrap();

            let mut volume = open_volume(&image_path);
            // The Filer can display a lowercase name, but the Editor and
            // other system tools don't handle them well -- volume names
            // should always be uppercased regardless of host file casing.
            volume
                .transfer("FeatureDemo.pas", true, true, false)
                .unwrap();

            let reopened = open_volume(&image_path);
            let entry = &reopened.directory.entries[0];
            assert_eq!(from_length_prefixed(&entry.name), "FEATUREDEMO.PAS");
        });
    }

    #[test]
    fn transfer_to_image_rejects_filename_over_15_chars_with_clear_message() {
        in_temp_dir(|dir| {
            let image_path = dir.join("scratch.dsk");
            std::fs::copy(fixture_path("empty.dsk"), &image_path).unwrap();
            // 16 characters -- one over the p-System limit.
            std::fs::write("SIXTEEN_CHARS.TX", "hi\n").unwrap();

            let mut volume = open_volume(&image_path);
            let err = volume
                .transfer("SIXTEEN_CHARS.TX", true, true, false)
                .unwrap_err();
            let message = err.to_string();
            assert!(message.contains("SIXTEEN_CHARS.TX"));
            assert!(message.contains("15 characters"));
        });
    }

    #[test]
    fn transfer_to_image_keeps_entries_sorted_by_first_block() {
        in_temp_dir(|dir| {
            let image_path = dir.join("scratch.dsk");
            std::fs::copy(fixture_path("blog.dsk"), &image_path).unwrap();
            // blog.dsk's WORK.TEXT ends at block 16 and MAKEFILES.TEXT
            // starts at block 30 -- a gap earlier than the volume's tail,
            // reproducing the scenario that exposed out-of-order entries.
            std::fs::write("MIDDLE.TEXT", "fits in the gap after WORK.TEXT\n").unwrap();

            let mut volume = open_volume(&image_path);
            volume.transfer("MIDDLE.TEXT", true, true, false).unwrap();

            let reopened = open_volume(&image_path);
            let num_files = reopened.directory.volume.num_files as usize;
            let blocks: Vec<u16> = reopened.directory.entries[..num_files]
                .iter()
                .map(|e| e.first_block)
                .collect();
            let mut sorted = blocks.clone();
            sorted.sort();
            assert_eq!(
                blocks, sorted,
                "directory entries must stay in ascending first_block order"
            );
        });
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
            // Real Apple Pascal always reports a full block here for text
            // files regardless of actual content length -- confirmed
            // against every text file on tests/AppleDsks/blog.dsk and a
            // real Editor-authored 0-byte text file on experiments4.dsk.
            assert_eq!(entry.bytes_in_last_block, 512);

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
    fn transfer_from_image_errors_when_name_not_found() {
        in_temp_dir(|dir| {
            let image_path = dir.join("scratch.dsk");
            std::fs::copy(fixture_path("empty.dsk"), &image_path).unwrap();

            let mut volume = open_volume(&image_path);
            // Forgetting --to-image (to_image: false) on an empty volume
            // used to silently succeed without writing anything.
            let err = volume
                .transfer("NOTHERE.TEXT", false, true, false)
                .unwrap_err();
            assert!(err.to_string().contains("NOTHERE.TEXT"));
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
            assert!(volume.transfer("BIGFILE.DATA", true, false, false).is_err());
        });
    }

    #[test]
    fn remove_deletes_the_entry_and_persists_it() {
        in_temp_dir(|dir| {
            let image_path = dir.join("scratch.dsk");
            std::fs::copy(fixture_path("blog.dsk"), &image_path).unwrap();

            let mut volume = open_volume(&image_path);
            let names_before: Vec<String> = {
                let num_files = volume.directory.volume.num_files as usize;
                volume.directory.entries[..num_files]
                    .iter()
                    .map(|e| from_length_prefixed(&e.name))
                    .collect()
            };
            assert!(names_before.contains(&"WORK.TEXT".to_string()));

            volume.remove("WORK.TEXT").unwrap();

            let reopened = open_volume(&image_path);
            let num_files = reopened.directory.volume.num_files as usize;
            assert_eq!(num_files, names_before.len() - 1);
            let names_after: Vec<String> = reopened.directory.entries[..num_files]
                .iter()
                .map(|e| from_length_prefixed(&e.name))
                .collect();
            assert!(!names_after.contains(&"WORK.TEXT".to_string()));
            // Removing WORK.TEXT (the first entry) shouldn't disturb the
            // relative order of the remaining entries.
            assert_eq!(names_after, names_before[1..]);
        });
    }

    #[test]
    fn remove_errors_when_name_not_found() {
        in_temp_dir(|dir| {
            let image_path = dir.join("scratch.dsk");
            std::fs::copy(fixture_path("blog.dsk"), &image_path).unwrap();

            let mut volume = open_volume(&image_path);
            let err = volume.remove("NOTHERE.TEXT").unwrap_err();
            assert!(err.to_string().contains("NOTHERE.TEXT"));
        });
    }

    #[test]
    fn change_renames_the_file_and_it_stays_readable_under_the_new_name() {
        in_temp_dir(|dir| {
            let image_path = dir.join("scratch.dsk");
            std::fs::copy(fixture_path("blog.dsk"), &image_path).unwrap();

            let mut volume = open_volume(&image_path);
            volume.change("SHORT.TEXT", "RENAMED.TEXT").unwrap();

            let mut reopened = open_volume(&image_path);
            // The old name is gone...
            assert!(reopened.transfer("SHORT.TEXT", false, true, false).is_err());
            // ...but the file is still readable under its new name.
            reopened
                .transfer("RENAMED.TEXT", false, true, false)
                .unwrap();
            assert!(std::fs::read_to_string("RENAMED.TEXT").is_ok());
        });
    }

    #[test]
    fn change_uppercases_the_target_name_and_rejects_over_length_names() {
        in_temp_dir(|dir| {
            let image_path = dir.join("scratch.dsk");
            std::fs::copy(fixture_path("blog.dsk"), &image_path).unwrap();

            let mut volume = open_volume(&image_path);
            volume.change("SHORT.TEXT", "lowercase.text").unwrap();
            let num_files = volume.directory.volume.num_files as usize;
            assert!(
                volume.directory.entries[..num_files]
                    .iter()
                    .any(|e| from_length_prefixed(&e.name) == "LOWERCASE.TEXT")
            );

            let err = volume
                .change("SHORT2.TEXT", "SIXTEEN_CHARS.TXT")
                .unwrap_err();
            assert!(err.to_string().contains("15 characters"));
        });
    }

    #[test]
    fn krunch_closes_gaps_while_keeping_file_contents_intact() {
        in_temp_dir(|dir| {
            let image_path = dir.join("scratch.dsk");
            std::fs::copy(fixture_path("blog.dsk"), &image_path).unwrap();

            // Capture WORK.TEXT's content before krunching so it can be
            // compared against what's readable afterwards.
            let mut before = open_volume(&image_path);
            before.transfer("WORK.TEXT", false, true, false).unwrap();
            let work_before = std::fs::read_to_string("WORK.TEXT").unwrap();
            std::fs::remove_file("WORK.TEXT").unwrap();

            let mut volume = open_volume(&image_path);
            volume.krunch().unwrap();

            let mut reopened = open_volume(&image_path);
            let num_files = reopened.directory.volume.num_files as usize;
            // Every entry should now immediately follow the previous one
            // (or the directory itself, for the first entry) -- no gaps
            // left anywhere.
            let mut cursor = reopened.directory.volume.first_block_after_directory;
            for entry in &reopened.directory.entries[..num_files] {
                assert_eq!(entry.first_block, cursor);
                cursor = entry.first_after_block;
            }

            reopened.transfer("WORK.TEXT", false, true, false).unwrap();
            let work_after = std::fs::read_to_string("WORK.TEXT").unwrap();
            assert_eq!(work_before, work_after);
        });
    }

    #[test]
    fn zero_clears_the_directory() {
        in_temp_dir(|dir| {
            let image_path = dir.join("scratch.dsk");
            std::fs::copy(fixture_path("blog.dsk"), &image_path).unwrap();

            let mut volume = open_volume(&image_path);
            volume.zero(None).unwrap();

            let reopened = open_volume(&image_path);
            assert_eq!(reopened.directory.volume.num_files, 0);
            // No new name was given -- the volume's own name shouldn't
            // have been touched.
            assert_eq!(
                from_length_prefixed(&reopened.directory.volume.volume_name),
                "BLOG"
            );
        });
    }

    #[test]
    fn zero_with_new_name_renames_and_uppercases_the_volume() {
        in_temp_dir(|dir| {
            let image_path = dir.join("scratch.dsk");
            std::fs::copy(fixture_path("blog.dsk"), &image_path).unwrap();

            let mut volume = open_volume(&image_path);
            volume.zero(Some("newvol")).unwrap();

            let reopened = open_volume(&image_path);
            assert_eq!(reopened.directory.volume.num_files, 0);
            assert_eq!(
                from_length_prefixed(&reopened.directory.volume.volume_name),
                "NEWVOL"
            );
        });
    }

    #[test]
    fn zero_rejects_over_length_new_name() {
        in_temp_dir(|dir| {
            let image_path = dir.join("scratch.dsk");
            std::fs::copy(fixture_path("blog.dsk"), &image_path).unwrap();

            let mut volume = open_volume(&image_path);
            // 8 characters -- one over the volume name's 7-character limit
            // (shorter than a file's 15-character limit).
            let err = volume.zero(Some("TOOLONG8")).unwrap_err();
            assert!(err.to_string().contains("7 characters"));

            // The rejected rename shouldn't have partially applied --
            // the directory must still parse with its original name and
            // file count untouched.
            let reopened = open_volume(&image_path);
            assert_eq!(reopened.directory.volume.num_files, 8);
            assert_eq!(
                from_length_prefixed(&reopened.directory.volume.volume_name),
                "BLOG"
            );
        });
    }
}
