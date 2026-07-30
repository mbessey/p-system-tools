// The procedure dictionary sits at the end of a code segment, growing
// backward from the last byte. The last word is entry 0: (segment number,
// procedure count N). Entry i (1..=N), immediately before that going toward
// lower addresses, is a self-relative pointer to procedure i's JTAB header.
// "Self-relative" is defined by the manual's jump-instruction arithmetic:
// target = address_of_the_word_holding_the_pointer - stored_value.

use super::decode::read_word;

#[derive(Debug, Clone)]
pub struct ProcedureInfo {
    pub number: u8,
    pub lex_level: i8,
    pub enter_ic: usize,
    pub exit_ic: usize,
    pub param_size: u16,
    pub data_size: u16,
    pub code_end: usize,
    // Address of this procedure's JTAB header -- equivalently code_end + 8.
    // Stored (rather than left implicit) because it's also the anchor
    // negative-SB jump displacements resolve against (see
    // docs/p-code-jumps-and-standard-calls.md): the manual's "SB DIV 2 is a
    // word offset into JTAB" scheme reads a word at jtab_addr + SB and
    // interprets it as a further self-relative pointer, the same trick
    // resolve_self_relative below uses for the header's own fields.
    pub jtab_addr: usize,
}

#[derive(Debug, Clone)]
pub struct ProcedureDictionary {
    pub segment_number: u8,
    pub procedures: Vec<ProcedureInfo>,
}

/// Resolves a self-relative pointer: `word_addr` is the address of the word
/// holding `value`, which encodes a target as "distance from `word_addr` to
/// the target." Returns `None` on underflow (a `value` larger than
/// `word_addr` itself, which can't be a real pointer in this scheme).
/// `pub(crate)` rather than private: `disassembler::resolve`'s
/// `resolve_jump_target` uses this exact same arithmetic for negative-`SB`
/// jump displacements (see the manual quote there) -- it's the same
/// self-relative-pointer trick applied to a jump operand instead of a
/// dictionary-header field, so it reuses this rather than reimplementing it.
pub(crate) fn resolve_self_relative(word_addr: usize, value: u16) -> Option<usize> {
    word_addr.checked_sub(value as usize)
}

fn parse_jtab(segment: &[u8], jtab_addr: usize) -> Option<ProcedureInfo> {
    let header_word = read_word(segment, jtab_addr)?;
    let number = (header_word & 0xFF) as u8;
    let lex_level = (header_word >> 8) as i8;

    let enter_ic_word_addr = jtab_addr.checked_sub(2)?;
    let enter_ic_value = read_word(segment, enter_ic_word_addr)?;
    let enter_ic = resolve_self_relative(enter_ic_word_addr, enter_ic_value)?;

    let exit_ic_word_addr = jtab_addr.checked_sub(4)?;
    let exit_ic_value = read_word(segment, exit_ic_word_addr)?;
    let exit_ic = resolve_self_relative(exit_ic_word_addr, exit_ic_value)?;

    let param_size_addr = jtab_addr.checked_sub(6)?;
    let param_size = read_word(segment, param_size_addr)?;

    let data_size_addr = jtab_addr.checked_sub(8)?;
    let data_size = read_word(segment, data_size_addr)?;

    // Code must run forward: entry point, then exit point, then the JTAB
    // header that follows it. Without this, a corrupt/adversarial EnterIC
    // could resolve to an address past code_end, and callers slicing
    // segment[enter_ic..code_end] would panic on start > end. exit_ic must
    // be strictly less than code_end (not just <=): the real exit
    // instruction occupies at least one byte of actual code, so it can
    // never legitimately sit exactly on the header boundary -- allowing
    // equality here would silently defeat callers that truncate a printed
    // listing at exit_ic, since a zero-length "code after exit_ic" region
    // can never be found/truncated to.
    if !(enter_ic <= exit_ic && exit_ic < data_size_addr) {
        return None;
    }

    Some(ProcedureInfo {
        number,
        lex_level,
        enter_ic,
        exit_ic,
        param_size,
        data_size,
        code_end: data_size_addr,
        jtab_addr,
    })
}

