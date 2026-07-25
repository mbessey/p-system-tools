use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::time::SystemTime;
use chrono::prelude::*;

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

struct PdateYDM {
    // These types picked to be friendly for conversion to system time.
    year: i32,
    day: u32,
    month: u32,
}

fn normalize_pdate_year(pdate: u16) -> i32
{
    let offset = ((pdate & 0xfe00) >> 9) as i32;

    // This logic assumes "offset" is between 0-100.  If it's ever > 100,
    // We'll have overlap in 2001-2027
    if offset < 70 {
        // If before 1970, assume it's 20xx.
        return offset + 2000;
    }

    return offset + 1900;
}

impl PdateYDM {
    fn new(pdate: u16) -> PdateYDM {
        PdateYDM {
            year: normalize_pdate_year(pdate),
            day: ((pdate & 0x01f0) >> 4) as u32,
            month: (pdate & 0x0F) as u32,
        }
    }
}

pub fn pdate_to_systime(pdate: u16) -> SystemTime {
    let ydm = PdateYDM::new(pdate);

    // Meanwhile, since we only get day (not time) we will set it to 0000
    // in whatever timezone TZ is set to. This may cause off-by-one-day
    // problems in the timestamp.

    return SystemTime::from(
        Local.with_ymd_and_hms(ydm.year, ydm.month, ydm.day, 0, 0, 0).unwrap());
}

pub fn pdate_to_string(pdate: u16) -> String {
    let ydm = PdateYDM::new(pdate);

    return format!("{:04}-{:02}-{:02}", ydm.year, ydm.month, ydm.day);
}

pub fn text_from_blocks(buffer: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut skip_next = false;
    for i in 1024..buffer.len() {
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
        Self {
            image: name.to_string(),
            blocks: buffer,
            directory: directory
        }
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

    pub fn transfer(&self, name: &str, to_image: bool, is_text: bool,
        preserve_date: bool) {
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
                    // Because we want to possibly use set_times, we'll
                    // have to use more conventional File:: methods.
                    let mut filedesc = File::create(file_name).expect("create failed");
                    if is_text {
                        let text_buffer = text_from_blocks(file_buffer);
                        let _ = filedesc.write(text_buffer.as_slice());
                    } else {
                        let _ = filedesc.write(file_buffer);
                    }
                    println!("Wrote {name} to disk");
                    if preserve_date {
                        let _ =
                            filedesc.set_modified(pdate_to_systime(entry.date));
                    }
                    filedesc.sync_all().expect("Cannot commit to file");
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

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path(name: &str) -> String {
        format!("{}/../tests/AppleDsks/{name}", env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn empty_disk_directory() {
        let d = AppleDisk::new(&fixture_path("empty.dsk"));
        assert_eq!(d.directory.volume.num_files, 0);
        assert_eq!(pstring_to_string(&d.directory.volume.volume_name), "WORK");
        assert_eq!(d.directory.volume.num_blocks, 280);
        assert_eq!(pdate_to_string(d.directory.volume.date), "1984-11-07");
        assert_eq!(d.num_blocks(), 280);
    }

    #[test]
    fn manyfiles_disk_directory() {
        let d = AppleDisk::new(&fixture_path("manyfiles.dsk"));
        assert_eq!(d.directory.volume.num_files, 76);

        let entry0 = &d.directory.entries[0];
        assert_eq!(pstring_to_string(&entry0.name), "DATAFILE01.DATA");
        assert_eq!(entry0.first_block, 6);
        assert_eq!(entry0.first_after_block, 9);

        // DATAFILE11 was deleted, so entry 10 is DATAFILE12 -- this checks
        // parsing doesn't assume contiguous file numbering.
        let entry10 = &d.directory.entries[10];
        assert_eq!(pstring_to_string(&entry10.name), "DATAFILE12.DATA");
        assert_eq!(entry10.first_block, 39);

        let entry75 = &d.directory.entries[75];
        assert_eq!(pstring_to_string(&entry75.name), "DATAFILE77.DATA");
        assert_eq!(entry75.first_block, 234);
        assert_eq!(entry75.first_after_block, 237);
    }

    #[test]
    fn blog_disk_directory() {
        let d = AppleDisk::new(&fixture_path("blog.dsk"));
        assert_eq!(d.directory.volume.num_files, 8);
        let expected_names = [
            "WORK.TEXT", "MAKEFILES.TEXT", "FILESYSTEM.TEXT", "EDITOR.TEXT",
            "SHORT.TEXT", "SHORT2.TEXT", "INDENTS.TEXT", "INDENT.TEXT",
        ];
        for (i, expected_name) in expected_names.iter().enumerate() {
            let entry = &d.directory.entries[i];
            assert_eq!(pstring_to_string(&entry.name), *expected_name);
            assert_eq!(entry.file_type, 3);
        }
    }

    #[test]
    fn blog_disk_text_simple() {
        let d = AppleDisk::new(&fixture_path("blog.dsk"));
        // SHORT.TEXT: first_block=148, first_after_block=152
        let file_buffer = d.read_blocks(148, 4);
        let text = text_from_blocks(file_buffer);
        assert_eq!(
            String::from_utf8(text).unwrap(),
            "This is about as simple as it gets.\nA couple of lines,\n\nAnd two paragraphs.\n"
        );
    }

    #[test]
    fn blog_disk_text_indented() {
        let d = AppleDisk::new(&fixture_path("blog.dsk"));
        // INDENTS.TEXT: first_block=156, first_after_block=160. Exercises the
        // run-length-encoded indentation (0x10 marker) decode path.
        let file_buffer = d.read_blocks(156, 4);
        let text = text_from_blocks(file_buffer);
        let expected = "This is about as simple as it gets.\n\
             \u{20}A couple of lines.\n\
             \u{20}\u{20}Each further indented,\n\
             \u{20}\u{20}\u{20}Slowly,\n\
             \u{20}\u{20}\u{20}\u{20}Inexorably,\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}Approaching the right margin\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}I guess at the limit, we'd hit 'p'\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}But I don't have the patience...\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}Eight\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}Nine\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}Ten\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}Eleven\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}Twelve\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}Thirteen\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}Fourteen\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}Fifteen\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}Sixteen\n\
             \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\u{20}\n";
        assert_eq!(String::from_utf8(text).unwrap(), expected);
    }
}
