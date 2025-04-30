use core::num;
use std::{fs, u32, usize};
use clap::{Args, Parser, Subcommand};

/// A command-file tool for manipulating Apple Pascal disk images
#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct MainArgs {
    /// Name of disk image to use
    #[arg(short, long)]
    image: String,
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand)]
enum Commands {
    List,
    Remove {name: String},
    Transfer(TransferArgs),
    Change {from: String, to: String},
    Krunch,
    Zero,
    Dump {from: usize, to: usize} 
}

#[derive(Args, Debug)]
struct TransferArgs {
    name: String,
    #[arg(short, long)]
    to_image: bool
}

fn main() {
    let args = MainArgs::parse();
    let image = args.image;
    let d = AppleDisk::new(&image);
    match &args.command {
        Commands::List => d.list(),
        Commands::Remove { name } => d.remove(name),
        Commands::Transfer(args, ) => d.transfer(&args.name, args.to_image),
        Commands::Change { from, to } => d.change(from, to),
        Commands::Krunch => d.krunch(),
        Commands::Zero => d.zero(),
        Commands::Dump { from, to } => d.dump(*from, *to)
    }
}

struct AppleDisk {
    image: String,
    buffer: Vec<u8>,
}

impl AppleDisk {
    pub fn read_blocks(&self, index: usize, count: usize) -> &[u8] {
        let start:usize = index * 512;
        let end:usize = (index + count) * 512;
        return &self.buffer[start..end]
    }

    pub fn num_blocks(&self) -> usize {
        return self.buffer.len() / 512
    }

    pub fn new(name: &str) -> Self {
        let mut new_self = Self {
            image: name.to_string(),
            buffer: Vec::new(),
        };
        new_self.fill_buffer();
        return new_self;
    }

    fn fill_buffer(& mut self) {
        //TODO: Don't assume we have enough memory to read the whole file at once here (but we totally do on the desktop)
        let contents: Vec<u8> = fs::read(&self.image) .expect("couldn't read file");
        self.buffer = Vec::with_capacity(contents.len());
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
            //println!("track {track}, offset {track_offset}");
            for sector in 0..16 as usize {
                let sector2 = sector_map[sector];
                //println!("track: {track}, sector {sector2} -> {sector}");
                //let target_sector_offset = sector * 256 + track_offset;
                let source_sector_offset = sector2 * 256 + track_offset;
                //println!("");
                for byte in 0..256 as usize {
                    self.buffer.push(contents[source_sector_offset+byte]);
                }
            }
        }
        //println!("file len: {}, buffer len: {}", contents.len(), self.buffer.len());
        assert!(contents.len() == self.buffer.len());
    }

    fn list(&self) {
        println!("Listing files on {0}", self.image);
        let buffer = self.read_blocks(2, 4);
        let directory_ptr = buffer.as_ptr() as *const Directory;
        let directory = unsafe {directory_ptr.read_unaligned() };
        println!("First block (should be 0): {}", directory.volume.first_system_block);
        println!("First directory block (should be 6): {}", directory.volume.first_directory_block);
        println!("File type (should be 0): {}", directory.volume.file_type);
        println!("Volume name:      {}", pstring_to_string(&directory.volume.volume_name));
        println!("Number of blocks: {}", directory.volume.num_blocks);
        println!("Number of files:  {}", directory.volume.num_files);
        println!("Last access time: {}", directory.volume.last_access_time);
        println!("Date:             {}", pdate_to_string(directory.volume.date));
        println!("Reserved:         {:?}", directory.volume.reserved);
        for index in 0..directory.volume.num_files {
            let entry = &directory.entries[index as usize];
            println!("Entry {index}:");
            println!("  First block:         {}", entry.first_block);
            println!("  First block after:   {}", entry.first_after_block);
            println!("  File type:           {}", entry.file_type);
            println!("  Name:                {}", pstring_to_string(&entry.name));
            println!("  Bytes in last block: {}", entry.bytes_in_last_block);
            println!("  Date:                {}", pdate_to_string(entry.date));
        }
    }
    
    fn remove(&self, name: &str) {
        println!("Removing {name} on {0}", self.image);
    }
    
    fn transfer(&self, name: &str, to_image: bool) {
        if to_image {
            println!("Copying {name} to {0}", self.image);
            todo!("Copying to image not implemented yet");
        } else {
            let buffer = self.read_blocks(2, 4);
            let directory_ptr = buffer.as_ptr() as *const Directory;
            let directory = unsafe {directory_ptr.read_unaligned() };
            println!("Copying {name} from {0}", self.image);
            for entry in &directory.entries {
                let entry_name = pstring_to_string(&entry.name);
                if entry_name == name {
                    println!("Found {name} at block {0}", entry.first_block);
                    let file_buffer = self.read_blocks(entry.first_block as usize, entry.first_after_block as usize - entry.first_block as usize);
                    let file_name = format!("{name}");
                    fs::write(file_name, file_buffer).expect("Unable to write file");
                    println!("Wrote {name} to disk");
                    return;
                }
            }
        }
    }
    
    fn change(&self, from: &str, to: &str) {
        println!("Renaming {from} to {to} on {0}", self.image);
        
    }
    
    fn krunch(&self) {
        println!("Consolidating free space on {0}", self.image);
    }
    
    fn zero(&self) {
        println!("Clearing directory on {0}", self.image);
    }

    fn dump(&self, from: usize, to: usize) {
        if from > to {
            panic!("from ({from}) must be less than to ({to})");
        }
        if to > self.num_blocks() {
            panic!("to ({to}) must be less than {0} blocks", self.num_blocks());
        }
        if from > self.num_blocks() {
            panic!("from ({from}) must be less than {0} blocks", self.num_blocks());
        }
        println!("Dumping contexts of {0} from block {1} to {2}", self.image, from, to);
        let line_len = 16;
        for block_no in from..=to {
            let block = self.read_blocks(block_no, 1);
            for line in 0..512/line_len {
                let offset: usize = block_no * 512 + line * line_len;
                print!("{:06x}  ", offset);
                for byte in 0..line_len {
                    let val = block[byte + line * line_len];
                    print!("{:02x} ", val);
                }
                print!("  |");
                for byte in 0..line_len {
                    let mut c = block[byte + line * line_len];
                    if c < 32 || c > 126 {
                        c = 46;
                    }
                    print!("{}", char::from(c));
                }
                println!("|");
            }
            println!("")
        }
    }
}

