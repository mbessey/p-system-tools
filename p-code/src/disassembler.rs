// Decodes UCSD p-System / Apple Pascal "P-machine" bytecode, and the
// procedure dictionary that terminates each code segment. See the Apple
// Pascal Operating System Reference Manual appendices "ARCHITECTURE OF THE
// P-MACHINE" and "OPERATION OF THE P-MACHINE" for the source format.

pub mod decode;
pub mod instruction;
pub mod procedure_dict;
pub mod resolve;

pub use decode::disassemble;
pub use instruction::{Mnemonic, Operand, csp_name};
pub use procedure_dict::parse_procedure_dictionary;
pub use resolve::{assign_labels, jump_displacement, resolve_jump_target, trace_reachable};
