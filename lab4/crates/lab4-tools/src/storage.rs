//! Storage timing helpers for local and Ceph-backed paths.

use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::{Lab4Error, Lab4Result};

const READ_BUFFER_SIZE: usize = 1024 * 1024;

/// Storage operation kind.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum StorageOperation {
    /// Read a file sequentially.
    Read,
    /// Copy a file from source to target.
    Copy,
}

/// One storage timing record emitted as JSONL.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct StorageRecord {
    /// Stable case id supplied by the caller.
    pub case_id: String,
    /// Operation kind.
    pub operation: StorageOperation,
    /// Source path.
    pub source: String,
    /// Optional target path for copy operations.
    pub target: Option<String>,
    /// Unix timestamp in milliseconds captured before the operation.
    pub started_at_unix_ms: u128,
    /// Wall-clock operation duration in milliseconds.
    pub duration_ms: u128,
    /// Number of bytes read or copied.
    pub bytes: u64,
    /// Approximate throughput in bytes per second.
    pub bytes_per_second: Option<f64>,
}

/// Measures sequential reading of a file.
///
/// # Errors
///
/// Returns [`Lab4Error::ClockBeforeUnixEpoch`] if a timestamp cannot be built,
/// or [`Lab4Error::Io`] if the source file cannot be opened or read.
pub fn measure_read(case_id: String, path: &Path) -> Lab4Result<StorageRecord> {
    let started_at_unix_ms = unix_time_ms()?;
    let started = Instant::now();
    let file = File::open(path).map_err(|source| Lab4Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut reader = BufReader::with_capacity(READ_BUFFER_SIZE, file);
    let mut buffer = vec![0; READ_BUFFER_SIZE];
    let mut bytes = 0_u64;

    loop {
        let bytes_read = reader.read(&mut buffer).map_err(|source| Lab4Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if bytes_read == 0 {
            break;
        }
        bytes += u64::try_from(bytes_read).map_err(|_| Lab4Error::InvalidPrompt {
            id: case_id.clone(),
            reason: "read size does not fit into u64".to_owned(),
        })?;
    }

    let duration_ms = started.elapsed().as_millis();
    Ok(StorageRecord {
        case_id,
        operation: StorageOperation::Read,
        source: path.display().to_string(),
        target: None,
        started_at_unix_ms,
        duration_ms,
        bytes,
        bytes_per_second: throughput(bytes, duration_ms),
    })
}

/// Measures copying a file from source to target.
///
/// # Errors
///
/// Returns [`Lab4Error::ClockBeforeUnixEpoch`] if a timestamp cannot be built,
/// or [`Lab4Error::Io`] if the source cannot be copied to the target.
pub fn measure_copy(case_id: String, source: &Path, target: &Path) -> Lab4Result<StorageRecord> {
    let started_at_unix_ms = unix_time_ms()?;
    let started = Instant::now();
    let bytes = fs::copy(source, target).map_err(|error| Lab4Error::Io {
        path: source.to_path_buf(),
        source: error,
    })?;
    let duration_ms = started.elapsed().as_millis();

    Ok(StorageRecord {
        case_id,
        operation: StorageOperation::Copy,
        source: source.display().to_string(),
        target: Some(target.display().to_string()),
        started_at_unix_ms,
        duration_ms,
        bytes,
        bytes_per_second: throughput(bytes, duration_ms),
    })
}

#[allow(clippy::cast_precision_loss)]
fn throughput(bytes: u64, duration_ms: u128) -> Option<f64> {
    if duration_ms == 0 {
        return None;
    }
    let seconds = duration_ms as f64 / 1000.0;
    Some(bytes as f64 / seconds)
}

fn unix_time_ms() -> Lab4Result<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|_| Lab4Error::ClockBeforeUnixEpoch)
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn test_measure_read__counts_file_bytes() -> Result<(), Box<dyn std::error::Error>> {
        let path = unique_temp_path("lab4-storage-read.txt")?;
        {
            let mut file = File::create(&path)?;
            file.write_all(b"abc123")?;
        }

        let record = measure_read("read-test".to_owned(), &path)?;
        fs::remove_file(&path)?;

        assert_eq!(record.bytes, 6);
        assert_eq!(record.operation, StorageOperation::Read);
        Ok(())
    }

    #[test]
    fn test_measure_copy__copies_file_bytes() -> Result<(), Box<dyn std::error::Error>> {
        let source = unique_temp_path("lab4-storage-copy-source.txt")?;
        let target = unique_temp_path("lab4-storage-copy-target.txt")?;
        {
            let mut file = File::create(&source)?;
            file.write_all(b"abcdef")?;
        }

        let record = measure_copy("copy-test".to_owned(), &source, &target)?;
        fs::remove_file(&source)?;
        fs::remove_file(&target)?;

        assert_eq!(record.bytes, 6);
        assert_eq!(record.operation, StorageOperation::Copy);
        Ok(())
    }

    fn unique_temp_path(name: &str) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        Ok(std::env::temp_dir().join(format!("{timestamp}-{name}")))
    }
}
