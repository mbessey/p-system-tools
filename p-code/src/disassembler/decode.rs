use super::instruction::{Instruction, Mnemonic, Operand};

// Word: next two bytes, low byte first.
pub(crate) fn read_word(code: &[u8], pos: usize) -> Option<u16> {
    p_system_format::bytes::read_u16_le(code, pos)
}

// "B" format: one byte if 0..127, else two bytes with bit 7 of the first
// byte cleared to form the high byte. Returns (value, bytes consumed).
fn read_big(code: &[u8], pos: usize) -> Option<(u16, usize)> {
    let b0 = *code.get(pos)?;
    if b0 <= 127 {
        Some((b0 as u16, 1))
    } else {
        let b1 = *code.get(pos + 1)?;
        Some(((((b0 & 0x7F) as u16) << 8) | b1 as u16, 2))
    }
}

fn embedded(offset: usize, mnemonic: Mnemonic, value: u8) -> Instruction {
    Instruction {
        offset,
        bytes_len: 1,
        mnemonic,
        operand: Operand::Embedded(value),
    }
}

fn none(offset: usize, mnemonic: Mnemonic) -> Option<Instruction> {
    Some(Instruction {
        offset,
        bytes_len: 1,
        mnemonic,
        operand: Operand::None,
    })
}

fn u8_operand(code: &[u8], offset: usize, mnemonic: Mnemonic) -> Option<Instruction> {
    let v = *code.get(offset + 1)?;
    Some(Instruction {
        offset,
        bytes_len: 2,
        mnemonic,
        operand: Operand::U8(v),
    })
}

fn i8_operand(code: &[u8], offset: usize, mnemonic: Mnemonic) -> Option<Instruction> {
    let v = *code.get(offset + 1)? as i8;
    Some(Instruction {
        offset,
        bytes_len: 2,
        mnemonic,
        operand: Operand::I8(v),
    })
}

fn big_operand(code: &[u8], offset: usize, mnemonic: Mnemonic) -> Option<Instruction> {
    let (v, len) = read_big(code, offset + 1)?;
    Some(Instruction {
        offset,
        bytes_len: 1 + len,
        mnemonic,
        operand: Operand::Big(v),
    })
}

fn word_operand(code: &[u8], offset: usize, mnemonic: Mnemonic) -> Option<Instruction> {
    let v = read_word(code, offset + 1)?;
    Some(Instruction {
        offset,
        bytes_len: 3,
        mnemonic,
        operand: Operand::Word(v),
    })
}

fn u8_big(code: &[u8], offset: usize, mnemonic: Mnemonic) -> Option<Instruction> {
    let a = *code.get(offset + 1)?;
    let (b, len) = read_big(code, offset + 2)?;
    Some(Instruction {
        offset,
        bytes_len: 2 + len,
        mnemonic,
        operand: Operand::U8Big(a, b),
    })
}

fn u8_u8(code: &[u8], offset: usize, mnemonic: Mnemonic) -> Option<Instruction> {
    let a = *code.get(offset + 1)?;
    let b = *code.get(offset + 2)?;
    Some(Instruction {
        offset,
        bytes_len: 3,
        mnemonic,
        operand: Operand::U8U8(a, b),
    })
}

fn string_data(code: &[u8], offset: usize, mnemonic: Mnemonic) -> Option<Instruction> {
    let len = *code.get(offset + 1)? as usize;
    let start = offset + 2;
    let chars = code.get(start..start + len)?.to_vec();
    Some(Instruction {
        offset,
        bytes_len: 2 + len,
        mnemonic,
        operand: Operand::StringData(chars),
    })
}

fn word_data(code: &[u8], offset: usize, mnemonic: Mnemonic) -> Option<Instruction> {
    let words = *code.get(offset + 1)? as usize;
    let byte_len = words * 2;
    let start = offset + 2;
    let block = code.get(start..start + byte_len)?.to_vec();
    Some(Instruction {
        offset,
        bytes_len: 2 + byte_len,
        mnemonic,
        operand: Operand::WordData(block),
    })
}

// EQU/GEQ/GRT/LEQ/LES/NEQ: UB selects the compared type; an extra B byte
// count follows only when UB is 10 (byte arrays) or 12 (words/records).
fn type_compare(code: &[u8], offset: usize, mnemonic: Mnemonic) -> Option<Instruction> {
    let mut pos = offset + 1;
    let type_sel = *code.get(pos)?;
    pos += 1;
    let extra = if type_sel == 10 || type_sel == 12 {
        let (big, len) = read_big(code, pos)?;
        pos += len;
        Some(big)
    } else {
        None
    };
    Some(Instruction {
        offset,
        bytes_len: pos - offset,
        mnemonic,
        operand: Operand::TypeCompare(type_sel, extra),
    })
}

