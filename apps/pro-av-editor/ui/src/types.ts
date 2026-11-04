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

// Tauri IPC Bridge
declare global {
  interface Window {
    __TAURI__?: {
      invoke: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
      dialog?: {
        open: (opts?: Record<string, unknown>) => Promise<string | null>;
        save: (opts?: Record<string, unknown>) => Promise<string | null>;
      };
      fs?: {
        getDrivesAndPlaces: () => Promise<unknown>;
        fileUrl: (path: string) => string;
      };
    };
  }
}

export function getMediaFileUrl(filePath: string): string {
  if (!filePath) return '';
  if (typeof window !== 'undefined' && window.__TAURI__?.fs?.fileUrl) {
    return window.__TAURI__.fs.fileUrl(filePath);
  }
  return '/__file?path=' + encodeURIComponent(filePath);
}

export async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (typeof window !== 'undefined' && window.__TAURI__ && window.__TAURI__.invoke) {
    return window.__TAURI__.invoke<T>(cmd, args);
  }
  if (typeof fetch !== 'undefined' && typeof window !== 'undefined' && window.location && window.location.protocol.startsWith('http')) {
    try {
      const response = await fetch('/__ipc/' + encodeURIComponent(cmd), {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(args || {}),
      });
      if (response.ok) {
        const payload = await response.json();
        if (payload && '__ipc_data' in payload) {
          return payload.__ipc_data as T;
        }
      }
    } catch {
      // Fallback for headless unit test environments
    }
  }
  return mockClientFallback<T>(cmd, args);
}

function mockClientFallback<T>(cmd: string, _args?: Record<string, unknown>): Promise<T> {
  if (cmd === 'get_project_state_cmd' || cmd === 'create_project_cmd' || cmd === 'load_project_cmd') {
    const proj: TimelineProject = {
      schema_version: 1,
      project_id: 'proj_preview',
      name: 'Flagship Studio Project',
      settings: {
        width: 1920,
        height: 1080,
        frame_rate: 'Fps60',
        sample_rate: 48000,
        audio_channels: 2,
      },
      tracks: [
        {
          id: 'v1',
          name: 'Video 1',
          track_type: 'Video',
          z_index: 0,
          muted: false,
          solo: false,
          locked: false,
          clips: [
            {
              id: 'c1',
              name: 'Interview Take 01',
              clip_type: 'Video',
              track_id: 'v1',
              source_in_ms: 0,
              source_out_ms: 5000,
              timeline_in_ms: 0,
              timeline_out_ms: 5000,
              speed_multiplier: 1.0,
              volume: { default_value: 1.0, keyframes: [] },
              pan: 0.0,
              opacity: { default_value: 1.0, keyframes: [] },
              blend_mode: 'Normal',
              transform: {
                scale_x: 1.0,
                scale_y: 1.0,
                translate_x: 0,
                translate_y: 0,
                rotation_degrees: 0,
                anchor_x: 0.5,
                anchor_y: 0.5,
              },
              color_grading: {
                lift: { r: 0, g: 0, b: 0, luma: 0 },
                gamma: { r: 0, g: 0, b: 0, luma: 0 },
                gain: { r: 0, g: 0, b: 0, luma: 0 },
                exposure: 0,
                contrast: 1,
                saturation: 1,
                temperature: 0,
                tint: 0,
              },
              eq_settings: { enabled: true, bands: [] },
              compressor: {
                enabled: false,
                threshold_db: -18,
                ratio: 4,
                attack_ms: 10,
                release_ms: 100,
                makeup_gain_db: 0,
              },
            },
          ],
        },
        {
          id: 'a1',
          name: 'Audio 1',
          track_type: 'Audio',
          z_index: 0,
          muted: false,
          solo: false,
          locked: false,
          clips: [],
        },
      ],
      transitions: [],
      markers: [{ id: 'm1', timestamp_ms: 2500, label: 'Cut 1', color_hex: '#f59e0b' }],
      created_at_secs: 1700000000,
      updated_at_secs: 1700000000,
    };
    return Promise.resolve(proj as unknown as T);
  }
  if (cmd === 'get_video_scopes_cmd') {
    const scopes: VideoScopeData = {
      histogram_r: new Array(256).fill(10),
      histogram_g: new Array(256).fill(10),
      histogram_b: new Array(256).fill(10),
      histogram_luma: new Array(256).fill(10),
      waveform_matrix: new Array(128 * 64).fill(2),
      waveform_cols: 128,
      waveform_rows: 64,
      vectorscope_grid: new Array(64 * 64).fill(3),
      vectorscope_dim: 64,
    };
    return Promise.resolve(scopes as unknown as T);
  }
  if (cmd === 'get_audio_loudness_cmd') {
    const lufs: LoudnessMeterData = {
      momentary_lufs: -18.4,
      short_term_lufs: -19.2,
      integrated_lufs: -23.0,
      true_peak_dbfs: -1.2,
    };
    return Promise.resolve(lufs as unknown as T);
  }
  if (cmd === 'get_templates_cmd') {
    const tpls: ProjectTemplate[] = [
      {
        id: 'tpl_standard_video',
        name: 'Standard Video Editing (1080p60)',
        category: 'Video',
        settings: {
          width: 1920,
          height: 1080,
          frame_rate: 'Fps60',
          sample_rate: 48000,
          audio_channels: 2,
        },
        initial_tracks: [],
      },
    ];
    return Promise.resolve(tpls as unknown as T);
  }
  if (cmd === 'get_export_presets_cmd') {
    const presets: ExportPreset[] = [
      {
        id: 'preset_4k60_master',
        name: '4K UHD Master (60 FPS)',
        description: 'ProRes Master',
        width: 3840,
        height: 2160,
        frame_rate: 'Fps60',
        video_bitrate_kbps: 80000,
        audio_sample_rate: 48000,
        audio_channels: 2,
        audio_bitrate_kbps: 384,
        container: 'mp4',
      },
    ];
    return Promise.resolve(presets as unknown as T);
  }
  return Promise.resolve({} as unknown as T);
}
