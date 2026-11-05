#![forbid(unsafe_code)]
//! AI File Explorer Domain Models & Data Structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Metadata for a discovered or indexed file item
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileItem {
    pub path: PathBuf,
    pub file_name: String,
    pub extension: Option<String>,
    pub size_bytes: u64,
    pub is_dir: bool,
    pub modified_timestamp_secs: u64,
    pub is_readonly: bool,
}

impl FileItem {
    /// Formats the byte size into a human-readable string (B, KB, MB, GB, TB)
    pub fn human_readable_size(&self) -> String {
        format_bytes(self.size_bytes)
    }
}

/// Helper function to format byte sizes into human-readable strings
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Directory contents listing for file browser navigation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirectoryListing {
    pub current_path: PathBuf,
    pub parent_path: Option<PathBuf>,
    pub items: Vec<FileItem>,
    pub total_count: usize,
    pub total_size_bytes: u64,
}

/// Aggregate folder statistics for storage analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FolderStats {
    pub root_path: PathBuf,
    pub total_files: usize,
    pub total_dirs: usize,
    pub total_size_bytes: u64,
    pub extension_counts: HashMap<String, usize>,
    pub extension_sizes: HashMap<String, u64>,
    pub largest_files: Vec<FileItem>,
}

/// A cluster of identical files identified by the duplicate detection engine
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DuplicateGroup {
    pub file_size: u64,
    pub blake3_hash: String,
    pub file_paths: Vec<PathBuf>,
    pub wasted_bytes: u64,
}

/// Summary report of duplicate file scanning
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeduplicationSummary {
    pub total_scanned_files: usize,
    pub total_scanned_bytes: u64,
    pub duplicate_groups: Vec<DuplicateGroup>,
    pub total_duplicate_files: usize,
    pub potential_reclaimed_bytes: u64,
    pub elapsed_millis: u64,
}

/// Result of deleting redundant or duplicate files
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeleteFilesResult {
    pub deleted_count: usize,
    pub reclaimed_bytes: u64,
    pub failed_paths: Vec<String>,
}

/// Result of copying files between locations (e.g. Dual-Pane Commander Mode)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CopyFilesResult {
    pub copied_count: usize,
    pub total_bytes: u64,
    pub failed_paths: Vec<String>,
}

/// Result of moving files between locations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MoveFilesResult {
    pub moved_count: usize,
    pub failed_paths: Vec<String>,
}

/// Dynamic Smart Virtual Folder query definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SmartFolder {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub description: String,
    pub query: String,
    pub extensions: Vec<String>,
    pub min_size_bytes: Option<u64>,
    pub max_size_bytes: Option<u64>,
    pub is_builtin: bool,
}

/// Bounding rectangle for treemap rendering
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TreemapRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Hierarchical node in a storage treemap
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TreemapNode {
    pub name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub is_dir: bool,
    pub bounds: TreemapRect,
    pub children: Vec<TreemapNode>,
}

/// Transformation rules for transactional batch renaming
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "config")]
pub enum BatchRenameRule {
    /// Regex pattern find and replace
    RegexReplace {
        pattern: String,
        replacement: String,
    },
    /// Prepend and/or append text to the filename
    PrefixSuffix { prefix: String, suffix: String },
    /// Sequential numbering with custom padding (e.g., photo_001.jpg)
    Numbering {
        prefix: String,
        start_number: u32,
        pad_digits: usize,
    },
    /// Change filename casing to lowercase
    ToLowerCase,
    /// Change filename casing to uppercase
    ToUpperCase,
}

/// Preview of a proposed single file rename
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenamePreview {
    pub original_path: PathBuf,
    pub proposed_path: PathBuf,
    pub has_conflict: bool,
    pub conflict_reason: Option<String>,
}

/// Result of an executed transactional batch rename operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BatchRenameResult {
    pub total_requested: usize,
    pub successful_renames: usize,
    pub rolled_back: bool,
    pub error_message: Option<String>,
}

/// File system change events emitted by the watcher
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WatchEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
    Renamed { from: PathBuf, to: PathBuf },
}

/// Metadata extracted from a structured document (PDF, DOCX, Markdown, Code)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub page_count: Option<usize>,
    pub word_count: usize,
    pub format: String,
    pub creation_date: Option<String>,
}

/// Extracted document containing full-text content and document metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtractedDocument {
    pub path: PathBuf,
    pub file_name: String,
    pub extension: Option<String>,
    pub metadata: DocumentMetadata,
    pub content_text: String,
    pub language: Option<String>,
    pub extracted_at_secs: u64,
}

/// Recognized word with bounding box and confidence score from OCR engine
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcrWord {
    pub text: String,
    pub confidence: f32,
    pub bbox: [u32; 4], // x, y, width, height
}

/// Output result from offline OCR analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcrResult {
    pub text: String,
    pub words: Vec<OcrWord>,
    pub confidence: f32,
    pub line_count: usize,
    pub processing_time_ms: f64,
}

/// Visual fingerprint and OCR data for a screenshot or image
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScreenshotFingerprint {
    pub path: PathBuf,
    pub dhash: u64,
    pub ocr_text: String,
    pub dimensions: (u32, u32),
}

/// Match result for screenshot similarity search
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScreenshotSearchResult {
    pub path: PathBuf,
    pub dhash: u64,
    pub hamming_distance: u32,
    pub similarity_pct: f32,
    pub ocr_snippet: Option<String>,
}

/// 8-bit quantized dense semantic embedding vector
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorEmbedding {
    pub doc_path: PathBuf,
    pub vector: Vec<i8>,
    pub dimension: usize,
    pub magnitude: f32,
}

/// Mode of search execution
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SearchMode {
    Keyword,
    Semantic,
    Combined,
}

/// Query parameters for document content and semantic search
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContentSearchQuery {
    pub query: String,
    pub mode: SearchMode,
    pub limit: usize,
    pub extension_filter: Option<String>,
}

/// Individual hit from a content or semantic search
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SemanticSearchResult {
    pub path: PathBuf,
    pub file_name: String,
    pub score: f32,
    pub snippet: String,
    pub format: String,
}

/// Aggregated content search result bundle
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContentSearchResult {
    pub items: Vec<SemanticSearchResult>,
    pub total_matches: usize,
    pub execution_time_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 1024 * 2), "2.00 TB");
    }

    #[test]
    fn test_file_item_human_readable_size() {
        let item = FileItem {
            path: PathBuf::from("/test/file.rs"),
            file_name: "file.rs".to_string(),
            extension: Some("rs".to_string()),
            size_bytes: 2048,
            is_dir: false,
            modified_timestamp_secs: 1700000000,
            is_readonly: false,
        };
        assert_eq!(item.human_readable_size(), "2.00 KB");
    }
}
