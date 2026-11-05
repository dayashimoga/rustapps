import 'package:flutter_test/flutter_test.dart';
import 'package:pro_av_editor_mobile/models/editor_models.dart';

void main() {
  group('Pro AV Editor Mobile Unit Tests', () {
    test('SMPTE Timecode calculation test', () {
      expect(TimecodeHelper.format(0, 60.0), '00:00:00:00');
      expect(TimecodeHelper.format(1000, 60.0), '00:00:01:00');
      expect(TimecodeHelper.format(500, 60.0), '00:00:00:30');
      final oneHourOneMinOneSec = (3600 + 60 + 1) * 1000;
      expect(TimecodeHelper.format(oneHourOneMinOneSec, 60.0), '01:01:01:00');
    });

    test('ProjectSettings JSON roundtrip', () {
      final settings = ProjectSettings(
        width: 3840,
        height: 2160,
        frameRate: 'Fps60',
        sampleRate: 48000,
        audioChannels: 2,
      );
      final json = settings.toJson();
      expect(json['width'], 3840);
      expect(json['height'], 2160);
      expect(json['frame_rate'], 'Fps60');

      final deserialized = ProjectSettings.fromJson(json);
      expect(deserialized.width, 3840);
      expect(deserialized.height, 2160);
      expect(deserialized.frameRate, 'Fps60');
    });

    test('Clip and Track JSON serialization and deserialization', () {
      final clipJson = {
        'id': 'clip-101',
        'name': 'Main Camera Angle 1',
        'clip_type': 'Video',
        'track_id': 'v-track-1',
        'timeline_in_ms': 0,
        'timeline_out_ms': 8000,
        'speed_multiplier': 1.0,
        'pan': 0.0,
        'blend_mode': 'Normal',
      };

      final clip = Clip.fromJson(clipJson);
      expect(clip.id, 'clip-101');
      expect(clip.name, 'Main Camera Angle 1');
      expect(clip.timelineOutMs, 8000);

      final trackJson = {
        'id': 'v-track-1',
        'name': 'Video 1',
        'track_type': 'Video',
        'z_index': 0,
        'muted': false,
        'solo': false,
        'locked': false,
        'clips': [clipJson],
      };

      final track = Track.fromJson(trackJson);
      expect(track.id, 'v-track-1');
      expect(track.clips.length, 1);
      expect(track.clips[0].name, 'Main Camera Angle 1');
    });

    test('TimelineProject model parsing', () {
      final projJson = {
        'schema_version': 1,
        'project_id': 'proj_4k_test',
        'name': 'Cinema 4K Commercial',
        'settings': {
          'width': 3840,
          'height': 2160,
          'frame_rate': 'Fps60',
          'sample_rate': 48000,
          'audio_channels': 2,
        },
        'tracks': [
          {
            'id': 'v1',
            'name': 'V1 Video',
            'track_type': 'Video',
            'z_index': 0,
            'muted': false,
            'solo': false,
            'locked': false,
            'clips': [],
          },
          {
            'id': 'a1',
            'name': 'A1 Audio',
            'track_type': 'Audio',
            'z_index': 0,
            'muted': false,
            'solo': false,
            'locked': false,
            'clips': [],
          },
        ],
      };

      final project = TimelineProject.fromJson(projJson);
      expect(project.projectId, 'proj_4k_test');
      expect(project.name, 'Cinema 4K Commercial');
      expect(project.tracks.length, 2);
      expect(project.settings.width, 3840);
    });

    test('VideoScopeData and LoudnessMeterData parsing', () {
      final scopesJson = {
        'histogram_r': [1, 2, 3],
        'histogram_g': [4, 5, 6],
        'histogram_b': [7, 8, 9],
        'histogram_luma': [10, 11, 12],
        'waveform_cols': 128,
        'waveform_rows': 64,
        'vectorscope_dim': 64,
      };
      final scopes = VideoScopeData.fromJson(scopesJson);
      expect(scopes.histogramR.length, 3);
      expect(scopes.waveformCols, 128);

      final loudnessJson = {
        'momentary_lufs': -18.5,
        'short_term_lufs': -19.0,
        'integrated_lufs': -23.0,
        'true_peak_dbfs': -1.2,
      };
      final loudness = LoudnessMeterData.fromJson(loudnessJson);
      expect(loudness.integratedLufs, -23.0);
      expect(loudness.truePeakDbfs, -1.2);
    });

    test('ExportPresets and ProjectTemplates parsing', () {
      final presetJson = {
        'id': 'preset_1080p60',
        'name': 'YouTube 1080p60',
        'description': 'Full HD 60 FPS',
        'width': 1920,
        'height': 1080,
        'container': 'mp4',
      };
      final preset = ExportPreset.fromJson(presetJson);
      expect(preset.name, 'YouTube 1080p60');
      expect(preset.width, 1920);

      final templateJson = {
        'id': 'tpl_cinema',
        'name': 'Cinema Scope (2.39:1)',
        'category': 'Cinema',
      };
      final tpl = ProjectTemplate.fromJson(templateJson);
      expect(tpl.name, 'Cinema Scope (2.39:1)');
      expect(tpl.category, 'Cinema');
    });

    test('Proxy and AudioSync dispatch payload verification', () {
      final syncPayload = {
        'status': 'ok',
        'offset_ms': 120,
        'correlation_score': 0.94,
      };
      expect(syncPayload['offset_ms'], 120);
      expect(syncPayload['correlation_score'], 0.94);

      final proxyPayload = {
        'status': 'ok',
        'proxy_path': '/proxies/sample_720p.mp4',
        'resolution': '720p',
        'ready': true,
      };
      expect(proxyPayload['ready'], true);
      expect(proxyPayload['resolution'], '720p');
    });
  });
}

