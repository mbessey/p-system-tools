// Implements `p-code disassemble`: walks a codefile's active segments,
// decodes each procedure's p-code instructions, and prints them in a
// human-readable listing. Beyond raw decoding (handled by
// `disassembler::decode`), this module is responsible for resolving jump
// targets to labels (see `disassembler::resolve`) and formatting the
// optional offset/label/bytes columns around each instruction.

use crate::disassembler::instruction::Instruction;
use crate::disassembler::procedure_dict::ProcedureInfo;
use crate::disassembler::{self, Mnemonic, Operand};
use crate::segment_dictionary::SegmentDictionary;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// Which optional columns/renderings to include in the printed listing --
/// grouped into one struct (rather than threaded as three separate bools)
/// purely to keep `print_instructions`/`format_instruction_line`'s
/// argument counts under clippy's `too_many_arguments` threshold.
#[derive(Clone, Copy)]
struct DisplayOptions {
    offsets: bool,
    bytes: bool,
    labels: bool,
}

/// Runs the `disassemble` subcommand: reads `file_name`, parses its segment
/// dictionary, and prints every active segment's procedures and their
/// decoded instructions.
///
/// - `show_offsets`: prefix each instruction with its offset within the
///   segment.
/// - `show_bytes`: prefix each instruction with its raw hex bytes.
/// - `show_labels`: resolve jump targets and print address-based labels
///   (`loc_00dd:`, IDA-Pro-style) on target instructions, with jump
///   operands rendered as the label name instead of the raw displacement.
///   See `disassembler::resolve` for the addressing algorithm and its
///   limits (XJP's per-case table is never labeled).
///
/// Returns an error if `file_name` can't be read or its segment dictionary
/// doesn't parse (`SegmentDictionary::parse`'s error conditions). A
/// segment whose procedure dictionary doesn't parse isn't an error -- it's
/// reported inline and disassembly falls back to a flat, unresolved decode
/// of the whole segment (jump resolution needs a procedure's `jtab_addr`,
/// which isn't available without a parsed dictionary).
pub fn run(
    file_name: String,
    show_offsets: bool,
    show_bytes: bool,
    show_labels: bool,
) -> anyhow::Result<()> {
    let opts = DisplayOptions {
        offsets: show_offsets,
        bytes: show_bytes,
        labels: show_labels,
    };
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
        if show_offsets {
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
                    print_instructions(segment_bytes, Some(proc), opts);
                }
            }
            None => {
                println!(
                    "  (couldn't parse procedure dictionary; showing raw decode; labels unavailable without a parsed dictionary)"
                );
                // No ProcedureInfo means no jtab_addr, so negative-SB jump
                // targets can't be resolved here -- suppress the label
                // column entirely (rather than leaving it permanently
                // blank on every line) regardless of show_labels.
                let opts = DisplayOptions {
                    labels: false,
                    ..opts
                };
                print_instructions(segment_bytes, None, opts);
            }
        }
        println!();
    }
    Ok(())
}

