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

export interface TimelineBucketUi {
  bucket_index: number;
  start_epoch_ms: number;
  end_epoch_ms: number;
  total_count: number;
  trace_count: number;
  debug_count: number;
  info_count: number;
  warn_count: number;
  error_count: number;
  fatal_count: number;
  volume_per_second: number;
}

export interface TimelineHistogramUi {
  min_epoch_ms: number;
  max_epoch_ms: number;
  total_records: number;
  bucket_width_ms: number;
  buckets: TimelineBucketUi[];
}

export interface TemplateClusterUi {
  cluster_id: number;
  template: string;
  token_count: number;
  frequency: number;
  sample_lines: number[];
}

export interface AnomalySpikeUi {
  spike_id: number;
  start_epoch_ms: number;
  end_epoch_ms: number;
  z_score: number;
  observed_value: number;
  expected_value: number;
  metric_name: string;
  description: string;
  severity: string;
}

export interface ErrorClusterUi {
  cluster_id: number;
  template: string;
  error_count: number;
  affected_sources: string[];
  sample_lines: number[];
}
