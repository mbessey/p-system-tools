enum Argument {
    None,         // all arguments implicit
    Byte,         // a byte, from -128 to 127
    UnsignedByte, // a byte, from 0-255
    DontCareByte, // can be signed or unsigned. We don't care always 0-127
    Big,          // if 0-127, one byte, otherwise 2 bytes 128-32767
    Word,         // a word, LSB first
    Special       // For LSA and other variable-length instructions
}

struct Instruction {
    opcode: u8,
    name: String,
    args: Argument
}

fn instructions() -> Vec<Instruction> {
    let mut insts = Vec::new();
    for i in 0..127 {
        insts.push(Instruction {
            opcode: i,
            name: format!("SLDC {}", i),
            args: Argument::None
        });
    }
    return insts;
}

pub fn disassembly(bytes: &[u8], offset: u16, size: u16) -> String {
    let mut output:String = String::new();
    let mut i = 0;
    let instructions = instructions();
    while i < size {
        let byte = bytes[(offset+i) as usize];
        if let Some(instruction) = instructions.get(byte as usize) {
            output += &instruction.name;
            let mut arg_size = 0;
            match instruction.args {
                Argument::None => arg_size = 0,
                Argument::Byte => arg_size = 1,
                Argument::UnsignedByte => arg_size = 1,
                Argument::DontCareByte => arg_size = 1,
                Argument::Big => todo!(),
                Argument::Word => arg_size = 2,
                Argument::Special => todo!(),
            }
            i = i + 1 + arg_size;
        } else {
            output += "Unknown\n";
            i = i + 1;
        }
    }
    return output;
}
