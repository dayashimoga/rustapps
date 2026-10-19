#![forbid(unsafe_code)]
//! Transactional Atomic Batch Renamer with Rollback Journal (ADR-0024)

use crate::models::{BatchRenameResult, BatchRenameRule, RenamePreview};
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RenameError {
    #[error("Regex compile error: {0}")]
    Regex(#[from] regex::Error),
    #[error("I/O error during rename: {0}")]
    Io(#[from] std::io::Error),
    #[error("Conflict detected: {0}")]
    Conflict(String),
}

/// Transactional batch renamer
pub struct BatchRenamer;

impl BatchRenamer {
    /// Dry-run simulation generating proposed paths and validating against filesystem collisions
    pub fn preview_renames(
        file_paths: &[PathBuf],
        rule: &BatchRenameRule,
    ) -> Result<Vec<RenamePreview>, RenameError> {
        let mut previews = Vec::with_capacity(file_paths.len());
        let mut target_set: HashSet<PathBuf> = HashSet::new();

        let compiled_regex = match rule {
            BatchRenameRule::RegexReplace { pattern, .. } => Some(Regex::new(pattern)?),
            _ => None,
        };

        for (idx, original_path) in file_paths.iter().enumerate() {
            let parent = original_path.parent().unwrap_or_else(|| Path::new(""));
            let file_name = original_path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default();

            let stem = original_path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            let ext = original_path
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_default();

            let new_filename = match rule {
                BatchRenameRule::RegexReplace { replacement, .. } => {
                    if let Some(ref re) = compiled_regex {
                        re.replace_all(&file_name, replacement.as_str()).to_string()
                    } else {
                        file_name.clone()
                    }
                }
                BatchRenameRule::PrefixSuffix { prefix, suffix } => {
                    format!("{}{}{}{}", prefix, stem, suffix, ext)
                }
                BatchRenameRule::Numbering {
                    prefix,
                    start_number,
                    pad_digits,
                } => {
                    let num = start_number + (idx as u32);
                    format!("{}{:0width$}{}", prefix, num, ext, width = pad_digits)
                }
                BatchRenameRule::ToLowerCase => file_name.to_lowercase(),
                BatchRenameRule::ToUpperCase => file_name.to_uppercase(),
            };

            let proposed_path = parent.join(new_filename);

            // Collision checks
            let mut has_conflict = false;
            let mut conflict_reason = None;

            if !original_path.exists() {
                has_conflict = true;
                conflict_reason = Some("Source file does not exist".to_string());
            } else if proposed_path != *original_path {
                if target_set.contains(&proposed_path) {
                    has_conflict = true;
                    conflict_reason = Some("Duplicate destination in batch".to_string());
                } else if proposed_path.exists() {
                    has_conflict = true;
                    conflict_reason = Some("Destination file already exists on disk".to_string());
                }
            }

            target_set.insert(proposed_path.clone());

            previews.push(RenamePreview {
                original_path: original_path.clone(),
                proposed_path,
                has_conflict,
                conflict_reason,
            });
        }

        Ok(previews)
    }

    /// Execute the batch rename transactionally with automatic reverse-journal rollback upon failure
    pub fn execute_batch_rename(
        previews: &[RenamePreview],
    ) -> Result<BatchRenameResult, RenameError> {
        // Step 1: Pre-execution guard - ensure no conflicts
        for preview in previews {
            if preview.has_conflict {
                return Ok(BatchRenameResult {
                    total_requested: previews.len(),
                    successful_renames: 0,
                    rolled_back: false,
                    error_message: Some(format!(
                        "Cannot execute with unresolved conflicts: {:?}",
                        preview.conflict_reason
                    )),
                });
            }
        }

        let mut journal: Vec<(PathBuf, PathBuf)> = Vec::new(); // (current_renamed_path, original_path)

        for preview in previews {
            if preview.original_path == preview.proposed_path {
                continue; // Identity rename, no-op
            }

            match fs::rename(&preview.original_path, &preview.proposed_path) {
                Ok(_) => {
                    journal.push((preview.proposed_path.clone(), preview.original_path.clone()));
                }
                Err(err) => {
                    // Step 2: Rollback in reverse order
                    tracing::error!(
                        "Rename failed at {:?}, triggering rollback: {}",
                        preview.original_path,
                        err
                    );
                    let mut rollback_errors = Vec::new();

                    while let Some((renamed_path, original_path)) = journal.pop() {
                        if let Err(rb_err) = fs::rename(&renamed_path, &original_path) {
                            rollback_errors.push(format!(
                                "{:?} -> {:?}: {}",
                                renamed_path, original_path, rb_err
                            ));
                        }
                    }

                    let err_msg = if rollback_errors.is_empty() {
                        format!("Rename aborted and cleanly rolled back. Cause: {}", err)
                    } else {
                        format!(
                            "Rename failed: {}. Rollback partial failure: {}",
                            err,
                            rollback_errors.join("; ")
                        )
                    };

                    return Ok(BatchRenameResult {
                        total_requested: previews.len(),
                        successful_renames: 0,
                        rolled_back: true,
                        error_message: Some(err_msg),
                    });
                }
            }
        }

        Ok(BatchRenameResult {
            total_requested: previews.len(),
            successful_renames: journal.len(),
            rolled_back: false,
            error_message: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_preview_and_execute_batch_rename_numbering() {
        let dir = tempdir().unwrap();
        let f1 = dir.path().join("img_a.jpg");
        let f2 = dir.path().join("img_b.jpg");
        File::create(&f1).unwrap();
        File::create(&f2).unwrap();

        let files = vec![f1.clone(), f2.clone()];
        let rule = BatchRenameRule::Numbering {
            prefix: "photo_".to_string(),
            start_number: 1,
            pad_digits: 3,
        };

        let previews = BatchRenamer::preview_renames(&files, &rule).unwrap();
        assert_eq!(previews.len(), 2);
        assert!(!previews[0].has_conflict);
        assert!(!previews[1].has_conflict);
        assert_eq!(
            previews[0].proposed_path.file_name().unwrap(),
            "photo_001.jpg"
        );
        assert_eq!(
            previews[1].proposed_path.file_name().unwrap(),
            "photo_002.jpg"
        );

        let result = BatchRenamer::execute_batch_rename(&previews).unwrap();
        assert_eq!(result.successful_renames, 2);
        assert!(!result.rolled_back);
        assert!(result.error_message.is_none());

        assert!(!f1.exists());
        assert!(!f2.exists());
        assert!(dir.path().join("photo_001.jpg").exists());
        assert!(dir.path().join("photo_002.jpg").exists());
    }

    #[test]
    fn test_collision_detection_flags_conflict() {
        let dir = tempdir().unwrap();
        let f1 = dir.path().join("first.txt");
        let f2 = dir.path().join("target.txt");
        File::create(&f1).unwrap();
        File::create(&f2).unwrap();

        let files = vec![f1.clone()];
        let rule = BatchRenameRule::RegexReplace {
            pattern: "first".to_string(),
            replacement: "target".to_string(),
        };

        let previews = BatchRenamer::preview_renames(&files, &rule).unwrap();
        assert_eq!(previews.len(), 1);
        // target.txt already exists on disk
        assert!(previews[0].has_conflict);
        assert!(previews[0].conflict_reason.is_some());
    }

    #[test]
    fn test_regex_replace_rule() {
        let dir = tempdir().unwrap();
        let f1 = dir.path().join("draft_report_v1.md");
        File::create(&f1).unwrap();

        let files = vec![f1.clone()];
        let rule = BatchRenameRule::RegexReplace {
            pattern: "v1".to_string(),
            replacement: "final".to_string(),
        };

        let previews = BatchRenamer::preview_renames(&files, &rule).unwrap();
        assert_eq!(
            previews[0].proposed_path.file_name().unwrap(),
            "draft_report_final.md"
        );
    }
}
