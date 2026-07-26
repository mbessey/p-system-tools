# p-code jump targets and standard-routine calls

The `p-code disassemble` command decodes raw p-machine instructions but
doesn't (yet) compute where a jump actually lands, or name which built-in
routine a `CSP`/`CXP` call reaches. This note works both out by hand,
against a real compiled program, so the reasoning and the numbers are
available to whoever tackles adding that to the tool itself.

Everything here was derived by compiling [`tests/Features.text`](../tests/Features.text)
on a real/emulated Apple Pascal system into a codefile, disassembling it with
`p-code disassemble --offsets --bytes`, and tracing the output line-by-line
back against the Pascal source. It's empirical, not transcribed from a spec
sheet -- treat the *reasoning* as durable and the specific numbers as
"true for this one compile" (see [Caveats](#caveats)).

## Jump instruction targets

The manual describes it this way:

> Simple (non-case statement) jumps are all two bytes long. The first byte
> is the op-code, the second is a SB jump offset. If this offset is
> non-negative, it is simply added to IPC. (A value of zero for the jump
> offset will make any jump a two-byte NOP.) If SB is negative, then SB DIV 2
> is used as a word offset into JTAB, and IPC is set to the byte address
> (JTAB^[SB DIV 2]) - contents of (JTAB[SB DIV 2]).

In other words, `UJP`/`FJP`/`XJP`'s embedded default all carry a signed byte
operand (`SB`) that means one of two completely different things depending
on its sign:

```
if SB >= 0:
    target = (instruction_address + instruction_length) + SB

else:  # SB < 0
    index      = SB / 2                    # exact -- SB is always even when negative
    jtab_addr  = <enclosing procedure's JTAB header address>
    slot_addr  = jtab_addr + SB             # SB negative, so this subtracts
    word_value = read_u16_le(segment, slot_addr)
    target     = slot_addr - word_value     # self-relative, same scheme JTAB's own header uses
```

**Why two schemes?** A small forward/backward hop within a few dozen bytes
fits comfortably in a signed byte, so the compiler just encodes it directly
(the non-negative case). But a jump needing to reach further than a signed
byte can express -- typically a loop back to its own condition test, several
dozen or hundred bytes earlier -- can't be encoded that way. Rather than
widen every jump instruction to carry a 16-bit operand (doubling the size of
the overwhelmingly common short/local jumps), the compiler reserves the
negative half of the signed byte's range as an *index* into a small table of
full-width backward targets (JTAB) that lives at a fixed, small negative
offset from each procedure's own JTAB header. This keeps ordinary jumps
one byte cheaper while still allowing arbitrarily distant backward targets.

`jtab_addr` for a procedure is exactly `code_end + 8` in
[`p-code/src/disassembler/procedure_dict.rs`](../p-code/src/disassembler/procedure_dict.rs)'s
terms -- that file already implements the *identical* self-relative-pointer
resolution for the JTAB header's own fields (`enter_ic`, `exit_ic`,
`param_size`, `data_size` are each stored as "distance from the address of
this word to the target," resolved via `resolve_self_relative`). The
negative-`SB` jump scheme is the same trick, just applied to a jump
instruction's operand instead of a dictionary-header field. `jtab_addr`
itself isn't currently exposed by that module, and nothing in the crate
computes a jump *target* today -- `disassemble.rs` only decodes and prints
the raw signed operand.

### Worked examples

From `Features.text`'s `GotoDemo`:

```pascal
PROCEDURE GotoDemo;
LABEL 1;
VAR K: INTEGER;
BEGIN
  K := 0;
  1: K := K + 1;
  WRITELN('  K = ', K);
  IF K < 3 THEN GOTO 1;
  WRITELN('  Done with GOTO demo')
END;
```

compiles to (procedure 6, `jtab_addr = 0x0142`):

```
0109  a1 02   FJP  2     -> target 0x010d   (skip GOTO when K>=3: simple, SB=2>=0)
010b  b9 f6   UJP  -10   -> target 0x00dd   (GOTO 1, via JTAB: slot 0x0142-10=0x0138 holds 0x005b, 0x0138-0x005b=0x00dd)
```

`0x00dd` is exactly where the labeled `K := K + 1` begins -- confirmed by
reading the actual bytes at `0x0138` in the compiled codefile.

The same negative-offset scheme applies to *conditional* jumps too, not just
`UJP` -- it's the offset's sign that selects the addressing mode, not the
mnemonic. `LoopDemo`'s `REPEAT...UNTIL K > 5` compiles its `UNTIL` test to a
*conditional, backward* jump (`FJP -16`), which resolves via the exact same
JTAB path back to the loop body.

One instructive non-match: `main`'s very first instruction is `UJP -10` at
segment offset `0x0ab2`. Resolving it via JTAB lands at `0x0d70`, which
falls *between* that procedure's `exit_ic` (`0x0d68`) and `code_end`
(`0x0d7e`) -- i.e. outside the range `disassemble` actually prints. That's
compiler-generated boilerplate (very likely a stack/heap-extend check
inserted at the entry of the outer block), not a source-level jump -- a
useful reminder that not every jump target lands inside the disassembly you
can see.

## CSP -- call standard procedure

`CSP N` invokes one of a small, fixed table of primitive built-ins by
number. `p-code`'s `csp_name` (in
[`p-code/src/disassembler/instruction.rs`](../p-code/src/disassembler/instruction.rs))
already names a handful; from this compile:

| N | Routine | Confidence |
|---|---|---|
| 4 | `EXIT` | confirmed (already in `csp_name`; matches `EXIT(PROGRAM)`) |
| 0 | `IOCHECK` | high -- appears after *every* `CXP` I/O call below; the source has no `{$I-}`, so the automatic runtime I/O-error check is active |
| 36 | `PWROFTEN` | high -- `SLDC 3 / CSP 36` immediately precedes where `PWROFTEN(3)`'s result is used; not in `csp_name` yet, but fits right after the table's highest existing entry (`35 => "POT"`) |

## CXP -- call external/segment procedure

`CXP seg,proc` calls procedure number `proc` in segment `seg`. Segment 0
holds a set of runtime string/I-O support routines; each entry below was
identified by tracing the call site against the exact Pascal statement it
implements:

| `seg,proc` | Routine |
|---|---|
| `0,13` | WRITE INTEGER(file, value, width) |
| `0,17` | WRITE CHAR(file, value, width) |
| `0,18` | READLN read-string(file, addr, maxlen) |
| `0,19` | WRITE STRING(file, addr, width) -- handles both string literals and string variables |
| `0,21` | READLN's "skip to end of line" step |
| `0,22` | WRITE newline (the "LN" of `WRITELN`, or a bare `WRITELN;`) |
| `0,23` | CONCAT-append primitive (called once per source argument) |
| `0,24` | INSERT(src, dest, maxlen, pos) |
| `0,25` | COPY(src, dest, start, count) |
| `0,26` | DELETE(str, start, count) |
| `0,27` | POS(needle, haystack) |
| `0,29` | GOTOXY(x, y) |
| `31,4` | WRITE REAL(file, value, width, decimals) -- confirmed via `SLDC 10 / SLDC 2 / CXP 31,4` matching `PWROFTEN(3):10:2` |
| `30,4` | likely the analogous extended-precision (`LongNum`) support routine -- only seen in `ShowLong`/`StringDemo`; medium confidence, not confirmed as tightly as the others |

A fun side-finding while tracing this: `ODD(7)`/`ODD(8)` on literal
constants compile to just `SLDC 7`/`SLDC 8` with no runtime computation at
all -- the compiler constant-folds `ODD` and relies on the p-machine's "any
nonzero is true" boolean convention rather than normalizing the result to a
strict 0/1.

## Reproducing or extending this analysis

1. Compile a `.text` source into a `.code` file on a real or emulated Apple
   Pascal system.
2. `p-code --code-file <file> disassemble --offsets --bytes > out.asm`
3. To resolve a specific negative-`SB` jump by hand: find its enclosing
   procedure's `code_end` from `disassemble`'s own reported `exit_ic` (its
   `jtab_addr` is `code_end + 8`, though `code_end` itself isn't printed
   today -- see `procedure_dict.rs`), then read the two bytes at
   `jtab_addr + SB` from the raw codefile and subtract that value from the
   address it was read from.
4. Cross-check any inferred `CSP`/`CXP` routine number against the *exact*
   Pascal statement compiled at that call site, the way this document does
   -- don't trust a routine number in isolation without matching it to
   source.

## Caveats

- Everything numbered above reflects **this one compile** of
  `tests/Features.text` (which, at the time of this analysis, didn't yet
  include `ArrayRecordDemo`, `TreeDemo`/`AddName`, or `FileDemo`). Standard
  routine numbers are a property of the compiler/runtime, not the source
  language, so they should be stable across programs compiled by the same
  Apple Pascal version -- but haven't been checked against a different
  compiler version or a real UCSD p-System (non-Apple) implementation.
- The case-statement (`XJP`) per-entry jump table uses a different encoding
  than the simple two-byte jumps described here (word-sized table entries,
  sometimes routed through small per-value "trampoline" stubs when several
  case labels share one body) -- this document doesn't attempt to nail down
  that table's exact addressing convention.
