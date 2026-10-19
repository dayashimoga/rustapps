#![forbid(unsafe_code)]
//! Memory-Mapped Log Engine
//!
//! Provides zero-copy memory-mapped file access for reading multi-gigabyte log
//! files with minimal heap memory usage, staying strictly within the < 300MB RAM budget.

use shared_storage::MmapReader;
use std::fs::File;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Error definitions for mmap log engine
#[derive(Debug, Error)]
pub enum MmapEngineError {
    #[error("I/O error opening file '{path}': {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Storage error mapping file: {0}")]
    Storage(#[from] shared_storage::StorageError),
    #[error("Byte offset range [{start}, {end}] is out of bounds (file size: {file_size})")]
    OutOfBounds {
        start: u64,
        end: u64,
        file_size: u64,
    },
    #[error("File has been truncated (previous size: {previous}, current size: {current})")]
    FileTruncated { previous: u64, current: u64 },
}

/// Zero-copy memory-mapped log file handle
pub struct MmapLogEngine {
    path: PathBuf,
    reader: MmapReader,
    file_size: u64,
}

impl MmapLogEngine {
    /// Open a log file and map it into memory.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, MmapEngineError> {
        let path_buf = path.as_ref().to_path_buf();
        let file = File::open(&path_buf).map_err(|e| MmapEngineError::Io {
            path: path_buf.clone(),
            source: e,
        })?;

        let file_size = file
            .metadata()
            .map_err(|e| MmapEngineError::Io {
                path: path_buf.clone(),
                source: e,
            })?
            .len();

        let reader = MmapReader::open(&path_buf)?;

        Ok(Self {
            path: path_buf,
            reader,
            file_size,
        })
    }

    /// Reload the memory map if the file size on disk has changed.
    pub fn reload_if_resized(&mut self) -> Result<bool, MmapEngineError> {
        let metadata = std::fs::metadata(&self.path).map_err(|e| MmapEngineError::Io {
            path: self.path.clone(),
            source: e,
        })?;

        let current_size = metadata.len();
        if current_size == self.file_size {
            return Ok(false);
        }

        if current_size < self.file_size {
            return Err(MmapEngineError::FileTruncated {
                previous: self.file_size,
                current: current_size,
            });
        }

        // File has grown; re-open the mmap handle
        self.reader = MmapReader::open(&self.path)?;
        self.file_size = current_size;
        Ok(true)
    }

    /// Retrieve a slice of bytes between `[start, end]`.
    pub fn read_bytes_range(&self, start: u64, end: u64) -> Result<&[u8], MmapEngineError> {
        if start > end || end > self.file_size {
            return Err(MmapEngineError::OutOfBounds {
                start,
                end,
                file_size: self.file_size,
            });
        }

        let slice = self.reader.slice(start as usize, end as usize).ok_or(
            MmapEngineError::OutOfBounds {
                start,
                end,
                file_size: self.file_size,
            },
        )?;

        Ok(slice)
    }

    /// Retrieve text slice with lossy UTF-8 conversion, ensuring safety against non-UTF8 bytes.
    pub fn read_text_range(&self, start: u64, end: u64) -> Result<String, MmapEngineError> {
        let bytes = self.read_bytes_range(start, end)?;
        Ok(String::from_utf8_lossy(bytes).to_string())
    }

    /// Get total mapped size in bytes.
    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    /// Path to mapped file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Access raw underlying bytes.
    pub fn as_bytes(&self) -> &[u8] {
        self.reader.as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mmap_engine_read_range() {
        let temp = tempfile::tempdir().unwrap();
        let log_file = temp.path().join("app.log");
        let content = b"Line 1: System Boot\nLine 2: Service Init\nLine 3: Ready\n";
        std::fs::write(&log_file, content).unwrap();

        let engine = MmapLogEngine::open(&log_file).expect("Must open log file");
        assert_eq!(engine.file_size(), content.len() as u64);

        let slice = engine.read_text_range(0, 19).unwrap();
        assert_eq!(slice, "Line 1: System Boot");

        // Boundary checks
        let out_of_bounds = engine.read_bytes_range(0, 1000);
        assert!(out_of_bounds.is_err());
    }

    #[test]
    fn test_mmap_engine_reload_on_growth() {
        let temp = tempfile::tempdir().unwrap();
        let log_file = temp.path().join("growing.log");
        std::fs::write(&log_file, b"Initial Line\n").unwrap();

        let mut engine = MmapLogEngine::open(&log_file).unwrap();
        assert_eq!(engine.file_size(), 13);

        // Append line
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&log_file)
            .unwrap();
        use std::io::Write;
        f.write_all(b"Appended Second Line\n").unwrap();
        f.sync_all().unwrap();

        let changed = engine.reload_if_resized().unwrap();
        assert!(changed);
        assert_eq!(engine.file_size(), 34);

        let second_line = engine.read_text_range(13, 33).unwrap();
        assert_eq!(second_line, "Appended Second Line");
    }
}
