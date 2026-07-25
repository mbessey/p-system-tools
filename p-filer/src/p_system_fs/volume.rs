use p_system_format::pascal_string::from_length_prefixed;
use p_system_format::pdate::{pdate_to_string, pdate_to_systime};
use std::fs::File;
use std::io::prelude::*;

use super::directory::Directory;
use super::text::text_from_blocks;
use crate::disk_image::DiskImage;

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

    pub fn remove(&self, name: &str) -> anyhow::Result<()> {
        println!("Removing {name} on {0}", self.image_name);
        Ok(())
    }

    pub fn transfer(
        &self,
        name: &str,
        to_image: bool,
        is_text: bool,
        preserve_date: bool,
    ) -> anyhow::Result<()> {
        if to_image {
            println!("Copying {name} to {0}", self.image_name);
            todo!("Copying to image not implemented yet");
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
                        let _ = filedesc.write(file_buffer);
                    }
                    println!("Wrote {name} to disk");
                    if preserve_date {
                        let _ = filedesc.set_modified(pdate_to_systime(entry.date));
                    }
                    filedesc.sync_all()?;
                    return Ok(());
                }
            }
        }
        Ok(())
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
