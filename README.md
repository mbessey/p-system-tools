# p-system-tools

Tools for interacting with the UCSD p-System virtual machine, and specifically Apple Pascal.

This is a Rust workspace containing two command-line utilities:

- **[p-filer](p-filer)** — inspects and manipulates Apple Pascal disk images (`.dsk` files).
- **[p-code](p-code)** — inspects UCSD p-System code files (linked object files, produced by the Pascal compiler/linker).

Both tools are early-stage / work-in-progress (see [Status](#status) below).

## Building

Requires a recent Rust toolchain (edition 2024).

```sh
cargo build
```

This builds both crates as part of the workspace. Binaries are written to `target/debug/`.

To build and run a specific tool directly:

```sh
cargo run -p p-filer -- <args>
cargo run -p p-code -- <args>
```

## p-filer

Reads and writes Apple Pascal disk images. Handles the interleaved sector ordering used by Apple II `.dsk` images, and understands the p-System volume directory format.

```
p-filer --image <IMAGE> <COMMAND>
```

`--image` / `-i` is the path to the disk image file, required for every command.

`--verbose` / `-v` prints extra diagnostic information (currently: the disk's track/sector layout as it's loaded).

### Commands

| Command | Description |
|---|---|
| `list` | Print volume info (name, size, date) and every directory entry (name, type, block range, size, date). |
| `remove <name>` | Delete a file from the volume: removes its directory entry and saves the image. The file's blocks aren't zeroed, only freed — they become part of a gap that a later `transfer --to-image` or `krunch` can reuse or close. |
| `transfer <name> [--to-image] [--text] [-p, --preserve-date]` | Copy a file between the disk image and the host filesystem. By default copies **from** the image to the current directory; pass `--to-image` to copy a host file **to** the image instead, allocating it into the first large-enough gap in the volume's free space and saving the image back to disk (a `.bak` backup of the previous state is written alongside it). Only `name`'s final path component is used as the volume filename (so a full host path works fine), uppercased and limited to 15 characters, matching p-System volume filename conventions. Pass `--text` to convert p-System text file encoding (CR line endings, run-length-encoded indentation) to/from plain LF text. Pass `--preserve-date`/`-p` to sync the modification time between the extracted host file and the file's date on the volume, in whichever direction the copy is going. |
| `change <from> <to>` | Rename a file on the volume. `to` is uppercased and limited to 15 characters, the same convention `transfer --to-image` uses for a new volume filename. |
| `krunch` | Consolidate free space on the volume by sliding every file down to close any gap before it, merging all free space into one region at the volume's tail. |
| `zero [new_name]` | Clear the volume directory, marking every file as deleted. File blocks themselves aren't touched — only the directory's file count is reset, so the data is still physically present (and unrecoverable through this tool) until new files overwrite it. Optionally pass `new_name` to also rename the volume itself while zeroing it, matching the real Filer's Zero command; it's uppercased and limited to 7 characters (shorter than a file's 15-character limit). |
| `dump <from> <to>` | Hex/ASCII dump of disk blocks `from` through `to` (inclusive). |

### Examples

```sh
# List the contents of a disk image
p-filer --image my-disk.dsk list

# Extract a text file, converting it to plain text and preserving its date
p-filer --image my-disk.dsk transfer HELLO.TEXT --text --preserve-date

# Dump blocks 0-5 (the boot blocks and directory) as hex/ASCII
p-filer --image my-disk.dsk dump 0 5
```

Sample `.dsk` images are available in [`tests/AppleDsks`](tests/AppleDsks) (see [Tests](#tests) below), or supply your own Apple Pascal disk image.

## p-code

Parses UCSD p-System "codefiles" — the linked object files produced by compiling a Pascal program — and reports on their segment dictionary.

```
p-code --code-file <CODE_FILE> <COMMAND>
```

`--code-file` / `-c` is the path to the codefile, required for every command.

`--verbose` / `-v` prints extra diagnostic information (currently: the size of the in-memory segment dictionary layout).

### Commands

| Command | Description |
|---|---|
| `list` | Print the file's copyright string and a table of its segments (name, address, length, kind, and packed segment-info: unit number, code type, version). |
| `disassemble [--file-offsets] [--offsets] [--bytes] [--no-labels] [--footers]` | Disassemble the p-code in the file. Within a segment, procedures are printed in code-address order (not procedure-number order), so offsets always increase top-to-bottom. `CXP` (call external procedure) operands are annotated with the target segment's name when it can be resolved: `{SYSTEM}` for unit 0 (System.Library — not present in the codefile, so never further resolved), or the codefile's own segment name for units 1–15 that are active in its own segment dictionary. Pass `--file-offsets` to show each instruction's offset within the whole codefile; pass `--offsets` to show each instruction's offset within its segment (shown to the right of `--file-offsets` when both are given); pass `--bytes` to show each instruction's raw hex bytes; pass `--no-labels` to turn off jump-target labels and resolved-target operand text (shown by default); pass `--footers` to print each procedure's on-disk JTAB footer (raw hex; its fields are already shown decoded in the "Procedure N (...)" line above the code), plus any bytes `trace_reachable` couldn't prove reachable — both a final run before the footer and any gaps mid-procedure — instead of silently skipping those regions. The flags are independent and can be combined. *(`XJP`'s per-case jump table isn't resolved to labels — its addressing convention isn't confirmed; see [`docs/p-code-jumps-and-standard-calls.md`](docs/p-code-jumps-and-standard-calls.md))* |

### Examples

```sh
p-code --code-file tests/HelloWorld.code list

# Disassemble with file offset, segment offset, and raw hex bytes all shown
p-code --code-file tests/HelloWorld.code disassemble --file-offsets --offsets --bytes

# Disassemble with each procedure's on-disk JTAB footer shown after its code
p-code --code-file tests/HelloWorld.code disassemble --file-offsets --footers
```

## Tests

The [`tests`](tests) directory contains sample files (`HelloWorld.pas`, `HelloWorld.code`) useful for exercising `p-code` against a known-good codefile, plus `Features.text`, a broader UCSD/Apple Pascal language-feature demo for more thorough manual exercising of compilation and disassembly.

There are also 3 compressed .dsk files (Apple Pascal image format) for testing `p-filer`:
`empty.dsk` - a disk with no contents
`manyfiles.dsk` - a disk with 75 files on it
`blog.dsk` - a disk with several blog post text files on it

## Status

This project is under active development. Expect missing features and rough edges (for example, `p-code`'s disassembler has known gaps in jump-target/reachability tracking — see the `p-code` section above).

## License

MIT — see [LICENSE](LICENSE).