/// Decodes and prints one segment's worth of instructions -- either one
/// procedure's code (`proc: Some`) or, when the procedure dictionary
/// didn't parse, the whole segment as one flat, unresolved block
/// (`proc: None`).
///
/// - `segment_bytes`: the full segment `proc`'s addresses are relative to.
/// - `proc`: `Some` to decode one procedure's `[enter_ic, code_end)` range,
///   with jump resolution/labels and reachability-based tail extension.
///   `None` for the whole-segment fallback: no `jtab_addr` is available
///   without a parsed dictionary, so jumps can't be resolved there
///   regardless of `opts.labels` (callers should pass `opts.labels: false`
///   in that case, matching `run`'s no-dictionary branch).
/// - `opts`: which optional columns/renderings to include.
///
/// The `[enter_ic, exit_ic)` range (or the whole segment, in the no-`proc`
/// case) is always printed in full, straight-line program order --
/// `exit_ic` marks where the compiler's generated exit-handling code
/// begins, but `code_end` can run a little past that (alignment padding,
/// or an undecoded jump table), so a naive sweep to `code_end` would risk
/// printing that as if it were real code. Instead, whatever's at or past
/// `exit_ic` is only printed when `disassembler::trace_reachable` can
/// actually prove it's reachable via some jump or ordinary fall-through --
/// see that function's doc comment for the algorithm and the real
/// `main()` example (in docs/p-code-jumps-and-standard-calls.md) that
/// motivated it. For most procedures that's a no-op: the instruction
/// starting exactly at `exit_ic` is a normal return (`RBP`/`RNP`), reached
/// by ordinary fall-through from the body above it, so it's always
/// reachable and always printed. But a procedure whose last real
/// statement is a call that never returns (e.g. `CSP {EXIT}`) has
/// `exit_ic` land one byte *past* that call instead, on whatever
/// unrelated bytes the compiler tucked in there (`main`'s own
/// duplicate-check code) -- genuinely unreachable, and so left unprinted.
fn print_instructions(segment_bytes: &[u8], proc: Option<&ProcedureInfo>, opts: DisplayOptions) {
    let (code, base_offset, stop_after) = match proc {
        Some(p) => (
            &segment_bytes[p.enter_ic..p.code_end],
            p.enter_ic,
            Some(p.exit_ic.saturating_sub(p.enter_ic)),
        ),
        None => (segment_bytes, 0, None),
    };

    let mut instrs = disassembler::disassemble(code);
    let last =
        stop_after.and_then(|stop_at| instrs.iter().position(|i| stop_at < i.offset + i.bytes_len));

    // trace_reachable is computed once, up front, and reused below both to
    // decide whether the boundary instruction found by `last` belongs in
    // the head, and to extend the tail past it.
    let trace = proc.map(|p| disassembler::trace_reachable(segment_bytes, p));

    if let Some(last) = last {
        // `instrs[last]` is the first instruction whose span reaches
        // exit_ic -- normally exit_ic's own real final instruction
        // (RBP/RNP), which ordinary fall-through from the body above
        // always makes reachable. Only keep it when the trace actually
        // proves that (see this function's doc comment for why it might
        // not be).
        let boundary_addr = instrs[last].addr(base_offset);
        let boundary_is_live = trace
            .as_ref()
            .is_some_and(|t| t.contains_key(&boundary_addr));
        instrs.truncate(if boundary_is_live { last + 1 } else { last });
    }

    let jtab_addr = proc.map(|p| p.jtab_addr);

    // Resolve each head instruction's own jump target now, paired with the
    // instruction itself so the two can't drift out of sync (they used to
    // be built as two separate, same-length Vecs joined by a `.zip()`).
    // Not yet validated against the final printed set -- that requires
    // knowing the tail too, added below -- so an out-of-range or
    // never-printed target is filtered out once `labels` is built.
    let mut combined: Vec<(Instruction, Option<usize>)> = instrs
        .into_iter()
        .map(|instr| {
            let target = jtab_addr.and_then(|jtab_addr| {
                disassembler::jump_displacement(&instr.operand).and_then(|sb| {
                    disassembler::resolve_jump_target(
                        instr.addr(base_offset),
                        instr.bytes_len,
                        sb,
                        jtab_addr,
                        segment_bytes,
                    )
                })
            });
            (instr, target)
        })
        .collect();

    // Extend past exit_ic with whatever the reachability trace can prove
    // is genuinely live, remapped from absolute segment addresses to this
    // procedure's code-relative offsets so it can share the rest of this
    // function's rendering (and `code`'s `--bytes` slicing) with the head.
    // Each tail instruction's resolved target comes straight from the
    // trace rather than being recomputed here -- trace_reachable already
    // resolved it once, for this exact instruction, to decide whether to
    // keep tracing past it.
    if let Some(trace) = &trace {
        let head_end = combined
            .last()
            .map(|(i, _)| i.addr(base_offset) + i.bytes_len)
            .unwrap_or(base_offset);
        for (&addr, (instr, target)) in trace.range(head_end..) {
            combined.push((
                Instruction {
                    offset: addr - base_offset,
                    ..instr.clone()
                },
                *target,
            ));
        }
    }

    // printed_starts is a second, independent notion of "what's
    // reachable/printed," alongside trace_reachable's own -- unavoidably
    // so, since the head's unconditional flat dump (above) and the tail's
    // trace-filtered addresses (also above) are only both known once
    // combined. A resolved target only earns a label if it lands on an
    // address that ends up in this set; one that doesn't (padding, or the
    // interior of an unresolved XJP case table) is indistinguishable from
    // an unresolved one at render time, and falls back to the raw operand.
    let printed_starts: BTreeSet<usize> =
        combined.iter().map(|(i, _)| i.addr(base_offset)).collect();

    let labels: BTreeMap<usize, String> = if opts.labels {
        let targets = combined
            .iter()
            .filter_map(|(_, target)| *target)
            .filter(|target| printed_starts.contains(target))
            .collect();
        disassembler::assign_labels(&targets)
    } else {
        BTreeMap::new()
    };

    for (instr, target) in &combined {
        let label = labels.get(&instr.addr(base_offset)).map(String::as_str);
        let resolved_label = target.and_then(|t| labels.get(&t)).map(String::as_str);
        println!(
            "{}",
            format_instruction_line(instr, code, base_offset, opts, label, resolved_label)
        );
    }
}