// Directory entries are each 26 bytes. The first is a bit special, and contains information about the volume itself.
// The rest are the files on the volume. Directory entries occupy blocks 2 through 5 on the disk.
#[derive(Debug)]
#[repr(C)]
struct Directory {
    volume: VolumeInfo,
    entries: [DirectoryEntry; 77],
}

#[derive(Debug)]
#[repr(C)]
struct VolumeInfo {
    first_system_block: u16, // always zero
    first_directory_block: u16, // always 6
    file_type: u16, // always zero
    volume_name: [u8; 8], // Pascal string - length is first byte
    num_blocks: u16, // number of blocks in volume
    num_files: u16, // number of files in directory
    last_access_time: u16, // last access time - always zero?
    date: u16, // date set by user
    reserved: [u8; 4], // reserved for future use
}

#[derive(Debug)]
#[repr(C)]
struct DirectoryEntry {
    first_block: u16, // first block of file
    first_after_block: u16, // first block after file (last block + 1)
    file_type: u16, // type of file ()
    name: [u8; 16], // Pascal string - length is first byte
    bytes_in_last_block: u16, // number of bytes in last block
    date: u16, // modified date
}

fn pstring_to_string(pstring: &[u8]) -> String {
    let len = pstring[0] as usize;
    let mut result = String::new();
    for i in 1..=len {
        result.push(pstring[i] as char);
    }
    return result;
}

fn pdate_to_string(pdate: u16) -> String {
    let mut year = (pdate & 0xfe00) >> 9;
    let day = (pdate & 0x01f0) >> 4;
    let month = pdate & 0x0F;
    // year is 0-100, historically offset from 1900. Consider years "earlier" than 1970 to be 21st century
    if year < 70 {
        year += 2000;
    } else {
        year += 1900;
    }
    return format!("{:04}-{:02}-{:02}", year, month, day);
}
