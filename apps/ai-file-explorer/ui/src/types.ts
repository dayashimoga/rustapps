export interface FileItem {
  path: string;
  file_name: string;
  extension: string | null;
  size_bytes: number;
  is_dir: boolean;
  modified_timestamp_secs: number;
  is_readonly: boolean;
}

export interface DirectoryListing {
  current_path: string;
  parent_path: string | null;
  items: FileItem[];
  total_count: number;
  total_size_bytes: number;
}

export interface FolderStats {
  root_path: string;
  total_files: number;
  total_dirs: number;
  total_size_bytes: number;
  extension_counts: Record<string, number>;
  extension_sizes: Record<string, number>;
  largest_files: FileItem[];
}

export interface DuplicateGroup {
  file_size: number;
  blake3_hash: string;
  file_paths: string[];
  wasted_bytes: number;
}

export interface DeduplicationSummary {
  total_scanned_files: number;
  total_scanned_bytes: number;
  duplicate_groups: DuplicateGroup[];
  total_duplicate_files: number;
  potential_reclaimed_bytes: number;
  elapsed_millis: number;
}

export interface TreemapRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface TreemapNode {
  name: string;
  path: string;
  size_bytes: number;
  is_dir: boolean;
  bounds: TreemapRect;
  children: TreemapNode[];
}

export type BatchRenameRule =
  | { type: 'RegexReplace'; config: { pattern: string; replacement: string } }
  | { type: 'PrefixSuffix'; config: { prefix: string; suffix: string } }
  | { type: 'Numbering'; config: { prefix: string; start_number: number; pad_digits: number } }
  | { type: 'ToLowerCase' }
  | { type: 'ToUpperCase' };

export interface RenamePreview {
  original_path: string;
  proposed_path: string;
  has_conflict: boolean;
  conflict_reason: string | null;
}

export interface BatchRenameResult {
  total_requested: number;
  successful_renames: number;
  rolled_back: boolean;
  error_message: string | null;
}

export type ActiveTab = 'browser' | 'duplicates' | 'treemap' | 'batch_rename';
