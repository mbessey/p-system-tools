# p-code jump targets and standard-routine calls

The `p-code disassemble` command resolves where a jump (`UJP`/`FJP`/`EFJ`/
`NFJ`, and `XJP`'s embedded `default` field) actually lands, printing a
label instead of the raw displacement, and can print reachable code past a
procedure's `exit_ic` when a jump target lands there. It doesn't (yet) name
which built-in routine most `CSP`/`CXP` calls reach. This note works both
problems out by hand, against a real compiled program -- it's the
derivation the jump-resolution implementation is based on, and a reference
for anyone extending it further (naming more `CSP`/`CXP` routines below, or
tackling `XJP`'s per-case table).

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
instruction's operand instead of a dictionary-header field. `jtab_addr` is
exposed as `ProcedureInfo::jtab_addr`, and
[`p-code/src/disassembler/resolve.rs`](../p-code/src/disassembler/resolve.rs)'s
`resolve_jump_target` implements exactly this arithmetic --
`p-code/src/commands/disassemble.rs` uses it to print jump operands as
labels instead of raw signed displacements (`--no-labels` turns this back
off).

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

### Live code can exist past `exit_ic`

`main`'s very first instruction is `UJP -10` at segment offset `0x0ab2`.
Resolving it via JTAB lands at `0x0d70`, which falls *between* that
procedure's `exit_ic` (`0x0d68`) and `code_end` (`0x0d7e`) -- past the
`[enter_ic, exit_ic)` range `disassemble` always prints unconditionally
(see `p-code/src/commands/disassemble.rs`'s `print_instructions` doc
comment for exactly where that unconditional range ends -- `exit_ic`
itself is now reachability-gated, not automatically included). That
first jump landing there isn't a fluke, and it isn't dead space either. A
full recursive-descent trace (see
[the next section](#how-disassemble-finds-and-prints-code-past-exit_ic))
resolves what's actually reachable more precisely than this document's
original hand-decoding did:

```
0d68  SLDC 31        ; dead -- nothing jumps here, and CSP 4 {EXIT} above
0d69  CSP 22         ;   doesn't fall through into it (EXIT halts outright)
0d6b  SLDC 30        ; dead, same reason
0d6c  CSP 22         ; dead
0d6e  UJP 8          ; dead
0d70  SLDC 30        ; LIVE -- this is where main's leading UJP -10 lands
0d71  CSP 21
0d73  SLDC 31
0d74  CSP 21
0d76  UJP -12  -> 0x0ab4   ; loops back to main's real body, NOT 0x0d6c
0d78  RBP 0         ; dead -- nothing ever jumps back here either
0d7a  LLA 2         ; dead
0d7c  SLDC 12       ; dead
0d7d  SLDC 0        ; dead
```

The values being pushed -- **30 and 31** -- are exactly the two intrinsic
segment numbers identified in the [CXP table](#cxp----call-externalsegment-procedure)
below (`LongNum`/`STR` support and formatted-`REAL`-write support,
respectively). This is program-startup code: a one-time check/link step for
the intrinsic segments this specific program depends on, run once via the
backward jump from the procedure's true entry point. A *second* jump
(`0x0d76`'s `UJP -12`) then hands control straight back to `0x0ab4` -- the
byte right after the leading `UJP` -- to run the program's real body, now
that the link check is guaranteed to have happened.

**Correction from an earlier draft of this document:** this section
originally hand-decoded `0x0d76`'s `UJP -12` as landing on `0x0d6c`
("rejoining PATH A partway through"), and treated `0x0d78`'s `RBP 0` as the
point every path converges on. Implementing `resolve_jump_target` and
cross-checking it against the raw JTAB slot bytes at `0x0d7a` showed that
was a hand-arithmetic slip -- the real target is `0x0ab4`. That changes the
conclusion substantially: `0x0d68`-`0x0d6e` ("PATH A") and `0x0d78`-`0x0d7d`
(the "epilogue") are *both* dead -- nothing in this procedure ever jumps to
either range, and `CSP 4 {EXIT}` genuinely halts the interpreter rather
than falling through. Only `0x0d70`-`0x0d77` (the segment-link check
itself) is live. The epilogue bytes exist only because the compiler always
emits a procedure-exit sequence, whether or not this particular
procedure's control flow ever reaches it by falling off the end -- `main`
never does, since it always terminates via the explicit `EXIT(PROGRAM)`
call partway through its body.

It sits past `exit_ic` for the structural reason this document originally
identified: `exit_ic` marks the address right after the procedure's last
*source-level* instruction, which for `main` is the `CSP 4 {EXIT}` call
(`EXIT(PROGRAM)`, at `0x0d66`, ending at `0x0d68`) -- and on a real
p-machine, that call halts the interpreter outright and never falls
through to whatever bytes follow it. From the compiler's point of view,
that makes the space right after it "dead" in the normal fall-through
sense, and thus free real estate to tuck away code that's only ever
reached via a deliberate jump.

This also explains why [`tests/HelloWorld.code`](../tests/HelloWorld.code)
has no such prologue: its source (`tests/HelloWorld.pas`) uses only plain
`STRING`/`WRITELN`/`READLN` -- no `REAL` field-width formatting, no
extended-precision `LongNum`. It has zero dependency on segments 30 or 31,
so there's nothing to check or link at startup, and the compiler has no
reason to generate this prologue at all.

The general lesson: not every jump target lands inside the range
`disassemble` prints unconditionally, and when one doesn't, that's a sign
there's more *reachable* code past `exit_ic` -- `disassemble` now finds and
prints it automatically (see below), rather than requiring hand-decoding
the way this document originally did.

## How `disassemble` finds and prints code past `exit_ic`

Implemented in
[`p-code/src/disassembler/resolve.rs`](../p-code/src/disassembler/resolve.rs)'s
`trace_reachable`: recursive-descent control-flow tracing, not a linear
sweep with a fixed cutoff -- a naive sweep to `code_end` is unsound for the
reason this document already knew: `code_end` can include a stretch of
pure alignment padding, or a raw `XJP` case-jump table (word-sized data,
not opcodes at all), and decoding those bytes as if they were instructions
produces plausible-looking garbage with no way to tell it's garbage.

Concretely: starting from `enter_ic`, it decodes forward; whenever a
`UJP`/`FJP`/`EFJ`/`NFJ` is reached, it resolves the target using the
algorithm above and adds it to a work-list of "known to be a real
instruction start" if it hasn't been visited yet (a conditional jump also
keeps its own fall-through successor). `RNP`/`RBP` (return -- the
p-machine splits "return" into two opcodes by lexical level, but both end
the procedure) and `CSP` calling `EXIT` specifically are also treated as
having no fall-through, exactly like `UJP`/`XJP` -- `main`'s own worked
example above is what surfaced the need for the `EXIT` refinement:
naively granting `CSP {EXIT}` a fall-through wrongly marked its dead
duplicate check code ("PATH A") as reachable, since it's the very next
byte after `EXIT` in program order. `XJP`'s per-case `offsets` table is
never followed (see [Caveats](#caveats)); only its embedded `default`
field is resolved, the same as a plain `UJP`/`FJP`. Whatever addresses the
trace visits are provably reachable code; anything else in `[exit_ic,
code_end)` that no traced jump ever points at is left unprinted -- most
likely padding or a jump table, but not something to guess-decode.

`p-code/src/commands/disassemble.rs`'s `print_instructions` keeps printing
`[enter_ic, exit_ic)` unconditionally in program order, exactly as
before -- but the instruction sitting *at* `exit_ic` itself is only kept
if the trace proves it's reachable. For most procedures that's a no-op:
`exit_ic` lands on a normal return (`RBP`/`RNP`), always reached by
ordinary fall-through from the body above it. But `main`'s `exit_ic`
lands on the dead `SLDC 31` at the top of "PATH A" -- provably
unreachable, and now correctly left out, so a procedure whose only live
tail code is `main`'s segment-link check shows exactly that (and nothing
else at or past `exit_ic`).

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
   -- jump targets are now labeled and reachable code past `exit_ic` is
   printed automatically; pass `--no-labels` to see the old raw-displacement
   view if you want to re-derive a target by hand for cross-checking.
3. To resolve a specific negative-`SB` jump by hand anyway (e.g. to verify
   the tool, the way the correction above was found): find its enclosing
   procedure's `code_end` from `disassemble`'s own reported `exit_ic` (its
   `jtab_addr` is `code_end + 8`, though `code_end` itself isn't printed
   today -- see `procedure_dict.rs`), then read the two bytes at
   `jtab_addr + SB` from the raw codefile and subtract that value from the
   address it was read from.
4. Cross-check any inferred `CSP`/`CXP` routine number against the *exact*
   Pascal statement compiled at that call site, the way this document does
   -- don't trust a routine number in isolation without matching it to
   source. `CSP`/`CXP` routine naming is still a manual, by-hand process --
   only jump-target resolution has been automated.

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
