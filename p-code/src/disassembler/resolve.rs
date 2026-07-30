// Resolves where a jump instruction's raw displacement operand actually
// points, and assigns human-readable labels to the resulting target
// addresses. This is the part of the p-machine's jump encoding that
// `decode.rs` deliberately leaves alone -- decoding only extracts the raw
// signed-byte operand (or, for XJP, the raw case table), since interpreting
// it requires knowing the enclosing procedure's JTAB address, which isn't
// available at decode time. See docs/p-code-jumps-and-standard-calls.md for
// the manual quote and worked examples this algorithm is derived from.
//
// `resolve_jump_target` only knows how to resolve a single SB-style
// displacement (used by UJP/FJP/EFJ/NFJ, and by XJP's embedded `default`
// field). XJP's per-case-value table (`Operand::CaseJump::offsets`) uses a
// different, unconfirmed encoding (see that doc's Caveats section) and is
// deliberately not handled here.

use super::decode::read_word;
use super::instruction::Operand;

/// Extracts the raw signed displacement from an instruction's operand, for
/// the operand shapes that carry a resolvable jump target: a plain
/// `Operand::I8` (UJP/FJP/EFJ/NFJ), or `XJP`'s embedded
/// `Operand::CaseJump::default` field. Returns `None` for every other
/// operand shape -- either not a jump at all, or (for `XJP`'s per-case
/// `offsets` table) a jump this project doesn't know how to resolve (see
/// this module's header comment). Shared by `trace_reachable` and
/// `commands::disassemble`'s label-resolution pass so the two can't drift
/// apart on which operand shapes count as jumps.
pub fn jump_displacement(operand: &Operand) -> Option<i8> {
    match operand {
        Operand::I8(v) => Some(*v),
        Operand::CaseJump { default, .. } => Some(*default),
        _ => None,
    }
}

/// Computes the absolute segment address a jump instruction's raw signed
/// displacement (`sb`) targets, per the manual's addressing rule:
/// - `instr_addr`: absolute segment offset of the jump instruction's opcode
///   byte (not offset within a procedure-relative slice).
/// - `instr_len`: the whole instruction's encoded length in bytes. For a
///   non-negative `sb`, the target is relative to the end of this
///   instruction, so this must be the full length (for XJP, the whole
///   case-jump instruction, not just its embedded `default` field).
/// - `sb`: the raw signed displacement byte (`Operand::I8`'s value, or
///   `CaseJump::default`).
/// - `jtab_addr`: the enclosing procedure's JTAB header address
///   (`ProcedureInfo::jtab_addr`), consulted only when `sb` is negative.
/// - `segment_bytes`: the *full* segment, not one procedure's code slice --
///   a negative `sb`'s JTAB slot lives past `code_end`, outside any single
///   procedure's own `[enter_ic, code_end)` range.
///
/// Returns `None` when: the JTAB slot address (`jtab_addr + sb`) falls
/// outside `segment_bytes`; the word read from that slot is larger than
/// the slot's own address, so the self-relative subtraction would
/// underflow; or (defensively) the non-negative case's addition overflows
/// `usize`. A `Some` result is not guaranteed to land on a real decoded
/// instruction's start -- callers that need that guarantee must check it
/// themselves against their own set of known instruction-start addresses
/// (this function has no way to know what the caller has already decoded).
pub fn resolve_jump_target(
    instr_addr: usize,
    instr_len: usize,
    sb: i8,
    jtab_addr: usize,
    segment_bytes: &[u8],
) -> Option<usize> {
    if sb >= 0 {
        instr_addr.checked_add(instr_len)?.checked_add(sb as usize)
    } else {
        let slot_addr = jtab_addr.checked_add_signed(sb as isize)?;
        let word_value = read_word(segment_bytes, slot_addr)?;
        // Same self-relative-pointer scheme as the JTAB header's own
        // fields (procedure_dict::resolve_self_relative) -- reused here
        // rather than reimplemented, since it's the identical arithmetic
        // applied to a jump operand instead of a dictionary-header field.
        super::procedure_dict::resolve_self_relative(slot_addr, word_value)
    }
}

