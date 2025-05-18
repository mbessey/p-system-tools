use clap::{builder::Str, Args, Parser, Subcommand};

/// A command-file tool for manipulating UCSD pascal object files
#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct MainArgs {
    /// Name of disk image to use
    #[arg(short, long)]
    code_file: String,
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand)]
enum Commands {
    List,
    Disassemble,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct CodeInfo {
    address: u16,
    length: u16,
}

#[repr(u16)]
#[derive(Debug, Clone, Copy)]
enum SegmentKind {
    Linked,
    HostSegment,
    SegmentProcedure,
    UnitSegment,
    SeparateSegment,
    UnlinkedIntrinsic,
    LinkedIntrinsic,
    DataSegment
}

#[derive(Debug)]
#[repr(C)]
struct SegmentDictionary {
    code_info: [ CodeInfo; 16],
    seg_name: [[u8; 8]; 16],
    seg_kind: [SegmentKind; 16],
    text_addr: [u16; 16],
    seg_info: [u16; 16],
    // This is then followed by "library information", which is described thus:
    // Library information of undefined format occupies most of the remainder of the segment dictionary block.
    // That's...great.
}

impl SegmentDictionary {
    fn new(bytes: &[u8]) -> Self {
        let directory_ptr = bytes.as_ptr() as *const SegmentDictionary;
        let new_self = unsafe {directory_ptr.read_unaligned() };
        return new_self;
    }
}

fn main() {
    let args = MainArgs::parse();
    let file_name = args.code_file;
    match &args.command {
        Commands::List => list(file_name),
        Commands::Disassemble => disassemble(file_name),
    }
}

fn list(file_name: String) {
    println!("Listing code file {file_name}");
    let contents = std::fs::read(file_name).expect("Unable to read file");
    let segment_dictionary = SegmentDictionary::new(&contents);
    println!("File length: {}", contents.len());
    println!("Segments:");
    for s in 0..16 {
        let code_info = segment_dictionary.code_info[s];
        if code_info.address == 0 {
            continue;
        }
        let seg_name = segment_dictionary.seg_name[s];
        let seg_kind = segment_dictionary.seg_kind[s];
        let text_addr = segment_dictionary.text_addr[s];
        let seg_info = segment_dictionary.seg_info[s];

        println!("Segment {:#x?}, name: {}, address: {:#x?}, length: {:#x?}, kind: {:?}, text_addr: {:#x?}, seg_info: {:#x?}", s, string_from(&seg_name), code_info.address*512, code_info.length, seg_kind, text_addr, string_from_segment_info(seg_info));
    }
    println!();
}

fn disassemble(file_name: String) {
    println!("Disassembling code file {file_name}");
}

fn string_from(pascalString8: &[u8;8]) -> String {
    let mut result = String::new();
    for c in pascalString8 {
        if *c == 0x20 {
            break;
        }
        result.push(*c as char);
    }
    return result;
}

fn string_from_segment_info(segment_info: u16) -> String {
    let unit = segment_info & 0xff;
    let code_type = (segment_info & 0x0f00) >> 8;
    let type_s = match code_type {
        0 => "Unknown",
        1 => "Pcode Big-endian",
        2 => "Pcode Little-endian",
        _ => "Native code"
    };
    let version = (segment_info & 0xe000) >> 13;    
    let result = format!("[unit: {}, type: {}, version: {}]", unit, type_s, version);
    return result;
}