use clap::{Args, Parser, Subcommand};

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

#[repr(i32)]
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
        println!("Segment {:#x?}, name: {}, address: {:#x?}, length: {:#x?}, kind: {:?}", s, string_from(&seg_name), code_info.address*512, code_info.length, seg_kind);
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
