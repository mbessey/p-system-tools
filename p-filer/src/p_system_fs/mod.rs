use std::{fs, ptr::read_unaligned};

// Directory entries are each 26 bytes. The first is a bit special, and contains information about the volume itself.
// The rest are the files on the volume. Directory entries occupy blocks 2 through 5 on the disk.
#[derive(Debug)]
#[repr(C)]
struct Directory {
    volume: VolumeInfo,
    entries: [DirectoryEntry; 77],
}

impl Directory {
    fn new(bytes: &[u8]) -> Self {
        let directory_ptr = bytes.as_ptr() as *const Directory;
        let new_self = unsafe {directory_ptr.read_unaligned() };
        return new_self;
    }
}

#[derive(Debug)]
#[repr(C)]
struct VolumeInfo {
    first_system_block: u16, // always zero
    first_block_after_directory: u16, // always 6
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
pub struct DirectoryEntry {
    first_block: u16, // first block of file
    first_after_block: u16, // first block after file (last block + 1)
    file_type: u16, // type of file ()
    name: [u8; 16], // Pascal string - length is first byte
    bytes_in_last_block: u16, // number of bytes in last block
    date: u16, // modified date
}

pub fn pstring_to_string(pstring: &[u8]) -> String {
    let len = pstring[0] as usize;
    let mut result = String::new();
    for i in 1..=len {
        result.push(pstring[i] as char);
    }
    return result;
}

pub fn pdate_to_string(pdate: u16) -> String {
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

pub fn text_from_blocks(buffer: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut skip_next = false;
    for i in 1025..buffer.len() {
        let byte = buffer[i];
        if skip_next {
            skip_next = false;
            continue;
        }
        if byte == 0x0d {
            result.push(0x0a); // convert CR to LF
        } else if byte == 0x10 {
            let space_count = buffer[i+1] as usize - 32;
            for _ in 0..space_count {
                result.push(0x20); // emit spaces for indent
            }
            skip_next = true; // skip the next byte
        } else if byte == 0 {
            continue; // skip null bytes
        } else {
            result.push(byte);
        }
    }
    return result;
}        

pub struct AppleDisk {
    image: String,
    blocks: Vec<u8>,
    directory: Directory,
}

impl AppleDisk {
    pub fn read_blocks(&self, index: usize, count: usize) -> &[u8] {
        let start:usize = index * 512;
        let end:usize = (index + count) * 512;
        return &self.blocks[start..end]
    }

    pub fn num_blocks(&self) -> usize {
        return self.blocks.len() / 512
    }

    pub fn new(name: &str) -> Self {
        let buffer = Self::read_buffer(&name);
        let directory = Directory::new(&buffer[1024..2560]);
        let mut new_self = Self {
            image: name.to_string(),
            blocks: buffer,
            directory: directory
        };
        return new_self;
    }

    fn read_buffer(name: &str) -> Vec<u8> {
        let contents: Vec<u8> = fs::read(&name) .expect("couldn't read file");
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
            //println!("track {track}, offset {track_offset}");
            for sector in 0..16 as usize {
                let sector2 = sector_map[sector];
                //println!("track: {track}, sector {sector2} -> {sector}");
                //let target_sector_offset = sector * 256 + track_offset;
                let source_sector_offset = sector2 * 256 + track_offset;
                //println!("");
                for byte in 0..256 as usize {
                    buffer.push(contents[source_sector_offset+byte]);
                }
            }
        }
        //println!("file len: {}, buffer len: {}", contents.len(), self.buffer.len());
        assert!(contents.len() == buffer.len());
        return buffer;
    }

    pub fn list(&self) {
        println!("Listing files on {0}", self.image);
        println!("First block (should be 0): {}", self.directory.volume.first_system_block);
        println!("First block after directory (should be 6): {}", self.directory.volume.first_block_after_directory);
        println!("File type (should be 0): {}", self.directory.volume.file_type);
        println!("Volume name:      {}", pstring_to_string(&self.directory.volume.volume_name));
        println!("Number of blocks: {}", self.directory.volume.num_blocks);
        println!("Number of files:  {}", self.directory.volume.num_files);
        println!("Last access time: {}", self.directory.volume.last_access_time);
        println!("Date:             {}", pdate_to_string(self.directory.volume.date));
        println!("Reserved:         {:?}", self.directory.volume.reserved);
        for index in 0..self.directory.volume.num_files {
            let entry = &self.directory.entries[index as usize];
            println!("Entry {index}:");
            println!("  First block:         {}", entry.first_block);
            println!("  First block after:   {}", entry.first_after_block);
            println!("  File type:           {}", entry.file_type);
            println!("  Name:                {}", pstring_to_string(&entry.name));
            println!("  Bytes in last block: {}", entry.bytes_in_last_block);
            println!("  Date:                {}", pdate_to_string(entry.date));
        }
    }
    
    pub fn remove(&self, name: &str) {
        println!("Removing {name} on {0}", self.image);
    }
    
    pub fn transfer(&self, name: &str, to_image: bool, is_text: bool) {
        if to_image {
            println!("Copying {name} to {0}", self.image);
            todo!("Copying to image not implemented yet");
        } else {
            println!("Copying {name} from {0}", self.image);
            for entry in &self.directory.entries {
                let entry_name = pstring_to_string(&entry.name);
                if entry_name == name {
                    println!("Found {name} at block {0}", entry.first_block);
                    let file_buffer = self.read_blocks(entry.first_block as usize, entry.first_after_block as usize - entry.first_block as usize);
                    let file_name = format!("{name}");
                    if is_text {
                        let text_buffer = text_from_blocks(file_buffer);
                        fs::write(file_name, text_buffer).expect("Unable to write text file");
                    } else {
                        fs::write(file_name, file_buffer).expect("Unable to write binary file");
                    }
                    println!("Wrote {name} to disk");
                    return;
                }
            }
        }
    }
    
    pub fn change(&self, from: &str, to: &str) {
        println!("Renaming {from} to {to} on {0}", self.image);
        
    }
    
    pub fn krunch(&self) {
        println!("Consolidating free space on {0}", self.image);
    }
    
    pub fn zero(&self) {
        println!("Clearing directory on {0}", self.image);
    }

    pub fn dump(&self, from: usize, to: usize) {
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
