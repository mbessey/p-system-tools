# p-system-tools

Tools for interacting with the UCSD p-System virtual machine, and specifically Apple Pascal.

This is a Rust workspace containing two command-line utilities:

- **[p-filer](p-filer)** — inspects and manipulates Apple Pascal disk images (`.dsk` files).
- **[p-code](p-code)** — inspects UCSD p-System code files (linked object files, produced by the Pascal compiler/linker).

Both tools are early-stage / work-in-progress: several subcommands are stubs that print a message but don't yet perform the action (see [Status](#status) below).

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
| `remove <name>` | Delete a file from the volume. *(stub — not yet implemented)* |
| `transfer <name> [--to-image] [--text] [-p, --preserve-date]` | Copy a file between the disk image and the host filesystem. By default copies **from** the image to the current directory; pass `--to-image` to copy a host file **to** the image instead, allocating it into the first large-enough gap in the volume's free space and saving the image back to disk (a `.bak` backup of the previous state is written alongside it). Pass `--text` to convert p-System text file encoding (CR line endings, run-length-encoded indentation) to/from plain LF text. Pass `--preserve-date`/`-p` to sync the modification time between the extracted host file and the file's date on the volume, in whichever direction the copy is going. |
| `change <from> <to>` | Rename a file on the volume. *(stub — not yet implemented)* |
| `krunch` | Consolidate free space on the volume. *(stub — not yet implemented)* |
| `zero` | Clear the volume directory. *(stub — not yet implemented)* |
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

No sample `.dsk` image is included in this repository; supply your own Apple Pascal disk image.

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
| `disassemble` | Disassemble the p-code in the file. |

### Example

```sh
p-code --code-file tests/HelloWorld.code list
```

## Tests

The [`tests`](tests) directory contains sample files (`HelloWorld.pas`, `HelloWorld.code`) useful for exercising `p-code` against a known-good codefile.

There are also 3 compressed .dsk files (Apple Pascal image format) for testing `p-filer`:
`empty.dsk` - a disk with no contents
`manyfiles.dsk` - a disk with 75 files on it
`blog.dsk` - a disk with several blog post text files on it

## Status

This project is under active development. Expect missing features and rough edges (several `p-filer` subcommands are still stubs — see the command tables above).

## License

MIT — see [LICENSE](LICENSE).
