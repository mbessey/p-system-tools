//! Crate-wide error type for p-filer: every fallible operation on an Apple
//! Pascal disk image (I/O, directory/volume bookkeeping, host-file transfer)
//! reports one of these variants. Mirrors p-system-format's `FormatError` --
//! a flat thiserror enum rather than a boxed dynamic error, so every failure
//! mode this crate can produce is enumerable in one place.

use p_system_format::error::FormatError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Format(#[from] FormatError),

    #[error(
        "disk image size ({len} bytes) is not a whole number of tracks ({track_size} bytes each)"
    )]
    InvalidImageSize { len: usize, track_size: usize },

    #[error("directory is full")]
    DirectoryFull,

    #[error("{name} already exists on volume")]
    NameAlreadyExists { name: String },

    #[error("{name} not found on volume")]
    EntryNotFound { name: String },

    #[error("{name} was not found on {image_name}")]
    FileNotFoundOnImage { name: String, image_name: String },

    #[error("from ({from}) must be less than to ({to})")]
    DumpRangeInverted { from: usize, to: usize },

    #[error("{which} ({value}) must be less than {num_blocks} blocks")]
    DumpBlockOutOfRange {
        which: &'static str,
        value: usize,
        num_blocks: usize,
    },

    #[error("{name} has no valid file name")]
    NoFileName { name: String },

    // Same field shape as RenameTargetTooLong/VolumeNameTooLong below --
    // kept as separate variants (rather than one shared "name too long"
    // variant) so each existing, test-checked message string is preserved
    // exactly rather than drifting while anyhow is being removed.
    #[error(
        "\"{name}\" is {len} characters, but p-System volume filenames are limited to {limit} characters -- rename the file and try again"
    )]
    FileNameTooLong {
        name: String,
        len: usize,
        limit: usize,
    },

    #[error(
        "\"{name}\" is {len} characters, but p-System volume filenames are limited to {limit} characters"
    )]
    RenameTargetTooLong {
        name: String,
        len: usize,
        limit: usize,
    },

    #[error(
        "\"{name}\" is {len} characters, but p-System volume names are limited to {limit} characters"
    )]
    VolumeNameTooLong {
        name: String,
        len: usize,
        limit: usize,
    },

    #[error(
        "input contains byte {byte:#04x}, which is reserved by the p-System text encoding and can't be represented in a text-mode transfer (retry without --text)"
    )]
    ReservedTextByte { byte: u8 },

    #[error("{name} needs {needed} blocks, but the volume only has {available} blocks total")]
    NotEnoughSpaceTotal {
        name: String,
        needed: usize,
        available: usize,
    },

    #[error("not enough contiguous free space for {name}")]
    NoContiguousSpace { name: String },
}
