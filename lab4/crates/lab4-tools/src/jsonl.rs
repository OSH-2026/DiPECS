//! JSON Lines helpers for experiment data.

use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::{Lab4Error, Lab4Result};

/// Reads a JSONL file into a vector of typed records.
///
/// Empty lines are ignored so that hand-edited prompt files remain easy to
/// maintain.
///
/// # Errors
///
/// Returns [`Lab4Error::Io`] if the file cannot be read, or
/// [`Lab4Error::InvalidJsonLine`] if any non-empty line is not valid JSON for
/// the requested record type.
pub fn read_jsonl<T>(path: &Path) -> Lab4Result<Vec<T>>
where
    T: DeserializeOwned,
{
    let file = File::open(path).map_err(|source| Lab4Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();
    for (index, line) in reader.lines().enumerate() {
        let line = line.map_err(|source| Lab4Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let record = serde_json::from_str(&line).map_err(|source| Lab4Error::InvalidJsonLine {
            path: path.to_path_buf(),
            line_number: index + 1,
            source,
        })?;
        records.push(record);
    }
    Ok(records)
}

/// Writes a single typed record as one JSONL line.
///
/// # Errors
///
/// Returns [`Lab4Error::JsonWrite`] if the record cannot be serialized, or
/// [`Lab4Error::Io`] if the writer rejects the trailing newline.
pub fn write_jsonl_record<T>(writer: &mut impl Write, record: &T) -> Lab4Result<()>
where
    T: Serialize,
{
    serde_json::to_writer(&mut *writer, record).map_err(Lab4Error::JsonWrite)?;
    writer.write_all(b"\n").map_err(|source| Lab4Error::Io {
        path: Path::new("<writer>").to_path_buf(),
        source,
    })?;
    Ok(())
}
