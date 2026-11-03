import { describe, it, expect } from 'vitest';
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