pub fn parse_procedure_dictionary(segment: &[u8]) -> Option<ProcedureDictionary> {
    let entry0_addr = segment.len().checked_sub(2)?;
    let entry0 = read_word(segment, entry0_addr)?;
    let segment_number = (entry0 & 0xFF) as u8;
    let procedure_count = (entry0 >> 8) as usize;

    let mut procedures = Vec::with_capacity(procedure_count);
    for i in 1..=procedure_count {
        let entry_addr = entry0_addr.checked_sub(2 * i)?;
        let entry_value = read_word(segment, entry_addr)?;
        let jtab_addr = resolve_self_relative(entry_addr, entry_value)?;
        if jtab_addr >= segment.len() {
            return None;
        }
        procedures.push(parse_jtab(segment, jtab_addr)?);
    }

    Some(ProcedureDictionary {
        segment_number,
        procedures,
    })
}

#[cfg(test)]
mod tests {
    use super::super::decode::disassemble;
    use super::super::instruction::Mnemonic;
    use super::*;

    #[test]
    fn procedure_dictionary_truncated_input() {
        assert!(parse_procedure_dictionary(&[]).is_none());
        assert!(parse_procedure_dictionary(&[0]).is_none());
    }

    #[test]
    fn procedure_dictionary_synthetic_segment() {
        // code: NOP(215), RBP(193, DB=0)               -- offsets 0,1  (2 bytes)
        // pad:  1 byte for word alignment               -- offset 2
        // JTAB header (5 words, low->high address):
        //   DataSize   @3  = 10
        //   ParamSize  @5  = 0
        //   ExitIC     @7  = self-rel to offset 1 (the RBP opcode byte)
        //   EnterIC    @9  = self-rel to offset 0 (the NOP)
        //   ProcNum/Lex@11 = proc 1, lex 0
        // dictionary:
        //   entry1     @13 = self-rel to JTAB header addr (11)
        //   entry0     @15 = (segment=1, count=1)
        let mut seg = vec![0u8; 17];
        seg[0] = 215; // NOP
        seg[1] = 193; // RBP
        seg[2] = 0; // RBP's DB param
        // DataSize @3-4
        seg[3] = 10;
        seg[4] = 0;
        // ParamSize @5-6
        seg[5] = 0;
        seg[6] = 0;
        // ExitIC @7-8: word_addr=7, target=1 -> value = 7-1 = 6
        seg[7] = 6;
        seg[8] = 0;
        // EnterIC @9-10: word_addr=9, target=0 -> value = 9-0 = 9
        seg[9] = 9;
        seg[10] = 0;
        // ProcNum/Lex @11-12: proc=1 (low byte), lex=0 (high byte)
        seg[11] = 1;
        seg[12] = 0;
        // entry1 @13-14: word_addr=13, target(jtab)=11 -> value = 13-11 = 2
        seg[13] = 2;
        seg[14] = 0;
        // entry0 @15-16: segment=1, count=1
        seg[15] = 1;
        seg[16] = 1;

        let dict = parse_procedure_dictionary(&seg).unwrap();
        assert_eq!(dict.segment_number, 1);
        assert_eq!(dict.procedures.len(), 1);
        let p = &dict.procedures[0];
        assert_eq!(p.number, 1);
        assert_eq!(p.lex_level, 0);
        assert_eq!(p.enter_ic, 0);
        assert_eq!(p.exit_ic, 1);
        assert_eq!(p.param_size, 0);
        assert_eq!(p.data_size, 10);
        assert_eq!(p.code_end, 3);
        assert_eq!(p.jtab_addr, 11);

        let instrs = disassemble(&seg[p.enter_ic..p.code_end]);
        assert_eq!(instrs.len(), 2);
        assert_eq!(instrs[0].mnemonic, Mnemonic::NOP);
        assert_eq!(instrs[1].mnemonic, Mnemonic::RBP);
    }

