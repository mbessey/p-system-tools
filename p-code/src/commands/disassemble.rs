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
use p_system_format::pascal_string::from_space_padded;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// Which optional columns/renderings to include in the printed listing --
/// grouped into one struct (rather than threaded as separate bools) purely
/// to keep `print_instructions`/`format_instruction_line`'s argument counts
/// under clippy's `too_many_arguments` threshold.
#[derive(Clone, Copy)]
struct DisplayOptions {
    file_offsets: bool,
    offsets: bool,
    bytes: bool,
    labels: bool,
    footers: bool,
}

/// The per-segment context every instruction/gap/footer-rendering function
/// needs: the segment's own bytes (to slice out gap/footer regions), its
/// starting file offset (for the `--file-offsets` column), and the
/// codefile's segment dictionary (to resolve `CXP` call targets to segment
/// names). Bundled into one struct -- rather than threaded as three more
/// separate parameters -- purely to keep each function's argument count
/// under clippy's `too_many_arguments` threshold.
#[derive(Clone, Copy)]
struct SegmentContext<'a> {
    bytes: &'a [u8],
    file_offset: usize,
    dictionary: &'a SegmentDictionary,
}

/// Runs the `disassemble` subcommand: reads `file_name`, parses its segment
/// dictionary, and prints every active segment's procedures and their
/// decoded instructions. Within a segment, procedures are printed in
/// ascending code-address order (not procedure-dictionary order -- see the
/// sort in this function's body), so the listing's offsets always increase
/// top-to-bottom.
///
/// - `show_file_offsets`: prefix each instruction with its offset within
///   the whole codefile (segment's file offset plus its offset within the
///   segment). Printed leftmost when both offset columns are shown, since
///   it's the coarser, whole-file coordinate.
/// - `show_offsets`: prefix each instruction with its offset within the
///   segment.
/// - `show_bytes`: prefix each instruction with its raw hex bytes.
/// - `show_labels`: resolve jump targets and print address-based labels
///   (`loc_00dd:`, IDA-Pro-style) on target instructions, with jump
///   operands rendered as the label name instead of the raw displacement.
///   See `disassembler::resolve` for the addressing algorithm and its
///   limits (XJP's per-case table is never labeled).
/// - `show_footers`: print any bytes `disassembler::trace_reachable`
///   couldn't prove reachable -- both gaps in the middle of a procedure's
///   instruction sequence and the run of bytes between its last printed
///   instruction and its own on-disk JTAB footer, followed by that footer
///   (raw bytes; its fields are already decoded and printed in the
///   "Procedure N (...)" line above the code, so they aren't decoded
///   again here). No effect when the procedure dictionary didn't parse
///   (`proc: None` has no footer, and its whole-segment fallback decode
///   has no unreachable gaps to begin with).
///
/// Returns `Error::Io` if `file_name` can't be read, or `Error::Format` if
/// its segment dictionary doesn't parse (`SegmentDictionary::parse`'s error
/// conditions). A segment whose procedure dictionary doesn't parse isn't an
/// error -- it's reported inline and disassembly falls back to a flat,
/// unresolved decode of the whole segment (jump resolution needs a
/// procedure's `jtab_addr`, which isn't available without a parsed
/// dictionary).
pub fn run(
    file_name: String,
    show_file_offsets: bool,
    show_offsets: bool,
    show_bytes: bool,
    show_labels: bool,
    show_footers: bool,
) -> Result<(), crate::error::Error> {
    let opts = DisplayOptions {
        file_offsets: show_file_offsets,
        offsets: show_offsets,
        bytes: show_bytes,
        labels: show_labels,
        footers: show_footers,
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
        let ctx = SegmentContext {
            bytes: segment_bytes,
            file_offset: start,
            dictionary: &segment_dictionary,
        };

        println!("Segment {s} ({seg_name}):");
        if show_file_offsets {
            println!("  (offset within file)");
        }
        if show_offsets {
            println!("  (offset within segment; segment starts at file offset {start:#x})");
        }
        match disassembler::parse_procedure_dictionary(segment_bytes) {
            Some(dict) => {
                println!("  (SEGTABLE slot {})", dict.segment_number);
                for proc in procedures_in_code_order(&dict.procedures) {
                    println!(
                        "  Procedure {} (lex level {}, param size {}, data size {}, exit at {:04x}):",
                        proc.number, proc.lex_level, proc.param_size, proc.data_size, proc.exit_ic
                    );
                    print_instructions(ctx, Some(proc), opts);
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
                print_instructions(ctx, None, opts);
            }
        }
        println!();
    }
    Ok(())
}

/// Orders a segment's procedures by code address (`enter_ic`) rather than
/// procedure-dictionary order (which is procedure number, `dict.procedures`'
/// natural order). The compiler typically places the outermost block
/// (usually procedure 1) last in the segment, so printing in dictionary
/// order would jump backward in the file every time procedure 1 ends;
/// sorting here makes the listing read top-to-bottom in the same order as
/// the file's bytes, matching the `--file-offsets` column.
fn procedures_in_code_order(procedures: &[ProcedureInfo]) -> Vec<&ProcedureInfo> {
    let mut ordered: Vec<&ProcedureInfo> = procedures.iter().collect();
    ordered.sort_by_key(|p| p.enter_ic);
    ordered
}

/// Decodes and prints one segment's worth of instructions -- either one
/// procedure's code (`proc: Some`) or, when the procedure dictionary
/// didn't parse, the whole segment as one flat, unresolved block
/// (`proc: None`).
///
/// - `ctx`: the enclosing segment's bytes/file-offset/dictionary; see
///   `SegmentContext`.
/// - `proc`: `Some` to decode one procedure's `[enter_ic, code_end)` range,
///   with jump resolution/labels and reachability-based tail extension.
///   `None` for the whole-segment fallback: no `jtab_addr` is available
///   without a parsed dictionary, so jumps can't be resolved there
///   regardless of `opts.labels` (callers should pass `opts.labels: false`
///   in that case, matching `run`'s no-dictionary branch).
/// - `opts`: which optional columns/renderings to include.
///
/// The `[enter_ic, exit_ic)` range (or the whole segment, in the no-`proc`
/// case) is always decoded in full, straight-line program order -- but
/// only instructions `disassembler::trace_reachable` can actually prove
/// reachable (via some jump or ordinary fall-through) get printed; see
/// that function's doc comment for the algorithm and the real `main()`
/// example (in docs/p-code-jumps-and-standard-calls.md) that motivated it.
/// For most procedures that's a no-op: the instruction starting exactly at
/// `exit_ic` is a normal return (`RBP`/`RNP`), reached by ordinary
/// fall-through from the body above it, so it's always reachable and
/// always printed. But a procedure whose last real statement is a call
/// that never returns (e.g. `CSP {EXIT}`) can leave real, validly-decoding
/// p-code stranded on both sides of `exit_ic` with no path this tool
/// traces reaching it -- confirmed by hand-decoding examples from
/// tests/FEATURES.CODE's main program, which is why this isn't assumed to
/// be mere alignment padding (see `opts.footers`'s doc on `run`).
/// `code_end` can also run a little past the last real instruction for
/// genuine word-alignment reasons (0 or 1 pad bytes) before the JTAB
/// footer begins.
fn print_instructions(ctx: SegmentContext, proc: Option<&ProcedureInfo>, opts: DisplayOptions) {
    let (code, base_offset, stop_after) = match proc {
        Some(p) => (
            &ctx.bytes[p.enter_ic..p.code_end],
            p.enter_ic,
            Some(p.exit_ic.saturating_sub(p.enter_ic)),
        ),
        None => (ctx.bytes, 0, None),
    };

    let mut instrs = disassembler::disassemble(code);
    let last =
        stop_after.and_then(|stop_at| instrs.iter().position(|i| stop_at < i.offset + i.bytes_len));

    // trace_reachable is computed once, up front, and reused below both to
    // decide whether the boundary instruction found by `last` belongs in
    // the head, and to extend the tail past it.
    let trace = proc.map(|p| disassembler::trace_reachable(ctx.bytes, p));

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
                        ctx.bytes,
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
    // address that ends up in this set; one that doesn't (an unreached gap,
    // or the interior of an unresolved XJP case table) is indistinguishable
    // from an unresolved one at render time, and falls back to the raw
    // operand.
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

    // combined is built from two independently-filtered pieces (the head,
    // truncated at exit_ic; the tail, whatever trace_reachable proved live
    // past it) that don't necessarily abut -- there can be a real gap
    // between them (or, in principle, within either piece, though neither
    // currently produces one on its own). prev_end tracks where the
    // printed sequence last left off, so any such gap gets flagged with
    // opts.footers rather than silently vanishing between two printed
    // instructions with no indication anything was skipped.
    let mut prev_end: Option<usize> = None;
    for (instr, target) in &combined {
        let addr = instr.addr(base_offset);
        if opts.footers
            && let Some(prev) = prev_end
            && prev < addr
        {
            println!("{}", format_gap_line(ctx, prev, addr, opts));
        }
        let label = labels.get(&addr).map(String::as_str);
        let resolved_label = target.and_then(|t| labels.get(&t)).map(String::as_str);
        println!(
            "{}",
            format_instruction_line(instr, code, ctx, base_offset, opts, label, resolved_label)
        );
        prev_end = Some(addr + instr.bytes_len);
    }

    if opts.footers
        && let Some(p) = proc
    {
        // Where the printed instructions actually stop: not exit_ic
        // itself, but the byte past the last printed instruction's own
        // bytes (which may be exit_ic's instruction, or may run further if
        // trace_reachable extended the tail). Everything from here to
        // code_end is either a final unreached gap or, once past that,
        // this procedure's own JTAB footer.
        let code_tail_end = prev_end.unwrap_or(base_offset);
        println!("{}", format_procedure_footer(ctx, p, code_tail_end, opts));
    }
}

/// Formats the bytes between a procedure's last decoded instruction and the
/// next procedure's code, for `--footers`: any bytes left over before this
/// procedure's own on JTAB footer (see `format_gap_line`), then the
/// footer itself. Both are real bytes in the file, but neither is printed
/// as ordinary decoded p-code -- which is why they otherwise look like an
/// unexplained gap between consecutive procedures' listings. Called
/// "footer" (not "header") because it sits at the *end* of the procedure it
/// belongs to, immediately after that procedure's own code -- see
/// `procedure_dict::parse_jtab`, which reads this same span to build
/// `proc`'s fields in the first place. Shown here only as raw hex, not
/// decoded again: its fields are already printed in the "Procedure N (...)"
/// line above the code.
///
/// - `ctx`: as in `print_instructions`.
/// - `proc`: the procedure whose footer follows its own code, not the next
///   procedure's -- each procedure's JTAB footer describes itself.
/// - `code_tail_end`: segment-relative offset just past the last
///   instruction `print_instructions` actually printed for `proc`; the
///   region `[code_tail_end, proc.code_end)` is unreached bytes, included
///   only when non-empty.
/// - `opts`: only `opts.file_offsets`/`opts.offsets` affect this output
///   (via `offset_prefix`); `opts.footers` is what gates calling this
///   function in the first place.
fn format_procedure_footer(
    ctx: SegmentContext,
    proc: &ProcedureInfo,
    code_tail_end: usize,
    opts: DisplayOptions,
) -> String {
    let mut lines = Vec::new();
    if code_tail_end < proc.code_end {
        lines.push(format_gap_line(ctx, code_tail_end, proc.code_end, opts));
    }
    // jtab_addr is the address of the footer's last word (procedure
    // number/lex level); the footer itself spans code_end..jtab_addr+2.
    let footer_start = proc.code_end;
    let footer_end = proc.jtab_addr + 2;
    lines.push(format!(
        "{}(JTAB footer: {})",
        offset_prefix(ctx.file_offset, footer_start, opts),
        hex_bytes(&ctx.bytes[footer_start..footer_end])
    ));
    lines.join("\n")
}

/// Formats one "(unreached bytes: ...)" line describing the raw hex content
/// of `[start, end)` in `ctx.bytes` -- used both for gaps within a
/// procedure's printed instruction sequence (a region `trace_reachable`
/// couldn't prove reachable, so `print_instructions` never printed it) and,
/// via `format_procedure_footer`, for the run of bytes between a
/// procedure's last printed instruction and its JTAB footer. Deliberately
/// not called "padding": hand-decoding examples of this region from
/// tests/FEATURES.CODE's main program shows it's often real, validly-coded
/// p-code (e.g. a second copy of a duplicate-run check, or a genuine `RBP`
/// return) that just has no path this tool's reachability tracer follows --
/// not necessarily alignment filler. It's printed as raw hex rather than
/// decoded, since without a proven-reachable entry point there's no
/// trustworthy place to start decoding from.
fn format_gap_line(ctx: SegmentContext, start: usize, end: usize, opts: DisplayOptions) -> String {
    format!(
        "{}(unreached bytes: {})",
        offset_prefix(ctx.file_offset, start, opts),
        hex_bytes(&ctx.bytes[start..end])
    )
}

/// Renders `bytes` as space-separated lowercase hex, e.g. `"00 06 07"`. No
/// trailing space (unlike `format_instruction_line`'s `--bytes` column,
/// which pads to a fixed column width for its own alignment purposes).
fn hex_bytes(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 3);
    for b in bytes {
        let _ = write!(s, "{b:02x} ");
    }
    s.trim_end().to_string()
}

