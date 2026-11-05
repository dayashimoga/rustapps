import { beforeAll, describe, it, expect } from 'vitest';
import {
  formatTimecode,
  tauriInvoke,
  type TimelineProject,
  type VideoScopeData,
  type LoudnessMeterData,
  type ProjectTemplate,
  type ExportPreset,
} from './types';

describe('Pro AV Editor UI & IPC Bridge Tests', () => {
  beforeAll(() => {
    (globalThis as any).window = (globalThis as any).window || {};
    (globalThis as any).window.__TAURI__ = {
      invoke: async (cmd: string, _args: any) => {
        if (cmd === 'get_project_state_cmd') {
          return {
            project_id: 'proj_preview',
            name: 'Flagship Studio Project',
            settings: {
              width: 1920,
              height: 1080,
              frame_rate: 'Fps60',
            },
            tracks: [
              {
                id: 't1',
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
                    track_id: 't1',
                    source_in_ms: 0,
                    source_out_ms: 5000,
                    timeline_in_ms: 0,
                    timeline_out_ms: 5000,
                    speed_multiplier: 1.0,
                    volume: { default_value: 1.0, keyframes: [] },
                    pan: 0,
                    opacity: { default_value: 1.0, keyframes: [] },
                    blend_mode: 'Normal',
                    transform: {
                      scale_x: 1,
                      scale_y: 1,
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
                      lut_file_path: null,
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
                id: 't2',
                name: 'Audio 1',
                track_type: 'Audio',
                z_index: 1,
                muted: false,
                solo: false,
                locked: false,
                clips: [],
              },
            ],
            transitions: [],
            markers: [],
            media_pool: [],
          };
        }
        if (cmd === 'get_video_scopes_cmd') {
          return {
            histogram_r: new Array(256).fill(0),
            histogram_g: new Array(256).fill(0),
            histogram_b: new Array(256).fill(0),
            histogram_luma: new Array(256).fill(0),
            waveform_cols: 128,
            waveform_rows: 64,
            waveform_matrix: new Array(128 * 64).fill(0),
            vectorscope_dim: 64,
            vectorscope_grid: new Array(64 * 64).fill(0),
          };
        }
        if (cmd === 'get_audio_loudness_cmd') {
          return {
            integrated_lufs: -23.0,
            short_term_lufs: -22.5,
            momentary_lufs: -21.8,
            true_peak_dbfs: -1.2,
            lra_lu: 4.5,
          };
        }
        if (cmd === 'get_templates_cmd') {
          return [
            {
              id: 'tpl_standard_video',
              name: 'Standard 1080p60 Video',
              settings: { width: 1920, height: 1080, frame_rate: 'Fps60' },
            },
          ];
        }
        if (cmd === 'get_export_presets_cmd') {
          return [
            {
              id: 'preset_4k60_master',
              name: '4K UHD Master',
              width: 3840,
              height: 2160,
              container: 'mp4',
            },
          ];
        }
        throw new Error(`Unhandled mock command: ${cmd}`);
      },
    };
  });
  it('should accurately calculate SMPTE timecodes across frame rates', () => {
    expect(formatTimecode(0, 60.0)).toBe('00:00:00:00');
    expect(formatTimecode(1000, 60.0)).toBe('00:00:01:00');
    expect(formatTimecode(500, 60.0)).toBe('00:00:00:30');
    // 1 hour, 1 minute, 1 second, 0 frames
    const oneHourOneMinOneSec = (3600 + 60 + 1) * 1000;
    expect(formatTimecode(oneHourOneMinOneSec, 60.0)).toBe('01:01:01:00');
  });

  it('should retrieve project state via Tauri IPC bridge fallback', async () => {
    const project = await tauriInvoke<TimelineProject>('get_project_state_cmd');
    expect(project).toBeDefined();
    expect(project.project_id).toBe('proj_preview');
    expect(project.name).toBe('Flagship Studio Project');
    expect(project.settings.width).toBe(1920);
    expect(project.settings.height).toBe(1080);
    expect(project.settings.frame_rate).toBe('Fps60');
    expect(project.tracks.length).toBeGreaterThanOrEqual(2);
    expect(project.tracks[0]!.track_type).toBe('Video');
    expect(project.tracks[0]!.clips.length).toBe(1);
    expect(project.tracks[0]!.clips[0]!.name).toBe('Interview Take 01');
  });

  it('should retrieve video scopes data for color grading', async () => {
    const scopes = await tauriInvoke<VideoScopeData>('get_video_scopes_cmd', {
      timestampMs: 1000,
    });
    expect(scopes).toBeDefined();
    expect(scopes.histogram_r.length).toBe(256);
    expect(scopes.waveform_cols).toBe(128);
    expect(scopes.waveform_rows).toBe(64);
    expect(scopes.waveform_matrix.length).toBe(128 * 64);
    expect(scopes.vectorscope_dim).toBe(64);
    expect(scopes.vectorscope_grid.length).toBe(64 * 64);
  });

  it('should retrieve audio loudness meter data (EBU R128 compliance)', async () => {
    const meter = await tauriInvoke<LoudnessMeterData>('get_audio_loudness_cmd', {
      startMs: 0,
      endMs: 2000,
    });
    expect(meter).toBeDefined();
    expect(meter.integrated_lufs).toBeCloseTo(-23.0, 1);
    expect(meter.true_peak_dbfs).toBeLessThan(0.0);
  });

  it('should retrieve export presets and project templates', async () => {
    const templates = await tauriInvoke<ProjectTemplate[]>('get_templates_cmd');
    expect(templates.length).toBeGreaterThan(0);
    expect(templates[0]!.id).toBe('tpl_standard_video');
    expect(templates[0]!.settings.width).toBe(1920);

    const presets = await tauriInvoke<ExportPreset[]>('get_export_presets_cmd');
    expect(presets.length).toBeGreaterThan(0);
    expect(presets[0]!.id).toBe('preset_4k60_master');
    expect(presets[0]!.width).toBe(3840);
    expect(presets[0]!.height).toBe(2160);
    expect(presets[0]!.container).toBe('mp4');
  });
});
