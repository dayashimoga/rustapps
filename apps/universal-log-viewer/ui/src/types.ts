export interface LogLineRecord {
  line_number: number;
  byte_start: number;
  byte_end: number;
  text: string;
}

export interface OpenedFileInfo {
  path: string;
  total_lines: number;
  file_size_bytes: number;
}

export interface LogQuery {
  pattern: string;
  is_regex: boolean;
  case_sensitive: boolean;
  limit: number;
  start_line: number;
}

export interface LogSearchMatch {
  line_number: number;
  byte_start: number;
  byte_end: number;
  line_content: string;
  match_start_in_line: number;
  match_end_in_line: number;
}

export interface LogSearchReport {
  matches: LogSearchMatch[];
  total_matched_in_scan: number;
  elapsed_micros: number;
  reached_limit: boolean;
}

export interface BookmarkRecord {
  id: string;
  workspace_id: string;
  file_path: string;
  line_number: number;
  label: string;
}

export interface ProcessResourceUsage {
  resident_set_size_bytes: number;
  cpu_percentage: number;
}