    #[test]
    fn procedure_dictionary_rejects_out_of_order_enter_ic() {
        // same layout as procedure_dictionary_synthetic_segment, but EnterIC
        // is corrupted to resolve to offset 5 (word_addr=9, value=4), which
        // is past both exit_ic (1) and code_end (3) -- callers slicing
        // segment[enter_ic..code_end] would panic on start > end if this
        // weren't rejected during parsing.
        let mut seg = vec![0u8; 17];
        seg[0] = 215;
        seg[1] = 193;
        seg[2] = 0;
        seg[3] = 10;
        seg[4] = 0;
        seg[5] = 0;
        seg[6] = 0;
        seg[7] = 6;
        seg[8] = 0;
        seg[9] = 4;
        seg[10] = 0; // corrupted EnterIC: target = 9-4 = 5
        seg[11] = 1;
        seg[12] = 0;
        seg[13] = 2;
        seg[14] = 0;
        seg[15] = 1;
        seg[16] = 1;

        assert!(parse_procedure_dictionary(&seg).is_none());
    }

    #[test]
    fn procedure_dictionary_rejects_exit_ic_equal_to_code_end() {
        // same layout as procedure_dictionary_synthetic_segment, but ExitIC
        // is corrupted to resolve exactly to code_end (word_addr=7, value=4
        // -> target=3, which equals data_size_addr=3). A real exit
        // instruction can't legitimately sit exactly on the header
        // boundary; this must be rejected, not accepted with exit_ic ==
        // code_end (which would silently defeat any caller that truncates
        // a printed listing once it reaches exit_ic).
        let mut seg = vec![0u8; 17];
        seg[0] = 215;
        seg[1] = 193;
        seg[2] = 0;
        seg[3] = 10;
        seg[4] = 0;
        seg[5] = 0;
        seg[6] = 0;
        seg[7] = 4;
        seg[8] = 0; // corrupted ExitIC: target = 7-4 = 3 == code_end
        seg[9] = 9;
        seg[10] = 0;
        seg[11] = 1;
        seg[12] = 0;
        seg[13] = 2;
        seg[14] = 0;
        seg[15] = 1;
        seg[16] = 1;

        assert!(parse_procedure_dictionary(&seg).is_none());
    }

    #[test]
    fn real_hello_world_segment() {
        let contents = include_bytes!("../../../tests/HelloWorld.code");
        // segment 0: code_info = {address: 1, length: 0x70} -> bytes [512..624)
        let segment = &contents[512..512 + 0x70];

        let dict = parse_procedure_dictionary(segment).unwrap();
        assert_eq!(dict.segment_number, 1);
        assert_eq!(dict.procedures.len(), 1);
        let p = &dict.procedures[0];
        assert_eq!(p.number, 1);
        assert_eq!(p.lex_level, 0);
        assert_eq!(p.param_size, 4);
        assert_eq!(p.data_size, 82);
        assert_eq!(p.enter_ic, 0);
        assert_eq!(p.exit_ic, 0x25f - 0x200);
        assert_eq!(p.jtab_addr, p.code_end + 8);

        let instrs = disassemble(&segment[p.enter_ic..p.code_end]);
        // Full expected mnemonic sequence for this procedure's real code,
        // ground-truthed against the actual decode output rather than
        // hand-derived, so a regression anywhere in the middle of a real
        // multi-instruction stream (not just the opcodes covered in
        // isolation by the synthetic tests above) is caught.
        use Mnemonic::*;
        let expected = [
            NOP, NOP, LOD, LSA, NOP, SLDC, CXP, CSP, LOD, CXP, CSP, LOD, LAO, SLDC, CXP, CSP, LOD,
            CXP, CSP, LOD, NOP, LSA, SLDC, CXP, CSP, LOD, LAO, SLDC, CXP, CSP, LOD, CXP, CSP, RBP,
            SLDC,
        ];
        let actual: Vec<Mnemonic> = instrs.iter().map(|i| i.mnemonic).collect();
        assert_eq!(actual, expected);

        // ExitIC points at a real RBP instruction, confirming the whole
        // self-relative-pointer scheme end to end. code_end (J-8) lands one
        // byte past RBP's end here, on this segment's single word-alignment
        // pad byte, which decodes as a harmless extra SLDC(0) -- the
        // documented trailing-byte limitation of code_end, in its mildest
        // form (a lone pad byte rather than a real jump table).
        let rbp = instrs
            .iter()
            .find(|i| i.offset == p.exit_ic)
            .expect("RBP at exit_ic");
        assert_eq!(rbp.mnemonic, Mnemonic::RBP);
        assert_eq!(instrs.last().unwrap().mnemonic, Mnemonic::SLDC);
    }
}