fn decode_xjp(code: &[u8], offset: usize) -> Option<Instruction> {
    let mut pos = offset + 1;
    let low = read_word(code, pos)? as i16;
    pos += 2;
    let high = read_word(code, pos)? as i16;
    pos += 2;
    if *code.get(pos)? != 185 {
        return None; // embedded default jump must be a UJP instruction (opcode 185)
    }
    pos += 1;
    let default = *code.get(pos)? as i8;
    pos += 1;
    let count = (high - low + 1).max(0) as usize;
    if count > code.len().saturating_sub(pos) / 2 {
        return None; // guard against a corrupt min/max requesting a huge allocation
    }
    let mut offsets = Vec::with_capacity(count);
    for _ in 0..count {
        offsets.push(read_word(code, pos)? as i16);
        pos += 2;
    }
    Some(Instruction {
        offset,
        bytes_len: pos - offset,
        mnemonic: Mnemonic::XJP,
        operand: Operand::CaseJump {
            low,
            high,
            default,
            offsets,
        },
    })
}

pub fn decode_one(code: &[u8], offset: usize) -> Option<Instruction> {
    use Mnemonic::*;
    let opcode = *code.get(offset)?;
    match opcode {
        0..=127 => Some(embedded(offset, SLDC, opcode)),
        128 => none(offset, ABI),
        129 => none(offset, ABR),
        130 => none(offset, ADI),
        131 => none(offset, ADR),
        132 => none(offset, LAND),
        133 => none(offset, DIF),
        134 => none(offset, DVI),
        135 => none(offset, DVR),
        136 => none(offset, CHK),
        137 => none(offset, FLO),
        138 => none(offset, FLT),
        139 => none(offset, INN),
        140 => none(offset, INT),
        141 => none(offset, LOR),
        142 => none(offset, MODI),
        143 => none(offset, MPI),
        144 => none(offset, MPR),
        145 => none(offset, NGI),
        146 => none(offset, NGR),
        147 => none(offset, LNOT),
        148 => none(offset, SRS),
        149 => none(offset, SBI),
        150 => none(offset, SBR),
        151 => none(offset, SGS),
        152 => none(offset, SQI),
        153 => none(offset, SQR),
        154 => none(offset, STO),
        155 => none(offset, IXS),
        156 => none(offset, UNI),
        157 => u8_big(code, offset, LDE),
        158 => u8_operand(code, offset, CSP),
        159 => none(offset, LDCN),
        160 => u8_operand(code, offset, ADJ),
        161 => i8_operand(code, offset, FJP),
        162 => big_operand(code, offset, INC),
        163 => big_operand(code, offset, IND),
        164 => big_operand(code, offset, IXA),
        165 => big_operand(code, offset, LAO),
        166 => string_data(code, offset, LSA),
        167 => u8_big(code, offset, LAE),
        168 => big_operand(code, offset, MOV),
        169 => big_operand(code, offset, LDO),
        170 => u8_operand(code, offset, SAS),
        171 => big_operand(code, offset, SRO),
        172 => decode_xjp(code, offset),
        173 => u8_operand(code, offset, RNP),
        174 => u8_operand(code, offset, CIP),
        175 => type_compare(code, offset, EQU),
        176 => type_compare(code, offset, GEQ),
        177 => type_compare(code, offset, GRT),
        178 => u8_big(code, offset, LDA),
        179 => word_data(code, offset, LDC),
        180 => type_compare(code, offset, LEQ),
        181 => type_compare(code, offset, LES),
        182 => u8_big(code, offset, LOD),
        183 => type_compare(code, offset, NEQ),
        184 => u8_big(code, offset, STR),
        185 => i8_operand(code, offset, UJP),
        186 => none(offset, LDP),
        187 => none(offset, STP),
        188 => u8_operand(code, offset, LDM),
        189 => u8_operand(code, offset, STM),
        190 => none(offset, LDB),
        191 => none(offset, STB),
        192 => u8_u8(code, offset, IXP),
        193 => u8_operand(code, offset, RBP),
        194 => u8_operand(code, offset, CBP),
        195 => none(offset, EQUI),
        196 => none(offset, GEQI),
        197 => none(offset, GRTI),
        198 => big_operand(code, offset, LLA),
        199 => word_operand(code, offset, LDCI),
        200 => none(offset, LEQI),
        201 => none(offset, LESI),
        202 => big_operand(code, offset, LDL),
        203 => none(offset, NEQI),
        204 => big_operand(code, offset, STL),
        205 => u8_u8(code, offset, CXP),
        206 => u8_operand(code, offset, CLP),
        207 => u8_operand(code, offset, CGP),
        208 => string_data(code, offset, LPA),
        209 => u8_big(code, offset, STE),
        210 => Some(Instruction {
            offset,
            bytes_len: 1,
            mnemonic: UNKNOWN,
            operand: Operand::None,
        }),
        211 => i8_operand(code, offset, EFJ),
        212 => i8_operand(code, offset, NFJ),
        213 => big_operand(code, offset, BPT),
        214 => none(offset, XIT),
        215 => none(offset, NOP),
        216..=231 => Some(embedded(offset, SLDL, opcode - 215)),
        232..=247 => Some(embedded(offset, SLDO, opcode - 231)),
        248 => Some(embedded(offset, SIND, 0)),
        249..=255 => Some(embedded(offset, SIND, opcode - 248)),
    }
}

