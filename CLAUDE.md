# CLAUDE.md

## About this project

p-system-tools exists to help people learn about and understand the UCSD
p-System and the p-machine (the virtual machine Apple Pascal and other
p-System implementations run on). It's meant to be a friendly, approachable
project for new volunteers, including people who have never touched this
codebase — or the p-System — before. Optimize for the next reader's
understanding, not just for working code.

## Repository structure

A Rust workspace with three crates:

- **p-filer** — reads and writes Apple Pascal disk images (`.dsk` files):
  sector interleaving, the volume directory format, file transfer in/out.
- **p-code** — parses and disassembles UCSD p-System codefiles (compiled
  Pascal object files) and their segment dictionary / p-code instructions.
- **p-system-format** — format-parsing primitives shared by both crates
  (bounds-checked byte/Pascal-string/date encoding and decoding).

`tests/` holds fixture files used by both crates (sample `.dsk` images,
`.pas`/`.code` files) — not a separate crate.

## Documentation standards

Because this is a learning resource, documentation quality matters as much as
correctness:

- **Keep comments and the README up to date.** When behavior changes, update
  the comments and README describing it in the same change — never leave
  them describing the old behavior.
- **Explain *why*, not just *what*.** Code and identifier names already say
  what a line does; a comment that only restates that adds nothing. Write
  comments that explain the reasoning: why this approach and not the obvious
  alternative, what real p-System behavior or file-format quirk is being
  matched, what invariant would break if this changed.
- **Call out unfinished work explicitly.** If something is a stub, a
  deliberate simplification, or skips a case on purpose, say so in the
  comment (e.g. "stub — not yet implemented", a `TODO:` note) and, where it's
  user-facing, in the README's command tables too. Never let a silent gap
  look like a finished feature.

## File header comments

Every source file should start with a doc comment, in plain language,
covering:
- What the file is for / what problem it solves
- What types it defines and what they represent
- How it fits into the rest of the crate, if that's not obvious from its
  name or path

## Function doc comments

Every function should have a doc comment covering:
- What each argument means (not just its type)
- What the return value represents, including any special-cased results
- What errors it can return and under what conditions
- Any prerequisites the caller must satisfy, and any invariants the function
  relies on or maintains

## Before committing

Always run `cargo fmt` and `cargo clippy --workspace --all-targets` before
committing, and fix any warnings they raise.

CI tracks the latest stable Rust with no pinned toolchain and runs clippy
with `-D warnings`, so a clean run today can still fail in CI later if a new
stable release adds a lint. If that happens, update your local toolchain
(`rustup update stable`) to match and fix it there rather than guessing.
