//! Error types shared by Lab4 tools.

use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// Result type used by Lab4 library code.
pub type Lab4Result<T> = Result<T, Lab4Error>;

/// Structured errors for Lab4 utility operations.
#[derive(Debug, Error)]
pub enum Lab4Error {
    /// A JSONL line could not be parsed.
    #[error("invalid JSONL line {line_number} in {path}: {source}")]
    InvalidJsonLine {
        /// Path of the JSONL file being read.
        path: PathBuf,
        /// One-based line number.
        line_number: usize,
        /// Underlying JSON parser error.
        #[source]
        source: serde_json::Error,
    },

    /// A JSONL line could not be written.
    #[error("failed to write JSONL record: {0}")]
    JsonWrite(#[source] serde_json::Error),

    /// An input or output file could not be accessed.
    #[error("I/O error at {path}: {source}")]
    Io {
        /// Path involved in the failed operation.
        path: PathBuf,
        /// Underlying operating system error.
        #[source]
        source: io::Error,
    },

    /// A prompt record is not valid for this experiment.
    #[error("invalid prompt {id}: {reason}")]
    InvalidPrompt {
        /// Prompt identifier.
        id: String,
        /// Human-readable reason.
        reason: String,
    },

    /// Two prompt records share the same identifier.
    #[error("duplicate prompt id: {0}")]
    DuplicatePromptId(String),

    /// A command failed to spawn or wait.
    #[error("failed to run command {program}: {source}")]
    CommandIo {
        /// Program path or name.
        program: String,
        /// Underlying operating system error.
        #[source]
        source: io::Error,
    },

    /// The system clock moved backwards while building a timestamp.
    #[error("system clock is before Unix epoch")]
    ClockBeforeUnixEpoch,
}
