#![forbid(unsafe_code)]
//! Squarified Treemap Layout Engine for Hierarchical Storage Visualization (ADR-0025)

use crate::models::{TreemapNode, TreemapRect};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TreemapError {
    #[error("I/O error during treemap generation: {0}")]
    Io(#[from] std::io::Error),
    #[error("Target directory not found: {0}")]
    NotFound(PathBuf),
}

/// Squarified treemap generator
pub struct TreemapGenerator;

impl TreemapGenerator {
    /// Build a full hierarchical treemap tree with squarified rectangle bounds
    pub fn generate_treemap<P: AsRef<Path>>(
        root_path: P,
        width: f64,
        height: f64,
        max_depth: usize,
    ) -> Result<TreemapNode, TreemapError> {
        let root = root_path.as_ref();
        if !root.exists() {
            return Err(TreemapError::NotFound(root.to_path_buf()));
        }

        // Build size tree
        let mut root_node = Self::build_tree(root, 0, max_depth)?;

        // Layout squarified rectangles
        let initial_bounds = TreemapRect {
            x: 0.0,
            y: 0.0,
            width,
            height,
        };
        root_node.bounds = initial_bounds.clone();

        Self::layout_children(&mut root_node, &initial_bounds, 0, max_depth);

        Ok(root_node)
    }

    /// Recursively build folder/file nodes and calculate size sums
    fn build_tree(
        path: &Path,
        depth: usize,
        max_depth: usize,
    ) -> Result<TreemapNode, TreemapError> {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        let metadata = match fs::metadata(path) {
            Ok(m) => m,
            Err(e) => return Err(TreemapError::Io(e)),
        };

        let is_dir = metadata.is_dir();
        let mut size_bytes = if is_dir { 0 } else { metadata.len() };
        let mut children = Vec::new();

        if is_dir && depth < max_depth {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let child_path = entry.path();
                    if let Ok(child_node) = Self::build_tree(&child_path, depth + 1, max_depth) {
                        size_bytes = size_bytes.saturating_add(child_node.size_bytes);
                        children.push(child_node);
                    }
                }
            }
        }

        // Sort children by size descending for optimal squarified layout
        children.sort_by_key(|b| std::cmp::Reverse(b.size_bytes));

        Ok(TreemapNode {
            name,
            path: path.to_path_buf(),
            size_bytes,
            is_dir,
            bounds: TreemapRect {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
            children,
        })
    }

    /// Compute squarified bounds for child nodes
    fn layout_children(
        node: &mut TreemapNode,
        bounds: &TreemapRect,
        depth: usize,
        max_depth: usize,
    ) {
        if node.children.is_empty() || bounds.width <= 2.0 || bounds.height <= 2.0 {
            return;
        }

        let total_size: f64 = node.children.iter().map(|c| c.size_bytes as f64).sum();
        if total_size <= 0.0 {
            return;
        }

        let total_area = bounds.width * bounds.height;
        let mut items: Vec<(usize, f64)> = node
            .children
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let area = (c.size_bytes as f64 / total_size) * total_area;
                (i, area)
            })
            .collect();

        // Sort by area descending
        items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut rects = vec![
            TreemapRect {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            };
            node.children.len()
        ];

        Self::squarify(&items, bounds, &mut rects);

        for (idx, rect) in rects.into_iter().enumerate() {
            node.children[idx].bounds = rect.clone();
            if node.children[idx].is_dir && depth + 1 < max_depth {
                // Apply a small 2px margin for subdirectories so boundaries are visible
                let inner_bounds = TreemapRect {
                    x: rect.x + 1.0,
                    y: rect.y + 1.0,
                    width: (rect.width - 2.0).max(0.0),
                    height: (rect.height - 2.0).max(0.0),
                };
                let child = &mut node.children[idx];
                Self::layout_children(child, &inner_bounds, depth + 1, max_depth);
            }
        }
    }

    /// Standard Squarified Treemap partitioner
    fn squarify(items: &[(usize, f64)], container: &TreemapRect, out_rects: &mut [TreemapRect]) {
        if items.is_empty() {
            return;
        }

        let mut remaining_items = items;
        let mut current_container = container.clone();

        while !remaining_items.is_empty() {
            if current_container.width <= 0.0 || current_container.height <= 0.0 {
                break;
            }

            let is_horizontal = current_container.width >= current_container.height;
            let side_length = if is_horizontal {
                current_container.height
            } else {
                current_container.width
            };

            let mut row = vec![remaining_items[0]];
            let mut row_area = remaining_items[0].1;
            let mut best_aspect = Self::worst_aspect_ratio(&row, row_area, side_length);

            let mut i = 1;
            while i < remaining_items.len() {
                let next_item = remaining_items[i];
                let mut candidate_row = row.clone();
                candidate_row.push(next_item);
                let candidate_area = row_area + next_item.1;
                let candidate_aspect =
                    Self::worst_aspect_ratio(&candidate_row, candidate_area, side_length);

                if candidate_aspect <= best_aspect {
                    row = candidate_row;
                    row_area = candidate_area;
                    best_aspect = candidate_aspect;
                    i += 1;
                } else {
                    break;
                }
            }

            // Layout the row
            let row_breadth = if side_length > 0.0 {
                row_area / side_length
            } else {
                0.0
            };

            let mut offset = 0.0;
            for (idx, area) in &row {
                let item_length = if row_breadth > 0.0 {
                    area / row_breadth
                } else {
                    0.0
                };

                let rect = if is_horizontal {
                    TreemapRect {
                        x: current_container.x,
                        y: current_container.y + offset,
                        width: row_breadth,
                        height: item_length,
                    }
                } else {
                    TreemapRect {
                        x: current_container.x + offset,
                        y: current_container.y,
                        width: item_length,
                        height: row_breadth,
                    }
                };

                out_rects[*idx] = rect;
                offset += item_length;
            }

            // Advance container
            if is_horizontal {
                current_container.x += row_breadth;
                current_container.width = (current_container.width - row_breadth).max(0.0);
            } else {
                current_container.y += row_breadth;
                current_container.height = (current_container.height - row_breadth).max(0.0);
            }

            remaining_items = &remaining_items[row.len()..];
        }
    }

    fn worst_aspect_ratio(row: &[(usize, f64)], row_area: f64, side_length: f64) -> f64 {
        if row_area <= 0.0 || side_length <= 0.0 {
            return f64::INFINITY;
        }

        let s2 = side_length * side_length;
        let a2 = row_area * row_area;

        let mut max_aspect: f64 = 0.0;
        for (_, area) in row {
            if *area <= 0.0 {
                continue;
            }
            let aspect1 = (s2 * area) / a2;
            let aspect2 = a2 / (s2 * area);
            let worst = aspect1.max(aspect2);
            if worst > max_aspect {
                max_aspect = worst;
            }
        }

        if max_aspect == 0.0 {
            f64::INFINITY
        } else {
            max_aspect
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_treemap_generation_bounds() {
        let dir = tempdir().unwrap();
        let f1 = dir.path().join("f1.dat");
        let f2 = dir.path().join("f2.dat");
        let mut file1 = File::create(&f1).unwrap();
        let mut file2 = File::create(&f2).unwrap();
        file1.write_all(&vec![1u8; 1000]).unwrap();
        file2.write_all(&vec![2u8; 3000]).unwrap();

        let treemap = TreemapGenerator::generate_treemap(dir.path(), 800.0, 600.0, 3).unwrap();

        assert_eq!(treemap.bounds.width, 800.0);
        assert_eq!(treemap.bounds.height, 600.0);
        assert_eq!(treemap.children.len(), 2);

        for child in &treemap.children {
            assert!(child.bounds.width > 0.0);
            assert!(child.bounds.height > 0.0);
            assert!(child.bounds.x >= 0.0);
            assert!(child.bounds.y >= 0.0);
        }
    }
}
