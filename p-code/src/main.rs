use clap::{Parser, Subcommand};
mod disassembler;

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
#[allow(dead_code)]
enum SegmentKind {
    Linked,             // A ready-to-run program
    HostSegment,        // The outer block of a Pascal program, if it has unresolved references
    SegmentProcedure,   // Not used.
    UnitSegment,        // A Unit, ready to be linked
    SeparateSegment,    // Native-code segment
    UnlinkedIntrinsic,  // An Intrinsic unit with unresolved references
    LinkedIntrinsic,    // An Intrinsic unit
    DataSegment         // Data segment - data stored on the stack, used for some intrinsics
}

#[derive(Debug)]
#[repr(C)]
struct SegmentDictionary {
    code_info: [ CodeInfo; 16],     // one for each of 16 segments
    seg_name: [[u8; 8]; 16],        // 8 charcters, space-padded
    seg_kind: [SegmentKind; 16],    // one for each of 16 segments
    text_addr: [u16; 16],           // For Units, this points to the Interface section
    seg_info: [u16; 16],            // A bitfield for each segment
    intrinsic_segments: u32,        // One bit for each segment in System.Library
    // This is "library information", which is described by the Apple Pascal manual thus:
    // Library information of undefined format occupies most of the remainder of the segment dictionary block.
    // That's...great. I guess we'll figure that out when/if it comes up
    library_info: [u8; 140],
    copyright_string: [u8; 80],     // Copyright, as set by (*$C *), seems to be zero-terminated
}

impl SegmentDictionary {
    fn new(bytes: &[u8]) -> Self {
        let directory_ptr = bytes.as_ptr() as *const SegmentDictionary;
        let new_self = unsafe {directory_ptr.read_unaligned() };
        return new_self;
    }
}

fn main() {
    println!("size of SegmentDictionary is {}", std::mem::size_of::<SegmentDictionary>() );
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
    let copyright = match String::from_utf8(segment_dictionary.copyright_string.to_vec()) {
        Ok(v) => v,
        Err(e) => panic!("{}", e),
    };
    println!("Copyright: {}", copyright);
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

        println!("Segment {:#x?}, name: {}, address: {:#x?}, length: {:#x?},", s, string_from(&seg_name), code_info.address*512, code_info.length);
        println!("\t kind: {:?}, text_addr: {:#x?}, seg_info: {:#x?}", seg_kind, text_addr, string_from_segment_info(seg_info));
    }
    println!();
}

fn disassemble(file_name: String) {
    println!("Disassembling code file {file_name}");
    let contents = std::fs::read(&file_name).expect("Unable to read file");
    let segment_dictionary = SegmentDictionary::new(&contents);
    for s in 0..16 {
        let code_info = segment_dictionary.code_info[s];
        if code_info.address == 0 {
            continue;
        }
        let seg_name = string_from(&segment_dictionary.seg_name[s]);
        let start = code_info.address as usize * 512;
        let end = start + code_info.length as usize;
        if end > contents.len() {
            println!(
                "Segment {s} ({seg_name}): invalid bounds (start={start:#x}, end={end:#x}, file length={:#x}), skipping",
                contents.len()
            );
            continue;
        }
        let segment_bytes = &contents[start..end];

        println!("Segment {s} ({seg_name}):");
        match disassembler::parse_procedure_dictionary(segment_bytes) {
            Some(dict) => {
                println!("  (SEGTABLE slot {})", dict.segment_number);
                for proc in &dict.procedures {
                    println!(
                        "  Procedure {} (lex level {}, param size {}, data size {}, exit at {:04x}):",
                        proc.number, proc.lex_level, proc.param_size, proc.data_size, proc.exit_ic
                    );
                    let code = &segment_bytes[proc.enter_ic..proc.code_end];
                    // exit_ic is the verified offset of the procedure's real
                    // final instruction; code_end can run a little past it
                    // (alignment padding, or an undecoded jump table), so
                    // stop printing once that instruction has been shown
                    // rather than showing whatever comes after it as if it
                    // were real code.
                    let stop_after = proc.exit_ic.saturating_sub(proc.enter_ic);
                    print_instructions(code, proc.enter_ic, Some(stop_after));
                }
            }
            None => {
                println!("  (couldn't parse procedure dictionary; showing raw decode)");
                print_instructions(segment_bytes, 0, None);
            }
        }
        println!();
    }
}

fn print_instructions(code: &[u8], base_offset: usize, stop_after: Option<usize>) {
    let mut instrs = disassembler::disassemble(code);
    if let Some(stop_at) = stop_after {
        if let Some(last) = instrs.iter().position(|i| stop_at < i.offset + i.bytes_len) {
            instrs.truncate(last + 1);
        }
    }
    for instr in instrs {
        let hex: String = instr.raw_bytes(code).iter().map(|b| format!("{b:02x} ")).collect();
        let extra = match (instr.mnemonic, &instr.operand) {
            (disassembler::Mnemonic::CSP, disassembler::Operand::U8(sub)) => {
                disassembler::csp_name(*sub).map(|n| format!("  {{{n}}}")).unwrap_or_default()
            }
            _ => String::new(),
        };
        println!(
            "    {:04x}  {:<18} {:?}  {}{}",
            base_offset + instr.offset,
            hex,
            instr.mnemonic,
            format_operand(&instr.operand),
            extra
        );
    }
}

fn format_operand(operand: &disassembler::Operand) -> String {
    use disassembler::Operand::*;
    match operand {
        None => String::new(),
        Embedded(v) => format!("{v}"),
        U8(v) => format!("{v}"),
        I8(v) => format!("{v}"),
        Big(v) => format!("{v}"),
        U8Big(a, b) => format!("{a},{b}"),
        U8U8(a, b) => format!("{a},{b}"),
        Word(v) => format!("{v}"),
        TypeCompare(t, Option::None) => format!("{t}"),
        TypeCompare(t, Some(b)) => format!("{t},{b}"),
        StringData(bytes) => format!("{:?}", String::from_utf8_lossy(bytes)),
        WordData(bytes) => format!("{} words", bytes.len() / 2),
        CaseJump { low, high, default, offsets } => {
            format!("{low}..{high} default {default} table {offsets:?}")
        }
    }
}

fn string_from(pascal_string8: &[u8;8]) -> String {
    let mut result = String::new();
    for c in pascal_string8 {
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