/// Assigns an address-based label name ("loc_00dd", IDA-Pro-style) to each
/// of a set of jump-target addresses. `targets` must already be the final,
/// validated set the caller intends to print labels for -- every address
/// in it should be both a resolved jump target and itself among the
/// instructions actually printed; this function does no validation of its
/// own, it only names what it's given. Naming each label after its own
/// address (rather than assigning sequential numbers like "l1"/"l2") means
/// the label is unambiguous on sight -- no separate lookup needed to see
/// where a jump goes -- and sidesteps a real readability problem
/// sequential numbering had: a lowercase "l" reads as a "1" in most
/// sans-serif fonts, especially right where it matters most, at the jump
/// site's operand text.
pub fn assign_labels(
    targets: &std::collections::BTreeSet<usize>,
) -> std::collections::BTreeMap<usize, String> {
    targets
        .iter()
        .map(|&addr| (addr, format!("loc_{addr:04x}")))
        .collect()
}

/// Recursive-descent control-flow trace of a procedure's code: discovers
/// which addresses in `[enter_ic, code_end)` are provably reachable by
/// following resolved jump targets, rather than assuming every byte up to
/// `code_end` is real code (it isn't -- see
/// docs/p-code-jumps-and-standard-calls.md's "Open question": `code_end`
/// can include alignment padding or an undecoded XJP case-jump table,
/// either of which could "successfully" but wrongly decode as
/// plausible-looking garbage if approached as a linear sweep).
///
/// - `segment_bytes`: the full segment (needed both to decode instructions
///   at absolute addresses and to resolve jump targets via JTAB).
/// - `proc`: `enter_ic` seeds the trace; `code_end` is a hard upper bound
///   no decode attempt may cross (nothing at or past it is ever code --
///   that's where JTAB/procedure-dictionary metadata begins); `jtab_addr`
///   resolves negative-displacement jump targets.
///
/// Returns every instruction the trace actually visits, keyed by its own
/// absolute segment address, alongside its own resolved jump target (if
/// it has one -- `None` for a non-jump, an unresolvable jump, or a jump
/// that didn't resolve). Callers that need a jump's target anyway (to
/// label it) can reuse this instead of calling `resolve_jump_target`
/// again with the same arguments. Covers the *entire* `[enter_ic,
/// code_end)` range reachable this way -- not just the part past
/// `exit_ic`. `commands::disassemble` prints `[enter_ic, exit_ic)`
/// unconditionally regardless of what this function reports (a procedure
/// whose very first instruction is an unconditional jump with no path
/// back into its own straight-line body -- this happens for real, see
/// this project's worked `main` example -- can leave large stretches of
/// that range unmarked as reachable by this function, yet it's still the
/// procedure's real body and must still be shown); it consults this
/// function only for the single instruction sitting at `exit_ic` itself,
/// and for whatever comes after it.
///
/// Work-list algorithm, seeded with `enter_ic`: from each address, decode
/// with `decode_one` until: the address was already visited (stop this
/// path -- avoids infinite loops on backward jumps); the address falls
/// outside `[enter_ic, code_end)` (stop -- never treat dictionary/JTAB
/// metadata, or anything before this procedure's own start, as its code);
/// `decode_one` returns `None` (stop -- undecodable bytes, most likely
/// padding or the interior of an XJP case-jump table entered at the wrong
/// offset); or the instruction is a control-flow terminator (stop; push
/// the resolved target, if any, onto the work-list instead -- see below
/// for what counts as a terminator). `FJP`/`EFJ`/`NFJ` (conditional
/// jumps) push their resolved target the same way but also keep walking
/// to their own fall-through address. Any other mnemonic just continues
/// to the next byte. A jump that doesn't resolve (`resolve_jump_target`
/// returns `None`) is simply dropped from the work-list -- it never
/// aborts the trace. `XJP`'s per-case `offsets` table is never followed
/// (see this module's header comment); only its embedded `default` field
/// is treated as a jump target.
///
/// A terminator is `UJP`/`XJP` (unconditional jumps), `RNP`/`RBP` (return
/// -- control goes back to the caller, not the next byte here; the p-machine
/// splits "return" into two opcodes by lexical level, `RNP` for a nested
/// procedure and `RBP` for the outermost/base one, but both end the
/// procedure the same way), `XIT` (per the reference manual: "Exit the
/// operating system. Do a 'cold boot' of the system, like the operating
/// system's Halt command" -- execution never continues past it, full
/// stop), or `CSP` calling the `EXIT` standard routine specifically
/// (sub-opcode 4 -- it halts the p-machine outright). That last case
/// isn't a theoretical nicety: this project's
/// own worked `main()` example (see
/// docs/p-code-jumps-and-standard-calls.md) has `EXIT` immediately
/// followed by a compiler-emitted duplicate of its own segment-link-check
/// code, tucked past `exit_ic` purely to reuse the space `EXIT`'s halt
/// makes otherwise dead. Naively granting `CSP {EXIT}` a fall-through (as
/// an earlier version of this function did) marked that duplicate
/// reachable, which a real p-machine never executes.
pub fn trace_reachable(
    segment_bytes: &[u8],
    proc: &super::procedure_dict::ProcedureInfo,
) -> std::collections::BTreeMap<usize, (super::instruction::Instruction, Option<usize>)> {
    use super::decode::decode_one;
    use super::instruction::{Mnemonic, csp_name};

    // Deliberately not deduplicated: this trace starts at enter_ic and
    // re-decodes the procedure's head even though callers (e.g.
    // commands::disassemble) typically already decoded that same range
    // moments earlier for the unconditional flat dump. Passing those
    // already-decoded instructions in to skip re-decoding them would mean
    // accepting them in *some* coordinate space at this function's API
    // boundary -- and this module has been careful to keep absolute vs.
    // procedure-relative offsets from crossing that boundary by accident
    // (see the head/tail remapping in commands::disassemble). For a
    // disassembler over few-KB codefiles, a second decode pass over a
    // handful of instructions costs nothing worth that risk.
    // Bound decoding to code_end: decode_one is handed the *whole*
    // segment_bytes elsewhere in this crate (e.g. to resolve a negative-SB
    // JTAB slot, which legitimately lives past code_end), but here it must
    // never read a code byte from code_end onward -- that's
    // dictionary/JTAB metadata, not code. A multi-byte instruction whose
    // opcode byte sits just before code_end could otherwise have its
    // operand byte(s) read from that metadata and decoded as if they were
    // real, producing a plausible-looking but wrong instruction. Slicing
    // segment_bytes to code_end makes every one of decode_one's
    // bounds-checked reads (`code.get(...)?`) fail past that point, which
    // the existing "decode_one returned None" handling below already
    // treats as "stop this path" -- so this is a one-line fix, not a new
    // failure mode to handle.
    let code_bound = proc.code_end.min(segment_bytes.len());
    let bounded_code = &segment_bytes[..code_bound];

    let mut visited = std::collections::BTreeMap::new();
    let mut work_list = vec![proc.enter_ic];

    while let Some(addr) = work_list.pop() {
        if visited.contains_key(&addr) {
            continue;
        }
        if addr < proc.enter_ic || addr >= proc.code_end {
            continue;
        }
        let Some(instr) = decode_one(bounded_code, addr) else {
            continue;
        };

        let target = jump_displacement(&instr.operand).and_then(|sb| {
            resolve_jump_target(addr, instr.bytes_len, sb, proc.jtab_addr, segment_bytes)
        });
        if let Some(target) = target {
            work_list.push(target);
        }
        let calls_exit = matches!(
            (instr.mnemonic, &instr.operand),
            (Mnemonic::CSP, Operand::U8(sub)) if csp_name(*sub) == Some("EXIT")
        );
        let is_terminator = calls_exit
            || matches!(
                instr.mnemonic,
                Mnemonic::UJP | Mnemonic::XJP | Mnemonic::RNP | Mnemonic::RBP | Mnemonic::XIT
            );
        let next_addr = addr + instr.bytes_len;
        visited.insert(addr, (instr, target));
        if !is_terminator {
            work_list.push(next_addr);
        }
    }

    visited
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn resolve_nonnegative_sb_is_simple_addition() {
        // GotoDemo's FJP 2 at 0x0109 (2-byte instruction) -> target 0x010d,
        // per docs/p-code-jumps-and-standard-calls.md's worked example.
        let target = resolve_jump_target(0x0109, 2, 2, 0, &[]);
        assert_eq!(target, Some(0x010d));
    }

    #[test]
    fn resolve_negative_sb_via_jtab_slot() {
        // Small synthetic segment: a JTAB slot at address 0x0c holding the
        // word 5, self-relative to that same slot -- target = 0x0c - 5 = 7.
        let mut segment = vec![0u8; 16];
        segment[0x0c] = 5;
        segment[0x0d] = 0;
        // jtab_addr=0x10, sb=-4 -> slot_addr = 0x10 - 4 = 0x0c.
        let target = resolve_jump_target(0, 2, -4, 0x10, &segment);
        assert_eq!(target, Some(7));
    }

    #[test]
    fn resolve_negative_sb_out_of_bounds_slot_is_none() {
        let segment = vec![0u8; 4];
        // jtab_addr + sb = 2 - 8 = underflows usize entirely (checked_add_signed -> None).
        assert_eq!(resolve_jump_target(0, 2, -8, 2, &segment), None);
    }

    #[test]
    fn resolve_negative_sb_slot_read_past_segment_end_is_none() {
        let segment = vec![0u8; 4];
        // slot_addr = 10 - 2 = 8, past the 4-byte segment -- read_word fails.
        assert_eq!(resolve_jump_target(0, 2, -2, 10, &segment), None);
    }

    #[test]
    fn resolve_negative_sb_underflowing_word_value_is_none() {
        // slot_addr = 4, but the word stored there (100) exceeds it, so
        // "slot_addr - word_value" would underflow.
        let mut segment = vec![0u8; 8];
        segment[4] = 100;
        segment[5] = 0;
        assert_eq!(resolve_jump_target(0, 2, -4, 8, &segment), None);
    }

    #[test]
    fn assign_labels_names_each_target_after_its_own_address() {
        let mut targets = BTreeSet::new();
        targets.insert(0x20);
        targets.insert(0x05);
        targets.insert(0x10dd);
        let labels = assign_labels(&targets);
        assert_eq!(labels.get(&0x20).map(String::as_str), Some("loc_0020"));
        assert_eq!(labels.get(&0x05).map(String::as_str), Some("loc_0005"));
        assert_eq!(labels.get(&0x10dd).map(String::as_str), Some("loc_10dd"));
    }

    // Ground-truthed against the real tests/FEATURES.CODE fixture, using
    // GotoDemo's confirmed worked example (procedure 6) from
    // docs/p-code-jumps-and-standard-calls.md: FJP 2 at 0x0109 -> 0x010d,
    // and UJP -10 at 0x010b -> 0x00dd (via JTAB). Uses the segment
    // dictionary's own parsing so segment bounds aren't hardcoded blindly.
    #[test]
    fn resolve_real_gotodemo_jumps() {
        let contents = include_bytes!("../../../tests/FEATURES.CODE");
        let dict = crate::segment_dictionary::SegmentDictionary::parse(contents.as_slice())
            .expect("parse segment dictionary");
        let (_, code_info, _) = dict
            .active_segments()
            .find(|(_, _, name)| name.as_str() == "FEATURED")
            .expect("FEATURED segment present"); // segment name is 8-char-truncated "FEATURES"
        let start = code_info.address as usize * 512;
        let end = start + code_info.length as usize;
        let segment_bytes = &contents[start..end];

        let proc_dict = super::super::parse_procedure_dictionary(segment_bytes)
            .expect("parse procedure dictionary");
        let goto_demo = proc_dict
            .procedures
            .iter()
            .find(|p| p.number == 6)
            .expect("procedure 6 (GotoDemo)");

        // FJP 2 at 0x0109, 2-byte instruction -> 0x0109 + 2 + 2 = 0x010d.
        let fjp_target = resolve_jump_target(0x0109, 2, 2, goto_demo.jtab_addr, segment_bytes);
        assert_eq!(fjp_target, Some(0x010d));

        // UJP -10 at 0x010b -> via JTAB, target 0x00dd.
        let ujp_target = resolve_jump_target(0x010b, 2, -10, goto_demo.jtab_addr, segment_bytes);
        assert_eq!(ujp_target, Some(0x00dd));
    }

    // A minimal ProcedureInfo for trace_reachable tests that don't need a
    // real parsed dictionary -- only enter_ic/code_end/jtab_addr matter to
    // the trace; the other fields are irrelevant filler.
    fn synthetic_proc(
        enter_ic: usize,
        code_end: usize,
        jtab_addr: usize,
    ) -> super::super::procedure_dict::ProcedureInfo {
        super::super::procedure_dict::ProcedureInfo {
            number: 1,
            lex_level: 0,
            enter_ic,
            exit_ic: enter_ic,
            param_size: 0,
            data_size: 0,
            code_end,
            jtab_addr,
        }
    }

    #[test]
    fn trace_reachable_straight_line_no_jumps() {
        // SLDC 0, SLDC 1, NOP, RBP 0 -- no jumps, every byte in order.
        let segment = [0u8, 1, 215, 193, 0];
        let proc = synthetic_proc(0, segment.len(), 0);
        let trace = trace_reachable(&segment, &proc);
        let visited: Vec<usize> = trace.keys().copied().collect();
        assert_eq!(visited, vec![0, 1, 2, 3]);
    }

    #[test]
    fn trace_reachable_unconditional_jump_skips_dead_bytes() {
        // 0: NOP                          -- live (enter_ic)
        // 1: UJP +5 -> (1+2)+5=8          -- live, no fall-through
        // 3..7: NOP x5                    -- dead: only reachable if UJP
        //                                    incorrectly fell through
        // 8: RBP 0                        -- live (the jump's target)
        let mut segment = vec![215, 185, 5, 215, 215, 215, 215, 215, 193, 0];
        segment[0] = 215; // NOP (enter_ic)
        let proc = synthetic_proc(0, segment.len(), 0);
        let trace = trace_reachable(&segment, &proc);
        let visited: Vec<usize> = trace.keys().copied().collect();
        assert_eq!(visited, vec![0, 1, 8]);
    }

    #[test]
    fn trace_reachable_conditional_jump_visits_both_paths_but_not_the_gap() {
        // 0: SLDC 0                       -- live (enter_ic)
        // 1: FJP +3 -> (1+2)+3=6          -- live; conditional, so both the
        //                                    fall-through (3) and the taken
        //                                    branch (6) must be live
        // 3: SLDC 1                       -- live (fall-through path)
        // 4: UJP +4 -> (4+2)+4=10         -- live, no fall-through
        // 6: UJP +2 -> (6+2)+2=10         -- live (FJP's taken branch); its
        //                                    own jump converges on the same
        //                                    RBP as the fall-through path,
        //                                    rather than falling through
        //                                    into the dead gap below
        // 8..9: NOP x2                    -- dead: not on either path
        // 10: RBP 0                       -- live (where both paths converge)
        let segment = [
            0, // 0: SLDC 0
            161, 3, // 1: FJP +3
            1, // 3: SLDC 1
            185, 4, // 4: UJP +4
            185, 2, // 6: UJP +2 (FJP's target)
            215, 215, // 8,9: dead
            193, 0, // 10: RBP 0 (both UJPs' target)
        ];
        let proc = synthetic_proc(0, segment.len(), 0);
        let trace = trace_reachable(&segment, &proc);
        let visited: Vec<usize> = trace.keys().copied().collect();
        assert_eq!(visited, vec![0, 1, 3, 4, 6, 10]);
    }

    #[test]
    fn trace_reachable_never_decodes_past_code_end() {
        // Regression test for a real bug: decode_one was previously handed
        // the *whole* segment_bytes with no upper bound, so a multi-byte
        // instruction whose opcode byte sits just before code_end could
        // have its operand byte(s) read from code_end onward -- JTAB/
        // procedure-dictionary metadata, not code.
        //
        // 0: NOP                -- live (enter_ic), 1 byte
        // 1: LAO opcode (165)   -- big_operand: needs a B-format byte at 2.
        //                          Before the fix, decode_one would read
        //                          that byte from segment[2], which is
        //                          >= code_end -- i.e. metadata, not code.
        // code_end = 2, so byte 2 is already past the procedure's code.
        let segment = vec![215u8, 165, 99, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let proc = synthetic_proc(0, 2, 100); // code_end=2; jtab_addr unused here
        let trace = trace_reachable(&segment, &proc);
        for (&addr, (instr, _target)) in trace.iter() {
            assert!(
                addr + instr.bytes_len <= proc.code_end,
                "instruction at {addr:#x} (len {}) extends to {:#x}, past code_end {:#x}",
                instr.bytes_len,
                addr + instr.bytes_len,
                proc.code_end
            );
        }
        // The NOP at 0 is still found; the LAO at 1 is correctly rejected
        // (decode_one can't read its operand byte from beyond code_end).
        assert_eq!(trace.keys().copied().collect::<Vec<_>>(), vec![0]);
    }

    #[test]
    fn trace_reachable_xit_is_a_terminator() {
        // XIT ("Exit the operating system. Do a 'cold boot' of the
        // system, like the operating system's Halt command," per the
        // reference manual) never falls through, same as UJP/XJP/RNP/
        // CSP{EXIT}.
        // 0: NOP        -- live (enter_ic)
        // 1: XIT        -- live, no fall-through
        // 2: NOP        -- dead: only reachable if XIT incorrectly fell through
        let segment = [215u8, 214, 215];
        let proc = synthetic_proc(0, segment.len(), 0);
        let trace = trace_reachable(&segment, &proc);
        let visited: Vec<usize> = trace.keys().copied().collect();
        assert_eq!(visited, vec![0, 1]);
    }

    #[test]
    fn trace_reachable_rbp_is_a_terminator() {
        // RBP (opcode 193, "return, base procedure" -- lex level 0's
        // counterpart to RNP) never falls through, same as RNP itself.
        // 0: NOP           -- live (enter_ic)
        // 1: RBP 0 (2 bytes) -- live, no fall-through
        // 3: NOP           -- dead: only reachable if RBP incorrectly fell
        //                      through (this is exactly the shape of the
        //                      alignment-padding byte a real codefile has
        //                      right after a base procedure's RBP -- see
        //                      trace_reachable_real_hello_world_stops_at_rbp
        //                      below for the real-fixture version)
        let segment = [215u8, 193, 0, 215];
        let proc = synthetic_proc(0, segment.len(), 0);
        let trace = trace_reachable(&segment, &proc);
        let visited: Vec<usize> = trace.keys().copied().collect();
        assert_eq!(visited, vec![0, 1]);
    }

    // Regression test: RBP wasn't originally in trace_reachable's
    // terminator list (only RNP was), so the tail-extension wrongly
    // treated a base procedure's single alignment-padding byte after its
    // RBP as reachable -- and since that byte happens to be 0x00, a
    // valid SLDC 0, it was silently printed as if it were live code.
    #[test]
    fn trace_reachable_real_hello_world_stops_at_rbp() {
        let contents = include_bytes!("../../../tests/HelloWorld.code");
        // segment 0: code_info = {address: 1, length: 0x70} -> bytes [512..624)
        let segment_bytes = &contents[512..512 + 0x70];

        let proc_dict = super::super::parse_procedure_dictionary(segment_bytes)
            .expect("parse procedure dictionary");
        let main = &proc_dict.procedures[0];
        assert_eq!(main.exit_ic, 0x005f);

        let trace = trace_reachable(segment_bytes, main);
        assert!(
            trace.contains_key(&0x005f),
            "RBP itself (exit_ic) should be reachable"
        );
        assert!(
            !trace.contains_key(&0x0061),
            "the alignment-padding byte right after RBP should not be reachable"
        );
    }

    #[test]
    fn trace_reachable_real_main_segment_link_prologue() {
        // Ground-truthed against tests/FEATURES.CODE procedure 1 (`main`),
        // whose very first instruction is `UJP -10` -- see
        // docs/p-code-jumps-and-standard-calls.md's "Live code can exist
        // past exit_ic" section. `exit_ic` (0x0d68) lands on the byte
        // immediately after `CSP 4 {EXIT}` (at 0x0d66) rather than on
        // EXIT itself -- confirmed by hand against the raw bytes, since
        // this differs from the more common case (`exit_ic` pointing
        // directly at a procedure's final RBP/RNP).
        //
        // Asserts three things verified by hand-decoding the real bytes:
        // - 0x0d70 (the segment-link check) is reachable only via the
        //   leading `UJP -10`'s resolved target.
        // - 0x0ab4 (main's real body, right after that same leading UJP)
        //   is reachable too -- via a *second* jump, `UJP -12` at 0x0d76,
        //   which resolves back to 0x0ab4 once the link check finishes.
        //   (This numeric target was independently hand-verified against
        //   the raw JTAB slot bytes -- it doesn't match this doc's own
        //   prose, which describes this same jump landing at 0x0d6c; that
        //   prose appears to be a transcription slip, not a second valid
        //   reading, since 0x0ab4 is what both the manual's formula and a
        //   real p-machine's behavior -- "resume the real body after the
        //   one-time link check" -- would require.)
        // - 0x0d68/0x0d69 (the start of the compiler's dead, tucked-away
        //   duplicate check code, "PATH A") are NOT reachable: nothing
        //   jumps there, and `CSP 4 {EXIT}` at 0x0d66 is a control-flow
        //   terminator (see trace_reachable's doc comment), so its
        //   fall-through doesn't count either.
        let contents = include_bytes!("../../../tests/FEATURES.CODE");
        let dict = crate::segment_dictionary::SegmentDictionary::parse(contents.as_slice())
            .expect("parse segment dictionary");
        let (_, code_info, _) = dict
            .active_segments()
            .find(|(_, _, name)| name.as_str() == "FEATURED")
            .expect("FEATURED segment present");
        let start = code_info.address as usize * 512;
        let end = start + code_info.length as usize;
        let segment_bytes = &contents[start..end];

        let proc_dict = super::super::parse_procedure_dictionary(segment_bytes)
            .expect("parse procedure dictionary");
        let main = proc_dict
            .procedures
            .iter()
            .find(|p| p.number == 1)
            .expect("procedure 1 (main)");
        assert_eq!(main.enter_ic, 0x0ab2);
        assert_eq!(main.exit_ic, 0x0d68);

        let trace = trace_reachable(segment_bytes, main);
        assert!(
            trace.contains_key(&0x0d70),
            "segment-link check (reached only via the leading UJP -10) should be reachable"
        );
        assert!(
            trace.contains_key(&0x0ab4),
            "main's real body (reached via the link check's own UJP back) should be reachable"
        );
        assert!(
            !trace.contains_key(&0x0d68) && !trace.contains_key(&0x0d69),
            "dead top of PATH A (never jumped to, and CSP{{EXIT}} doesn't fall through) should not be reachable"
        );
    }
}
