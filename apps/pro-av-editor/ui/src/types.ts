// TypeScript models and IPC bridges for Professional Audio & Video Editor (PAVE)

export type FrameRate =
  | 'Fps23_976'
  | 'Fps24'
  | 'Fps25'
  | 'Fps29_97'
  | 'Fps30'
  | 'Fps50'
  | 'Fps59_94'
  | 'Fps60'
  | 'Fps120';

export type TrackType = 'Video' | 'Audio' | 'Subtitle' | 'Adjustment';

export type ClipType = 'Video' | 'Audio' | 'Image' | 'Title' | 'ColorSolid' | 'Adjustment';

export type BlendMode =
  | 'Normal'
  | 'Multiply'
  | 'Screen'
  | 'Overlay'
  | 'Add'
  | 'Darken'
  | 'Lighten'
  | 'Difference';

export interface Transform2D {
  scale_x: number;
  scale_y: number;
  translate_x: number;
  translate_y: number;
  rotation_degrees: number;
  anchor_x: number;
  anchor_y: number;
}

export interface ColorWheel {
  r: number;
  g: number;
  b: number;
  luma: number;
}

export interface ColorGradingSettings {
  lift: ColorWheel;
  gamma: ColorWheel;
  gain: ColorWheel;
  exposure: number;
  contrast: number;
  saturation: number;
  temperature: number;
  tint: number;
  lut_file_path?: string | null;
}

export type FilterType = 'Bell' | 'LowShelf' | 'HighShelf' | 'LowPass' | 'HighPass' | 'Notch';

export interface EqBand {
  enabled: boolean;
  filter_type: FilterType;
  frequency_hz: number;
  gain_db: number;
  q_factor: number;
}

export interface ParametricEqSettings {
  enabled: boolean;
  bands: EqBand[];
}

export interface CompressorSettings {
  enabled: boolean;
  threshold_db: number;
  ratio: number;
  attack_ms: number;
  release_ms: number;
  makeup_gain_db: number;
}

export interface AnimatedProperty {
  default_value: number;
  keyframes: Array<{ timestamp_ms: number; value: number; curve: string }>;
}

export interface ChromaKeySettings {
  enabled: boolean;
  key_color: [number, number, number];
  similarity: number;
  smoothness: number;
  despill: number;
}

export interface KeyframedTransform {
  scale_x?: AnimatedProperty | null;
  scale_y?: AnimatedProperty | null;
  translate_x?: AnimatedProperty | null;
  translate_y?: AnimatedProperty | null;
  rotation_degrees?: AnimatedProperty | null;
}

export interface Clip {
  id: string;
  name: string;
  clip_type: ClipType;
  track_id: string;
  source_id?: string | null;
  source_in_ms: number;
  source_out_ms: number;
  timeline_in_ms: number;
  timeline_out_ms: number;
  speed_multiplier: number;
  volume: AnimatedProperty;
  pan: number;
  opacity: AnimatedProperty;
  blend_mode: BlendMode;
  transform: Transform2D;
  keyframed_transform?: KeyframedTransform | null;
  chroma_key?: ChromaKeySettings;
  color_grading: ColorGradingSettings;
  eq_settings: ParametricEqSettings;
  compressor: CompressorSettings;
}

export interface Track {
  id: string;
  name: string;
  track_type: TrackType;
  z_index: number;
  muted: boolean;
  solo: boolean;
  locked: boolean;
  clips: Clip[];
}

export interface Transition {
  id: string;
  transition_type: string;
  duration_ms: number;
  from_clip_id: string;
  to_clip_id: string;
  center_timestamp_ms: number;
}

export interface TimelineMarker {
  id: string;
  timestamp_ms: number;
  label: string;
  color_hex: string;
}

export interface MediaSource {
  id: string;
  file_path: string;
  duration_ms: number;
  width: number;
  height: number;
  fps: number;
  sample_rate: number;
  channels: number;
  has_video: boolean;
  has_audio: boolean;
}

export interface ProjectSettings {
  width: number;
  height: number;
  frame_rate: FrameRate;
  sample_rate: number;
  audio_channels: number;
}

export interface TimelineProject {
  schema_version: number;
  project_id: string;
  name: string;
  settings: ProjectSettings;
  tracks: Track[];
  transitions: Transition[];
  markers: TimelineMarker[];
  media_pool?: MediaSource[];
  created_at_secs: number;
  updated_at_secs: number;
}

export interface VideoScopeData {
  histogram_r: number[];
  histogram_g: number[];
  histogram_b: number[];
  histogram_luma: number[];
  waveform_matrix: number[];
  waveform_cols: number;
  waveform_rows: number;
  vectorscope_grid: number[];
  vectorscope_dim: number;
}

export interface LoudnessMeterData {
  momentary_lufs: number;
  short_term_lufs: number;
  integrated_lufs: number;
  true_peak_dbfs: number;
}

export interface ExportPreset {
  id: string;
  name: string;
  description: string;
  width: number;
  height: number;
  frame_rate: FrameRate;
  video_bitrate_kbps: number;
  audio_sample_rate: number;
  audio_channels: number;
  audio_bitrate_kbps: number;
  container: string;
}

export interface ProjectTemplate {
  id: string;
  name: string;
  category: string;
  settings: ProjectSettings;
  initial_tracks: Array<[string, string, TrackType, number]>;
}

export interface MasterExportSummary {
  path: string;
  bytes_written: number;
  duration_ms: number;
  width: number;
  height: number;
  frame_count: number;
  fps: number;
  container: string;
}

// SMPTE timecode format helper
export function formatTimecode(ms: number, fps: number = 60.0): string {
  const totalSeconds = Math.floor(ms / 1000);
  const frames = Math.floor(((ms % 1000) / 1000) * fps);
  const seconds = totalSeconds % 60;
  const minutes = Math.floor(totalSeconds / 60) % 60;
  const hours = Math.floor(totalSeconds / 3600);

  const pad = (n: number) => n.toString().padStart(2, '0');
  return `${pad(hours)}:${pad(minutes)}:${pad(seconds)}:${pad(frames)}`;
}

import { invokeIpc, getLocalFileUrl } from './lib/ipc';

export function getMediaFileUrl(filePath: string): string {
  return getLocalFileUrl(filePath);
}

export async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return invokeIpc<T>(cmd, args);
}