/// Formats one decoded instruction as a single output line.
///
/// - `instr`: the instruction to render.
/// - `code`: the buffer `instr.offset` is relative to (only consulted for
///   the `--bytes` column's raw hex).
/// - `base_offset`: added to `instr.offset` to get the displayed/resolved
///   address (a procedure's `enter_ic`, or 0 for the no-dictionary
///   fallback).
/// - `opts`: which optional columns to include. `opts.labels` false omits
///   the label column entirely (not just left blank), so output matches
///   the pre-label-support format exactly.
/// - `label`: `Some(name)` when this instruction's own address is a
///   resolved-and-validated jump target and `opts.labels` is true; prints
///   in the label column. `None` prints that column blank-but-padded (so
///   later columns still align down the page) whenever `opts.labels` is
///   true but this particular instruction isn't a target.
/// - `resolved_label`: passed through to `format_operand` to render a
///   resolvable jump's operand as a label instead of a raw displacement;
///   see that function.
fn format_instruction_line(
    instr: &Instruction,
    code: &[u8],
    base_offset: usize,
    opts: DisplayOptions,
    label: Option<&str>,
    resolved_label: Option<&str>,
) -> String {
    let extra = match (instr.mnemonic, &instr.operand) {
        (Mnemonic::CSP, Operand::U8(sub)) => disassembler::csp_name(*sub)
            .map(|n| format!("  {{{n}}}"))
            .unwrap_or_default(),
        _ => String::new(),
    };
    // Each optional column is independent, so any combination of
    // opts.offsets/opts.labels/opts.bytes composes without duplicating the
    // mnemonic/operand suffix that's common to every case.
    let mut prefix = String::new();
    if opts.offsets {
        let _ = write!(prefix, "{:04x}  ", instr.addr(base_offset));
    }
    if opts.labels {
        // Width matches "loc_XXXX:" (9 chars) exactly, since every label
        // this tool generates is that shape -- see assign_labels.
        let label_text = label.map(|l| format!("{l}:")).unwrap_or_default();
        let _ = write!(prefix, "{label_text:<9} ");
    }
    if opts.bytes {
        let raw = instr.raw_bytes(code);
        let mut hex = String::with_capacity(raw.len() * 3);
        for b in raw {
            let _ = write!(hex, "{b:02x} ");
        }
        let _ = write!(prefix, "{hex:<18} ");
    }
    format!(
        "{prefix}{:?}  {}{}",
        instr.mnemonic,
        format_operand(instr.mnemonic, &instr.operand, resolved_label),
        extra
    )
}

