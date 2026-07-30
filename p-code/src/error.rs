//! Crate-wide error type for p-code: every fallible operation (reading a
//! codefile, parsing its segment dictionary, decoding its copyright string)
//! reports one of these variants.

use p_system_format::error::FormatError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Format(#[from] FormatError),

    #[error("codefile's copyright string is not valid UTF-8: {0}")]
    InvalidCopyrightString(#[from] std::string::FromUtf8Error),
}