/// Builds the `--file-offsets`/`--offsets` column prefix for one address --
/// shared by every per-line renderer in this module (instructions, gap
/// lines, the JTAB footer line) so they all stay aligned under the same
/// columns. `addr` is segment-relative; see `format_instruction_line`'s doc
/// comment for the file-offset-vs-offset column ordering rationale.
fn offset_prefix(segment_file_offset: usize, addr: usize, opts: DisplayOptions) -> String {
    let mut prefix = String::new();
    if opts.file_offsets {
        let _ = write!(prefix, "{:06x}  ", segment_file_offset + addr);
    }
    if opts.offsets {
        let _ = write!(prefix, "{:04x}  ", addr);
    }
    prefix
}

/// Resolves a `CXP` call's unit/segment-number operand to a human name, for
/// the `{NAME}` suffix on cross-segment calls (mirroring `CSP`'s `{NAME}`
/// suffix for standard routines).
///
/// Unit 0 is hardcoded as `"SYSTEM"`. This is a p-System convention this
/// project hasn't independently verified end-to-end, but the evidence
/// points the same way: tests/FEATURES.CODE's own segment 0 has only 12
/// procedures, yet its `CXP 0,*` calls reference procedure numbers (19, 22,
/// ...) that don't exist there, and the file's `intrinsic_segments`
/// bitfield is nonzero -- both consistent with those calls resolving
/// through System.Library rather than this codefile's own dictionary. This
/// tool never loads System.Library, so `"SYSTEM"` is the most it can say
/// without guessing a specific routine name.
///
/// Units 1-15 are looked up against this codefile's own segment
/// dictionary, for genuine intra-file segment/unit calls -- `None` if that
/// slot isn't active here (rather than guessing a name for a segment this
/// file doesn't define; the call may still be perfectly valid, just to a
/// segment this tool has no name for).
fn resolve_cxp_unit_name(unit: u8, segment_dictionary: &SegmentDictionary) -> Option<String> {
    if unit == 0 {
        return Some("SYSTEM".to_string());
    }
    let code_info = segment_dictionary.code_info.get(usize::from(unit))?;
    if code_info.address == 0 {
        return None;
    }
    Some(from_space_padded(
        &segment_dictionary.seg_name[usize::from(unit)],
    ))
}