/// Renders one instruction's operand as text. Most variants are a direct
/// translation of the decoded value; the two exceptions are documented at
/// their match arms below.
///
/// - `mnemonic`: the instruction this operand belongs to -- only consulted
///   to special-case `LSA`'s `StringData` operand (see that arm below);
///   every other variant renders the same regardless of mnemonic.
/// - `operand`: the decoded operand to render.
/// - `resolved_label`: `Some(name)` only when this instruction is a
///   resolvable jump (`Operand::I8`, or `Operand::CaseJump`'s `default`
///   field) whose target was both computed and validated as landing on a
///   printed instruction, and labels are enabled (see `print_instructions`).
///   `None` otherwise -- unresolved, unresolvable (e.g. `XJP`'s per-case
///   table entries, which this function never attempts to resolve), or
///   labels disabled -- falls back to the raw numeric operand.
fn format_operand(mnemonic: Mnemonic, operand: &Operand, resolved_label: Option<&str>) -> String {
    // Shared by the two resolvable-jump arms below (Operand::I8 and
    // CaseJump's embedded `default`): render as the target's label name
    // when one exists, otherwise fall back to the raw signed displacement
    // `v` -- unresolved, unvalidated, or labels disabled all look
    // identical here. No separate "(address)" suffix is needed: the label
    // itself already encodes the address (e.g. "loc_00dd"), per
    // assign_labels.
    let jump_operand_text = |v: i8| -> String {
        match resolved_label {
            Some(name) => name.to_string(),
            None => format!("{v}"),
        }
    };
    match operand {
        Operand::None => String::new(),
        Operand::Embedded(v) => format!("{v}"),
        Operand::U8(v) => format!("{v}"),
        Operand::I8(v) => jump_operand_text(*v),
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
            // Only `default` (the embedded fallback jump, using the same SB
            // scheme as UJP/FJP) is ever resolved to a label. The per-case
            // `offsets` table uses an addressing convention this project
            // hasn't confirmed (see docs/p-code-jumps-and-standard-calls.md's
            // Caveats section) -- guessing at it risks a confidently wrong
            // label, so it's deliberately left printed as raw numbers. The
            // "(unresolved)" marker makes that explicit in the output
            // itself, rather than leaving the distinction (resolved
            // `default` vs. never-attempted `offsets`) discoverable only by
            // reading that doc.
            format!(
                "{low}..{high} default {} table {offsets:?} (unresolved)",
                jump_operand_text(*default)
            )
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
            format_operand(Mnemonic::LSA, &operand, None),
            "\"Enter your name:\""
        );
    }

    #[test]
    fn lpa_renders_as_comma_separated_bytes() {
        let operand = Operand::StringData(vec![1, 2, 255]);
        assert_eq!(format_operand(Mnemonic::LPA, &operand, None), "1,2,255");
    }

    #[test]
    fn ldc_renders_as_comma_separated_words() {
        // little-endian word pairs: 0x0001, 0x0002, 0xffff
        let operand = Operand::WordData(vec![0x01, 0x00, 0x02, 0x00, 0xff, 0xff]);
        assert_eq!(format_operand(Mnemonic::LDC, &operand, None), "1,2,65535");
    }

    #[test]
    fn i8_renders_raw_when_unresolved() {
        let operand = Operand::I8(-10);
        assert_eq!(format_operand(Mnemonic::UJP, &operand, None), "-10");
    }

    #[test]
    fn i8_renders_label_when_resolved() {
        let operand = Operand::I8(-10);
        assert_eq!(
            format_operand(Mnemonic::UJP, &operand, Some("loc_00dd")),
            "loc_00dd"
        );
    }

    #[test]
    fn case_jump_table_stays_raw_but_default_resolves() {
        let operand = Operand::CaseJump {
            low: 1,
            high: 2,
            default: -1,
            offsets: vec![10, 20],
        };
        assert_eq!(
            format_operand(Mnemonic::XJP, &operand, Some("loc_0020")),
            "1..2 default loc_0020 table [10, 20] (unresolved)"
        );
    }

    #[test]
    fn format_instruction_line_matches_pre_label_format_when_labels_off() {
        let instr = Instruction {
            offset: 0x0b,
            bytes_len: 2,
            mnemonic: Mnemonic::UJP,
            operand: Operand::I8(-10),
        };
        let code = [0u8; 16];
        let opts = DisplayOptions {
            offsets: true,
            bytes: false,
            labels: false,
        };
        let line = format_instruction_line(&instr, &code, 0x100, opts, None, None);
        assert_eq!(line, "010b  UJP  -10");
    }

    #[test]
    fn format_instruction_line_shows_label_column_and_resolved_operand() {
        let instr = Instruction {
            offset: 0x0b,
            bytes_len: 2,
            mnemonic: Mnemonic::UJP,
            operand: Operand::I8(-10),
        };
        let code = [0u8; 16];
        let opts = DisplayOptions {
            offsets: true,
            bytes: false,
            labels: true,
        };
        let line = format_instruction_line(&instr, &code, 0x100, opts, None, Some("loc_00dd"));
        // Offset column "010b  ", then a blank label column (this
        // instruction's own address isn't a target) padded to width 9 plus
        // a trailing space, then the usual "UJP  " + resolved operand text.
        // Built with format! (rather than a hand-counted literal) so the
        // padding width can't silently drift out of sync with the real
        // implementation.
        let expected = format!("010b  {:<9} UJP  loc_00dd", "");
        assert_eq!(line, expected);
    }

    #[test]
    fn format_instruction_line_shows_label_at_its_own_definition_site() {
        // This instruction's own address (0x00dd) IS a jump target -- the
        // label column should show "loc_00dd:" here, distinct from the
        // jump-operand-text case above (where the label names some *other*
        // instruction's address).
        let instr = Instruction {
            offset: 0x00dd,
            bytes_len: 1,
            mnemonic: Mnemonic::SLDL,
            operand: Operand::Embedded(1),
        };
        let code = [0u8; 256];
        let opts = DisplayOptions {
            offsets: true,
            bytes: false,
            labels: true,
        };
        let line = format_instruction_line(&instr, &code, 0, opts, Some("loc_00dd"), None);
        let expected = format!("00dd  {:<9} SLDL  1", "loc_00dd:");
        assert_eq!(line, expected);
    }
}
