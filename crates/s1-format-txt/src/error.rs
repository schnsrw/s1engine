//! Error types for the plain text format crate.

/// Error type for TXT format operations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum TxtError {
    /// The input bytes could not be decoded as valid text.
    #[error("Failed to decode as {encoding}: {message}")]
    DecodingError { encoding: String, message: String },
}
