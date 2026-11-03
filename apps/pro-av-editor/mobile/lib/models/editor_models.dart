/// Professional Audio & Video Editor - Dart Domain Models

import 'dart:convert';

class ProjectSettings {
  final int width;
  final int height;
  final String frameRate;
  final int sampleRate;
  final int audioChannels;

  ProjectSettings({
    required this.width,
    required this.height,
    required this.frameRate,
    required this.sampleRate,
    required this.audioChannels,
  });

  factory ProjectSettings.fromJson(Map<String, dynamic> json) {
    return ProjectSettings(
      width: json['width'] as int? ?? 1920,
      height: json['height'] as int? ?? 1080,
      frameRate: json['frame_rate'] as String? ?? 'Fps60',
      sampleRate: json['sample_rate'] as int? ?? 48000,
      audioChannels: json['audio_channels'] as int? ?? 2,
    );
  }

  Map<String, dynamic> toJson() => {
        'width': width,
        'height': height,
        'frame_rate': frameRate,
        'sample_rate': sampleRate,
        'audio_channels': audioChannels,
      };
}

class Clip {
  final String id;
  final String name;
  final String clipType;
  final String trackId;
  final int timelineInMs;
  final int timelineOutMs;
  final double speedMultiplier;
  final double pan;
  final String blendMode;

  Clip({
    required this.id,
    required this.name,
    required this.clipType,
    required this.trackId,
    required this.timelineInMs,
    required this.timelineOutMs,
    this.speedMultiplier = 1.0,
    this.pan = 0.0,
    this.blendMode = 'Normal',
  });

  factory Clip.fromJson(Map<String, dynamic> json) {
    return Clip(
      id: json['id'] as String? ?? '',
      name: json['name'] as String? ?? 'Untitled Clip',
      clipType: json['clip_type'] as String? ?? 'Video',
      trackId: json['track_id'] as String? ?? '',
      timelineInMs: json['timeline_in_ms'] as int? ?? 0,
      timelineOutMs: json['timeline_out_ms'] as int? ?? 5000,
      speedMultiplier: (json['speed_multiplier'] as num?)?.toDouble() ?? 1.0,
      pan: (json['pan'] as num?)?.toDouble() ?? 0.0,
      blendMode: json['blend_mode'] as String? ?? 'Normal',
    );
  }

  Map<String, dynamic> toJson() => {
        'id': id,
        'name': name,
        'clip_type': clipType,
        'track_id': trackId,
        'timeline_in_ms': timelineInMs,
        'timeline_out_ms': timelineOutMs,
        'speed_multiplier': speedMultiplier,
        'pan': pan,
        'blend_mode': blendMode,
        'volume': {'default_value': 1.0, 'keyframes': []},
        'opacity': {'default_value': 1.0, 'keyframes': []},
        'transform': {
          'scale_x': 1.0,
          'scale_y': 1.0,
          'translate_x': 0.0,
          'translate_y': 0.0,
          'rotation_degrees': 0.0,
          'anchor_x': 0.5,
          'anchor_y': 0.5,
        },
        'color_grading': {
          'lift': {'r': 0.0, 'g': 0.0, 'b': 0.0, 'luma': 0.0},
          'gamma': {'r': 0.0, 'g': 0.0, 'b': 0.0, 'luma': 0.0},
          'gain': {'r': 0.0, 'g': 0.0, 'b': 0.0, 'luma': 0.0},
          'exposure': 0.0,
          'contrast': 1.0,
          'saturation': 1.0,
          'temperature': 0.0,
          'tint': 0.0,
        },
        'eq_settings': {'enabled': true, 'bands': []},
        'compressor': {
          'enabled': false,
          'threshold_db': -18.0,
          'ratio': 4.0,
          'attack_ms': 10.0,
          'release_ms': 100.0,
          'makeup_gain_db': 0.0,
        },
      };
}

class Track {
  final String id;
  final String name;
  final String trackType;
  final int zIndex;
  final bool muted;
  final bool solo;
  final bool locked;
  final List<Clip> clips;

  Track({
    required this.id,
    required this.name,
    required this.trackType,
    required this.zIndex,
    required this.muted,
    required this.solo,
    required this.locked,
    required this.clips,
  });

  factory Track.fromJson(Map<String, dynamic> json) {
    return Track(
      id: json['id'] as String? ?? '',
      name: json['name'] as String? ?? 'Track',
      trackType: json['track_type'] as String? ?? 'Video',
      zIndex: json['z_index'] as int? ?? 0,
      muted: json['muted'] as bool? ?? false,
      solo: json['solo'] as bool? ?? false,
      locked: json['locked'] as bool? ?? false,
      clips: (json['clips'] as List<dynamic>?)
              ?.map((c) => Clip.fromJson(c as Map<String, dynamic>))
              .toList() ??
          [],
    );
  }

  Map<String, dynamic> toJson() => {
        'id': id,
        'name': name,
        'track_type': trackType,
        'z_index': zIndex,
        'muted': muted,
        'solo': solo,
        'locked': locked,
        'clips': clips.map((c) => c.toJson()).toList(),
      };
}

