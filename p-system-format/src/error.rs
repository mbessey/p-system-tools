use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("truncated data: need {needed} bytes, only {available} available")]
    Truncated { needed: usize, available: usize },
    #[error("invalid value {value} for field `{field}`")]
    InvalidValue { field: &'static str, value: u32 },
}
