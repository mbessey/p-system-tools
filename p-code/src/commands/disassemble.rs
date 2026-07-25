use crate::disassembler;
use crate::segment_dictionary::SegmentDictionary;
use std::fmt::Write as _;

pub fn run(file_name: String, show_bytes: bool) -> anyhow::Result<()> {
    println!("Disassembling code file {file_name}");
    let contents = std::fs::read(&file_name)?;
    let segment_dictionary = SegmentDictionary::parse(&contents)?;
    for (s, code_info, seg_name) in segment_dictionary.active_segments() {
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
        if show_bytes {
            println!("  (offset within segment; segment starts at file offset {start:#x})");
        }
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
                    print_instructions(code, proc.enter_ic, Some(stop_after), show_bytes);
                }
            }
            None => {
                println!("  (couldn't parse procedure dictionary; showing raw decode)");
                print_instructions(segment_bytes, 0, None, show_bytes);
            }
        }
        println!();
    }
    Ok(())
}

fn print_instructions(
    code: &[u8],
    base_offset: usize,
    stop_after: Option<usize>,
    show_bytes: bool,
) {
    let mut instrs = disassembler::disassemble(code);
    if let Some(stop_at) = stop_after {
        if let Some(last) = instrs.iter().position(|i| stop_at < i.offset + i.bytes_len) {
            instrs.truncate(last + 1);
        }
    }
    for instr in instrs {
        let extra = match (instr.mnemonic, &instr.operand) {
            (disassembler::Mnemonic::CSP, disassembler::Operand::U8(sub)) => {
                disassembler::csp_name(*sub)
                    .map(|n| format!("  {{{n}}}"))
                    .unwrap_or_default()
            }
            _ => String::new(),
        };
        if show_bytes {
            let raw = instr.raw_bytes(code);
            let mut hex = String::with_capacity(raw.len() * 3);
            for b in raw {
                let _ = write!(hex, "{b:02x} ");
            }
            println!(
                "    {:04x}  {:<18} {:?}  {}{}",
                base_offset + instr.offset,
                hex,
                instr.mnemonic,
                format_operand(instr.mnemonic, &instr.operand),
                extra
            );
        } else {
            println!(
                "{:?}  {}{}",
                instr.mnemonic,
                format_operand(instr.mnemonic, &instr.operand),
                extra
            );
        }
    }
}

fn format_operand(mnemonic: disassembler::Mnemonic, operand: &disassembler::Operand) -> String {
    use disassembler::{Mnemonic, Operand};
    match operand {
        Operand::None => String::new(),
        Operand::Embedded(v) => format!("{v}"),
        Operand::U8(v) => format!("{v}"),
        Operand::I8(v) => format!("{v}"),
        Operand::Big(v) => format!("{v}"),
        Operand::U8Big(a, b) => format!("{a},{b}"),
        Operand::U8U8(a, b) => format!("{a},{b}"),
        Operand::Word(v) => format!("{v}"),
        Operand::TypeCompare(t, None) => format!("{t}"),
        Operand::TypeCompare(t, Some(b)) => format!("{t},{b}"),
        // LSA loads the address of a string constant -- show it as text.
        Operand::StringData(bytes) if mnemonic == Mnemonic::LSA => {
            format!("{:?}", String::from_utf8_lossy(bytes))
        }
        // Other block-argument opcodes (currently just LPA, a packed
        // constant array) aren't text -- show the raw byte values.
        Operand::StringData(bytes) => bytes
            .iter()
            .map(|b| b.to_string())
            .collect::<Vec<_>>()
            .join(","),
        Operand::WordData(bytes) => bytes
            .chunks_exact(2)
            .map(|w| u16::from_le_bytes([w[0], w[1]]).to_string())
            .collect::<Vec<_>>()
            .join(","),
        Operand::CaseJump {
            low,
            high,
            default,
            offsets,
        } => {
            format!("{low}..{high} default {default} table {offsets:?}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use disassembler::{Mnemonic, Operand};

    #[test]
    fn lsa_renders_as_quoted_string() {
        let operand = Operand::StringData(b"Enter your name:".to_vec());
        assert_eq!(
            format_operand(Mnemonic::LSA, &operand),
            "\"Enter your name:\""
        );
    }

    #[test]
    fn lpa_renders_as_comma_separated_bytes() {
        let operand = Operand::StringData(vec![1, 2, 255]);
        assert_eq!(format_operand(Mnemonic::LPA, &operand), "1,2,255");
    }

    #[test]
    fn ldc_renders_as_comma_separated_words() {
        // little-endian word pairs: 0x0001, 0x0002, 0xffff
        let operand = Operand::WordData(vec![0x01, 0x00, 0x02, 0x00, 0xff, 0xff]);
        assert_eq!(format_operand(Mnemonic::LDC, &operand), "1,2,65535");
    }
}