class TimelineProject {
  final int schemaVersion;
  final String projectId;
  final String name;
  final ProjectSettings settings;
  final List<Track> tracks;

  TimelineProject({
    required this.schemaVersion,
    required this.projectId,
    required this.name,
    required this.settings,
    required this.tracks,
  });

  factory TimelineProject.fromJson(Map<String, dynamic> json) {
    return TimelineProject(
      schemaVersion: json['schema_version'] as int? ?? 1,
      projectId: json['project_id'] as String? ?? '',
      name: json['name'] as String? ?? 'Untitled Studio Project',
      settings: ProjectSettings.fromJson(
          json['settings'] as Map<String, dynamic>? ?? {}),
      tracks: (json['tracks'] as List<dynamic>?)
              ?.map((t) => Track.fromJson(t as Map<String, dynamic>))
              .toList() ??
          [],
    );
  }

  Map<String, dynamic> toJson() => {
        'schema_version': schemaVersion,
        'project_id': projectId,
        'name': name,
        'settings': settings.toJson(),
        'tracks': tracks.map((t) => t.toJson()).toList(),
        'transitions': [],
        'markers': [],
      };
}

class VideoScopeData {
  final List<int> histogramR;
  final List<int> histogramG;
  final List<int> histogramB;
  final List<int> histogramLuma;
  final int waveformCols;
  final int waveformRows;
  final int vectorscopeDim;

  VideoScopeData({
    required this.histogramR,
    required this.histogramG,
    required this.histogramB,
    required this.histogramLuma,
    required this.waveformCols,
    required this.waveformRows,
    required this.vectorscopeDim,
  });

  factory VideoScopeData.fromJson(Map<String, dynamic> json) {
    return VideoScopeData(
      histogramR: (json['histogram_r'] as List<dynamic>?)?.cast<int>() ?? [],
      histogramG: (json['histogram_g'] as List<dynamic>?)?.cast<int>() ?? [],
      histogramB: (json['histogram_b'] as List<dynamic>?)?.cast<int>() ?? [],
      histogramLuma: (json['histogram_luma'] as List<dynamic>?)?.cast<int>() ?? [],
      waveformCols: json['waveform_cols'] as int? ?? 128,
      waveformRows: json['waveform_rows'] as int? ?? 64,
      vectorscopeDim: json['vectorscope_dim'] as int? ?? 64,
    );
  }
}

class LoudnessMeterData {
  final double momentaryLufs;
  final double shortTermLufs;
  final double integratedLufs;
  final double truePeakDbfs;

  LoudnessMeterData({
    required this.momentaryLufs,
    required this.shortTermLufs,
    required this.integratedLufs,
    required this.truePeakDbfs,
  });

  factory LoudnessMeterData.fromJson(Map<String, dynamic> json) {
    return LoudnessMeterData(
      momentaryLufs: (json['momentary_lufs'] as num?)?.toDouble() ?? -24.0,
      shortTermLufs: (json['short_term_lufs'] as num?)?.toDouble() ?? -24.0,
      integratedLufs: (json['integrated_lufs'] as num?)?.toDouble() ?? -23.0,
      truePeakDbfs: (json['true_peak_dbfs'] as num?)?.toDouble() ?? -1.0,
    );
  }
}

class ExportPreset {
  final String id;
  final String name;
  final String description;
  final int width;
  final int height;
  final String container;

  ExportPreset({
    required this.id,
    required this.name,
    required this.description,
    required this.width,
    required this.height,
    required this.container,
  });

  factory ExportPreset.fromJson(Map<String, dynamic> json) {
    return ExportPreset(
      id: json['id'] as String? ?? '',
      name: json['name'] as String? ?? '',
      description: json['description'] as String? ?? '',
      width: json['width'] as int? ?? 1920,
      height: json['height'] as int? ?? 1080,
      container: json['container'] as String? ?? 'mp4',
    );
  }
}

class ProjectTemplate {
  final String id;
  final String name;
  final String category;

  ProjectTemplate({
    required this.id,
    required this.name,
    required this.category,
  });

  factory ProjectTemplate.fromJson(Map<String, dynamic> json) {
    return ProjectTemplate(
      id: json['id'] as String? ?? '',
      name: json['name'] as String? ?? '',
      category: json['category'] as String? ?? 'Standard',
    );
  }
}

class TimecodeHelper {
  static String format(int ms, [double fps = 60.0]) {
    final totalSeconds = ms ~/ 1000;
    final frames = ((ms % 1000) / 1000 * fps).floor();
    final seconds = totalSeconds % 60;
    final minutes = (totalSeconds ~/ 60) % 60;
    final hours = totalSeconds ~/ 3600;

    String pad(int n) => n.toString().padLeft(2, '0');
    return '${pad(hours)}:${pad(minutes)}:${pad(seconds)}:${pad(frames)}';
  }
}