/// Formats one decoded instruction as a single output line.
///
/// - `instr`: the instruction to render.
/// - `code`: the buffer `instr.offset` is relative to (only consulted for
///   the `--bytes` column's raw hex).
/// - `ctx`: this instruction's segment context -- `ctx.file_offset` feeds
///   the `--file-offsets` column (added to `instr.addr(base_offset)`);
///   `ctx.dictionary` resolves a `CXP` call's target segment name for its
///   `{NAME}` suffix.
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
    ctx: SegmentContext,
    base_offset: usize,
    opts: DisplayOptions,
    label: Option<&str>,
    resolved_label: Option<&str>,
) -> String {
    let extra = match (instr.mnemonic, &instr.operand) {
        (Mnemonic::CSP, Operand::U8(sub)) => disassembler::csp_name(*sub)
            .map(|n| format!("  {{{n}}}"))
            .unwrap_or_default(),
        (Mnemonic::CXP, Operand::U8U8(unit, _)) => resolve_cxp_unit_name(*unit, ctx.dictionary)
            .map(|n| format!("  {{{n}}}"))
            .unwrap_or_default(),
        _ => String::new(),
    };
    // Each optional column is independent, so any combination of
    // opts.file_offsets/opts.offsets/opts.labels/opts.bytes composes
    // without duplicating the mnemonic/operand suffix that's common to
    // every case. File offset comes first (leftmost) since it's the
    // coarser, whole-file coordinate; segment offset is the finer-grained
    // one, closer to the instruction it's describing.
    let mut prefix = offset_prefix(ctx.file_offset, instr.addr(base_offset), opts);
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

    // Minimal ProcedureInfo builder for ordering tests -- only enter_ic
    // (what procedures_in_code_order sorts by) and number (to identify
    // which input procedure ended up where) matter here; the rest are
    // irrelevant filler.
    fn proc_at(number: u8, enter_ic: usize) -> ProcedureInfo {
        ProcedureInfo {
            number,
            lex_level: 0,
            enter_ic,
            exit_ic: enter_ic,
            param_size: 0,
            data_size: 0,
            code_end: enter_ic,
            jtab_addr: 0,
        }
    }

    // An all-zero 512-byte segment dictionary block parses successfully
    // (see segment_dictionary::tests::parse_accepts_minimum_size) and
    // yields an all-inactive dictionary -- a convenient stand-in wherever a
    // test needs a SegmentContext but doesn't care about CXP name
    // resolution.
    fn empty_segment_dictionary() -> SegmentDictionary {
        SegmentDictionary::parse(&[0u8; 512]).unwrap()
    }

    #[test]
    fn procedures_in_code_order_sorts_by_enter_ic_not_dictionary_order() {
        // Mirrors a real p-System layout: procedure 1 (main) is stored last
        // in the segment (highest enter_ic), with earlier-numbered
        // subroutines scattered before it in a different order than their
        // procedure numbers -- exactly the case that used to make the
        // printed listing's offsets jump backward at procedure 1's end.
        let dict_order = [
            proc_at(1, 0x0ab2),
            proc_at(2, 0x0000),
            proc_at(3, 0x0022),
            proc_at(4, 0x0092),
            proc_at(5, 0x0048),
        ];
        let ordered = procedures_in_code_order(&dict_order);
        let numbers: Vec<u8> = ordered.iter().map(|p| p.number).collect();
        assert_eq!(numbers, vec![2, 3, 5, 4, 1]);
    }

    #[test]
    fn format_procedure_footer_shows_gap_and_footer_when_bytes_precede_it() {
        // Ground-truthed against tests/FEATURES.CODE's procedure 2: its
        // last instruction (RNP, 2 bytes) ends at seg offset 0x17, one byte
        // short of code_end (0x18).
        let proc = ProcedureInfo {
            number: 2,
            lex_level: 1,
            enter_ic: 0x00,
            exit_ic: 0x15,
            param_size: 6,
            data_size: 0,
            code_end: 0x18,
            jtab_addr: 0x20,
        };
        let mut segment_bytes = vec![0u8; 0x22];
        segment_bytes[0x17] = 0x00;
        segment_bytes[0x18..0x22]
            .copy_from_slice(&[0x00, 0x00, 0x06, 0x00, 0x07, 0x00, 0x1e, 0x00, 0x02, 0x01]);
        let segment_dictionary = empty_segment_dictionary();
        let ctx = SegmentContext {
            bytes: &segment_bytes,
            file_offset: 0,
            dictionary: &segment_dictionary,
        };
        let opts = DisplayOptions {
            file_offsets: false,
            offsets: true,
            bytes: false,
            labels: false,
            footers: true,
        };
        let footer = format_procedure_footer(ctx, &proc, 0x17, opts);
        assert_eq!(
            footer,
            "0017  (unreached bytes: 00)\n\
             0018  (JTAB footer: 00 00 06 00 07 00 1e 00 02 01)"
        );
    }

    #[test]
    fn format_procedure_footer_omits_gap_line_when_none_precedes_it() {
        // Ground-truthed against tests/FEATURES.CODE's procedure 3: its
        // last instruction ends exactly on code_end (0x3e), so there's no
        // leading gap and only the footer line should appear.
        let proc = ProcedureInfo {
            number: 3,
            lex_level: 1,
            enter_ic: 0x22,
            exit_ic: 0x3c,
            param_size: 4,
            data_size: 0,
            code_end: 0x3e,
            jtab_addr: 0x46,
        };
        let mut segment_bytes = vec![0u8; 0x48];
        segment_bytes[0x3e..0x48]
            .copy_from_slice(&[0x00, 0x00, 0x04, 0x00, 0x06, 0x00, 0x22, 0x00, 0x03, 0x01]);
        let segment_dictionary = empty_segment_dictionary();
        let ctx = SegmentContext {
            bytes: &segment_bytes,
            file_offset: 0,
            dictionary: &segment_dictionary,
        };
        let opts = DisplayOptions {
            file_offsets: false,
            offsets: true,
            bytes: false,
            labels: false,
            footers: true,
        };
        let footer = format_procedure_footer(ctx, &proc, 0x3e, opts);
        assert_eq!(footer, "003e  (JTAB footer: 00 00 04 00 06 00 22 00 03 01)");
    }

    #[test]
    fn resolve_cxp_unit_name_hardcodes_unit_zero_as_system() {
        let segment_dictionary = empty_segment_dictionary();
        assert_eq!(
            resolve_cxp_unit_name(0, &segment_dictionary),
            Some("SYSTEM".to_string())
        );
    }

    #[test]
    fn resolve_cxp_unit_name_looks_up_an_active_local_segment() {
        let mut bytes = [0u8; 512];
        // code_info[3]: address=1 (nonzero, so active), length=10
        bytes[3 * 4..3 * 4 + 2].copy_from_slice(&1u16.to_le_bytes());
        bytes[3 * 4 + 2..3 * 4 + 4].copy_from_slice(&10u16.to_le_bytes());
        // seg_name[3] = "MYUNIT", space-padded to 8 bytes
        bytes[64 + 3 * 8..64 + 3 * 8 + 8].copy_from_slice(b"MYUNIT  ");
        let segment_dictionary = SegmentDictionary::parse(&bytes).unwrap();
        assert_eq!(
            resolve_cxp_unit_name(3, &segment_dictionary),
            Some("MYUNIT".to_string())
        );
    }

    #[test]
    fn resolve_cxp_unit_name_returns_none_for_an_inactive_slot() {
        let segment_dictionary = empty_segment_dictionary();
        assert_eq!(resolve_cxp_unit_name(5, &segment_dictionary), None);
    }

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
        let segment_dictionary = empty_segment_dictionary();
        let ctx = SegmentContext {
            bytes: &[],
            file_offset: 0,
            dictionary: &segment_dictionary,
        };
        let opts = DisplayOptions {
            file_offsets: false,
            offsets: true,
            bytes: false,
            labels: false,
            footers: false,
        };
        let line = format_instruction_line(&instr, &code, ctx, 0x100, opts, None, None);
        assert_eq!(line, "010b  UJP  -10");
    }

    #[test]
    fn format_instruction_line_shows_file_offset_column_leftmost() {
        let instr = Instruction {
            offset: 0x0b,
            bytes_len: 2,
            mnemonic: Mnemonic::UJP,
            operand: Operand::I8(-10),
        };
        let code = [0u8; 16];
        let segment_dictionary = empty_segment_dictionary();
        let ctx = SegmentContext {
            bytes: &[],
            file_offset: 0x400,
            dictionary: &segment_dictionary,
        };
        let opts = DisplayOptions {
            file_offsets: true,
            offsets: true,
            bytes: false,
            labels: false,
            footers: false,
        };
        // segment_file_offset (0x400) + base_offset (0x100) + instr.offset
        // (0x0b) = 0x50b, zero-padded to 6 digits, for the file-offset
        // column; 0x10b (4 digits) for the segment-offset column, exactly
        // as without file_offsets.
        let line = format_instruction_line(&instr, &code, ctx, 0x100, opts, None, None);
        assert_eq!(line, "00050b  010b  UJP  -10");
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
        let segment_dictionary = empty_segment_dictionary();
        let ctx = SegmentContext {
            bytes: &[],
            file_offset: 0,
            dictionary: &segment_dictionary,
        };
        let opts = DisplayOptions {
            file_offsets: false,
            offsets: true,
            bytes: false,
            labels: true,
            footers: false,
        };
        let line = format_instruction_line(&instr, &code, ctx, 0x100, opts, None, Some("loc_00dd"));
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
        let segment_dictionary = empty_segment_dictionary();
        let ctx = SegmentContext {
            bytes: &[],
            file_offset: 0,
            dictionary: &segment_dictionary,
        };
        let opts = DisplayOptions {
            file_offsets: false,
            offsets: true,
            bytes: false,
            labels: true,
            footers: false,
        };
        let line = format_instruction_line(&instr, &code, ctx, 0, opts, Some("loc_00dd"), None);
        let expected = format!("00dd  {:<9} SLDL  1", "loc_00dd:");
        assert_eq!(line, expected);
    }

    #[test]
    fn format_instruction_line_shows_cxp_system_suffix_for_unit_zero() {
        let instr = Instruction {
            offset: 0,
            bytes_len: 3,
            mnemonic: Mnemonic::CXP,
            operand: Operand::U8U8(0, 19),
        };
        let code = [0u8; 8];
        let segment_dictionary = empty_segment_dictionary();
        let ctx = SegmentContext {
            bytes: &[],
            file_offset: 0,
            dictionary: &segment_dictionary,
        };
        let opts = DisplayOptions {
            file_offsets: false,
            offsets: false,
            bytes: false,
            labels: false,
            footers: false,
        };
        let line = format_instruction_line(&instr, &code, ctx, 0, opts, None, None);
        assert_eq!(line, "CXP  0,19  {SYSTEM}");
    }
}