pub fn disassemble(code: &[u8]) -> Vec<Instruction> {
    let mut result = Vec::new();
    let mut offset = 0;
    while offset < code.len() {
        match decode_one(code, offset) {
            Some(instr) => {
                offset += instr.bytes_len;
                result.push(instr);
            }
            None => break,
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::super::instruction::csp_name;
    use super::*;

    #[test]
    fn no_operand_opcode() {
        let i = decode_one(&[128], 0).unwrap();
        assert_eq!(i.mnemonic, Mnemonic::ABI);
        assert_eq!(i.bytes_len, 1);
    }

    #[test]
    fn sldc_range_boundaries() {
        let i = decode_one(&[0], 0).unwrap();
        assert_eq!(i.mnemonic, Mnemonic::SLDC);
        assert!(matches!(i.operand, Operand::Embedded(0)));

        let i = decode_one(&[127], 0).unwrap();
        assert_eq!(i.mnemonic, Mnemonic::SLDC);
        assert!(matches!(i.operand, Operand::Embedded(127)));
    }

    #[test]
    fn sldl_range_boundaries() {
        let i = decode_one(&[216], 0).unwrap();
        assert_eq!(i.mnemonic, Mnemonic::SLDL);
        assert!(matches!(i.operand, Operand::Embedded(1)));

        let i = decode_one(&[231], 0).unwrap();
        assert_eq!(i.mnemonic, Mnemonic::SLDL);
        assert!(matches!(i.operand, Operand::Embedded(16)));
    }

    #[test]
    fn sldo_range_boundaries() {
        let i = decode_one(&[232], 0).unwrap();
        assert_eq!(i.mnemonic, Mnemonic::SLDO);
        assert!(matches!(i.operand, Operand::Embedded(1)));

        let i = decode_one(&[247], 0).unwrap();
        assert_eq!(i.mnemonic, Mnemonic::SLDO);
        assert!(matches!(i.operand, Operand::Embedded(16)));
    }

    #[test]
    fn sind_range_boundaries() {
        let i = decode_one(&[248], 0).unwrap();
        assert_eq!(i.mnemonic, Mnemonic::SIND);
        assert!(matches!(i.operand, Operand::Embedded(0)));

        let i = decode_one(&[249], 0).unwrap();
        assert!(matches!(i.operand, Operand::Embedded(1)));

        let i = decode_one(&[255], 0).unwrap();
        assert!(matches!(i.operand, Operand::Embedded(7)));
    }

    #[test]
    fn u8_and_i8_operands() {
        let i = decode_one(&[160, 5], 0).unwrap();
        assert_eq!(i.mnemonic, Mnemonic::ADJ);
        assert!(matches!(i.operand, Operand::U8(5)));
        assert_eq!(i.bytes_len, 2);

        let i = decode_one(&[185, 0xFE], 0).unwrap();
        assert_eq!(i.mnemonic, Mnemonic::UJP);
        assert!(matches!(i.operand, Operand::I8(-2)));
    }

    #[test]
    fn big_operand_one_and_two_byte_forms() {
        let i = decode_one(&[165, 3], 0).unwrap();
        assert_eq!(i.mnemonic, Mnemonic::LAO);
        assert!(matches!(i.operand, Operand::Big(3)));
        assert_eq!(i.bytes_len, 2);

        let i = decode_one(&[165, 0x81, 0x02], 0).unwrap();
        assert!(matches!(i.operand, Operand::Big(258)));
        assert_eq!(i.bytes_len, 3);
    }

    #[test]
    fn u8_big_operand() {
        // matches real bytes from tests/HelloWorld.code: b6 01 03 -> LOD(1,3)
        let i = decode_one(&[182, 1, 3], 0).unwrap();
        assert_eq!(i.mnemonic, Mnemonic::LOD);
        assert!(matches!(i.operand, Operand::U8Big(1, 3)));
        assert_eq!(i.bytes_len, 3);
    }

    #[test]
    fn u8_u8_operand() {
        // matches real bytes from tests/HelloWorld.code: cd 00 13 -> CXP(0,19)
        let i = decode_one(&[205, 0, 19], 0).unwrap();
        assert_eq!(i.mnemonic, Mnemonic::CXP);
        assert!(matches!(i.operand, Operand::U8U8(0, 19)));
        assert_eq!(i.bytes_len, 3);
    }

    #[test]
    fn string_data_operand() {
        let i = decode_one(&[166, 3, b'a', b'b', b'c'], 0).unwrap();
        assert_eq!(i.mnemonic, Mnemonic::LSA);
        match &i.operand {
            Operand::StringData(bytes) => assert_eq!(bytes, b"abc"),
            _ => panic!("wrong operand variant"),
        }
        assert_eq!(i.bytes_len, 5);
    }

    #[test]
    fn word_data_operand() {
        let i = decode_one(&[179, 2, 0, 1, 2, 3], 0).unwrap();
        assert_eq!(i.mnemonic, Mnemonic::LDC);
        match &i.operand {
            Operand::WordData(bytes) => assert_eq!(bytes, &[0, 1, 2, 3]),
            _ => panic!("wrong operand variant"),
        }
        assert_eq!(i.bytes_len, 6);
    }

    #[test]
    fn csp_naming() {
        let i = decode_one(&[158, 1], 0).unwrap();
        assert_eq!(i.mnemonic, Mnemonic::CSP);
        assert!(matches!(i.operand, Operand::U8(1)));
        assert_eq!(csp_name(1), Some("NEW"));
        assert_eq!(csp_name(99), None);
    }

    #[test]
    fn type_compare_family() {
        let i = decode_one(&[175, 2], 0).unwrap();
        assert!(matches!(i.operand, Operand::TypeCompare(2, None)));
        assert_eq!(i.bytes_len, 2);

        let i = decode_one(&[175, 10, 4], 0).unwrap();
        assert!(matches!(i.operand, Operand::TypeCompare(10, Some(4))));
        assert_eq!(i.bytes_len, 3);

        let i = decode_one(&[175, 12, 0x81, 0x02], 0).unwrap();
        assert!(matches!(i.operand, Operand::TypeCompare(12, Some(258))));
        assert_eq!(i.bytes_len, 4);

        // sibling opcode, no-extra-byte branch
        let i = decode_one(&[183, 6], 0).unwrap();
        assert_eq!(i.mnemonic, Mnemonic::NEQ);
        assert!(matches!(i.operand, Operand::TypeCompare(6, None)));
    }

    #[test]
    fn xjp_case_table() {
        let bytes = [172, 1, 0, 2, 0, 185, 0xFF, 10, 0, 20, 0];
        let i = decode_one(&bytes, 0).unwrap();
        assert_eq!(i.mnemonic, Mnemonic::XJP);
        assert_eq!(i.bytes_len, 11);
        match i.operand {
            Operand::CaseJump {
                low,
                high,
                default,
                offsets,
            } => {
                assert_eq!(low, 1);
                assert_eq!(high, 2);
                assert_eq!(default, -1);
                assert_eq!(offsets, vec![10, 20]);
            }
            _ => panic!("wrong operand variant"),
        }
    }

    #[test]
    fn xjp_rejects_invalid_default_jump_marker() {
        // same bytes as xjp_case_table, but the embedded "UJP" marker byte
        // (must be 185) is corrupted to 0
        let bytes = [172, 1, 0, 2, 0, 0, 0xFF, 10, 0, 20, 0];
        assert!(decode_one(&bytes, 0).is_none());
    }

    #[test]
    fn reserved_opcode_210() {
        let i = decode_one(&[210], 0).unwrap();
        assert_eq!(i.mnemonic, Mnemonic::UNKNOWN);
        assert!(matches!(i.operand, Operand::None));
        assert_eq!(i.bytes_len, 1);
    }

    #[test]
    fn truncated_input_returns_none() {
        assert!(decode_one(&[165], 0).is_none()); // LAO missing its B byte
        assert!(decode_one(&[172], 0).is_none()); // XJP truncated
        assert!(decode_one(&[], 0).is_none());
    }

    #[test]
    fn disassemble_stops_cleanly_on_trailing_truncation() {
        // ADI (no operand) followed by a truncated LAO (needs a B byte)
        let code = [130, 165];
        let instrs = disassemble(&code);
        assert_eq!(instrs.len(), 1);
        assert_eq!(instrs[0].mnemonic, Mnemonic::ADI);
    }

    #[test]
    fn disassemble_loop_offsets() {
        let code = [128, 128, 160, 5];
        let instrs = disassemble(&code);
        assert_eq!(instrs.len(), 3);
        assert_eq!(instrs[0].offset, 0);
        assert_eq!(instrs[1].offset, 1);
        assert_eq!(instrs[2].offset, 2);
    }
}
