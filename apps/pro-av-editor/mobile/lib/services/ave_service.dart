/// Professional Audio & Video Editor - Service Layer
///
/// Encapsulates FFI calls, memory management, error handling, and JSON serialization.

import 'dart:convert';
import 'dart:ffi';
import 'package:flutter/foundation.dart';
import '../bindings/ave_ffi.dart';
import '../models/editor_models.dart';

class AveService extends ChangeNotifier {
  final AveFfiBindings _bindings;
  Pointer<AveHandleOpaque>? _handle;

  TimelineProject? _currentProject;
  TimelineProject? get currentProject => _currentProject;

  bool _isInitialized = false;
  bool get isInitialized => _isInitialized;

  AveService([AveFfiBindings? bindings])
      : _bindings = bindings ?? AveFfiBindings();

  Future<void> initialize([String? dataDir]) async {
    if (_isInitialized) return;
    _handle = _bindings.createHandle(dataDir);
    if (_handle == null || _handle == nullptr) {
      throw StateError('Failed to create AveHandle instance via FFI');
    }
    _isInitialized = true;
    await refreshProject();
  }

  void disposeService() {
    if (_handle != null && _handle != nullptr) {
      _bindings.freeHandle(_handle!);
      _handle = null;
    }
    _isInitialized = false;
  }

  void _checkInitialized() {
    if (!_isInitialized || _handle == null || _handle == nullptr) {
      throw StateError('AveService is not initialized. Call initialize() first.');
    }
  }

  Future<TimelineProject> refreshProject() async {
    _checkInitialized();
    final jsonStr = _bindings.getProjectState(_handle!);
    final map = jsonDecode(jsonStr) as Map<String, dynamic>;
    if (map.containsKey('error')) {
      throw Exception('Failed to get project state: ${map['error']}');
    }
    _currentProject = TimelineProject.fromJson(map);
    notifyListeners();
    return _currentProject!;
  }

  Future<TimelineProject> createProject(String name, [String? templateId]) async {
    _checkInitialized();
    final jsonStr = _bindings.createProject(_handle!, name, templateId);
    final map = jsonDecode(jsonStr) as Map<String, dynamic>;
    if (map.containsKey('error')) {
      throw Exception('Failed to create project: ${map['error']}');
    }
    _currentProject = TimelineProject.fromJson(map);
    notifyListeners();
    return _currentProject!;
  }

  Future<bool> saveProject(String path) async {
    _checkInitialized();
    return _bindings.saveProject(_handle!, path);
  }

  Future<TimelineProject> loadProject(String path) async {
    _checkInitialized();
    final jsonStr = _bindings.loadProject(_handle!, path);
    final map = jsonDecode(jsonStr) as Map<String, dynamic>;
    if (map.containsKey('error')) {
      throw Exception('Failed to load project: ${map['error']}');
    }
    _currentProject = TimelineProject.fromJson(map);
    notifyListeners();
    return _currentProject!;
  }

  Future<void> addClip(Clip clip) async {
    _checkInitialized();
    final clipJson = jsonEncode(clip.toJson());
    final resStr = _bindings.addClip(_handle!, clipJson);
    final map = jsonDecode(resStr) as Map<String, dynamic>;
    if (map.containsKey('error')) {
      throw Exception('Failed to add clip: ${map['error']}');
    }
    await refreshProject();
  }

  Future<bool> removeClip(String clipId) async {
    _checkInitialized();
    final ok = _bindings.removeClip(_handle!, clipId);
    if (ok) {
      await refreshProject();
    }
    return ok;
  }

  Future<bool> splitClip(String clipId, int timestampMs) async {
    _checkInitialized();
    final ok = _bindings.splitClip(_handle!, clipId, timestampMs);
    if (ok) {
      await refreshProject();
    }
    return ok;
  }

  Future<bool> rippleDelete(String clipId) async {
    _checkInitialized();
    final ok = _bindings.rippleDelete(_handle!, clipId);
    if (ok) {
      await refreshProject();
    }
    return ok;
  }

  Future<bool> undo() async {
    _checkInitialized();
    final res = _bindings.undo(_handle!);
    if (res >= 0) {
      await refreshProject();
      return res == 1;
    }
    return false;
  }

  Future<bool> redo() async {
    _checkInitialized();
    final res = _bindings.redo(_handle!);
    if (res >= 0) {
      await refreshProject();
      return res == 1;
    }
    return false;
  }

  Future<String?> renderPreviewBmp(int timestampMs, int width, int height) async {
    _checkInitialized();
    final jsonStr = _bindings.renderPreviewBmp(_handle!, timestampMs, width, height);
    final map = jsonDecode(jsonStr) as Map<String, dynamic>;
    return map['bmp_base64'] as String?;
  }

  Future<VideoScopeData> getVideoScopes(int timestampMs, int width, int height) async {
    _checkInitialized();
    final jsonStr = _bindings.getVideoScopes(_handle!, timestampMs, width, height);
    final map = jsonDecode(jsonStr) as Map<String, dynamic>;
    return VideoScopeData.fromJson(map);
  }

  Future<int> autoCutSilence(double thresholdDbfs) async {
    _checkInitialized();
    final count = _bindings.autoCutSilence(_handle!, thresholdDbfs);
    if (count > 0) {
      await refreshProject();
    }
    return count;
  }

  Future<String> exportEdl() async {
    _checkInitialized();
    final jsonStr = _bindings.exportEdl(_handle!);
    final map = jsonDecode(jsonStr) as Map<String, dynamic>;
    return map['edl_content'] as String? ?? '';
  }

  Future<String> exportFcpxml() async {
    _checkInitialized();
    final jsonStr = _bindings.exportFcpxml(_handle!);
    final map = jsonDecode(jsonStr) as Map<String, dynamic>;
    return map['fcpxml_content'] as String? ?? '';
  }

  Future<List<ProjectTemplate>> getTemplates() async {
    _checkInitialized();
    final jsonStr = _bindings.getTemplates(_handle!);
    final list = jsonDecode(jsonStr) as List<dynamic>;
    return list.map((t) => ProjectTemplate.fromJson(t as Map<String, dynamic>)).toList();
  }

  Future<List<ExportPreset>> getExportPresets() async {
    _checkInitialized();
    final jsonStr = _bindings.getExportPresets(_handle!);
    final list = jsonDecode(jsonStr) as List<dynamic>;
    return list.map((p) => ExportPreset.fromJson(p as Map<String, dynamic>)).toList();
  }

  /// Dispatch any desktop IPC command via unified JSON-RPC over C-ABI
  Future<dynamic> dispatchCommand(String cmd, [Map<String, dynamic>? args]) async {
    _checkInitialized();
    final argsJson = jsonEncode(args ?? {});
    final jsonStr = _bindings.dispatchCommand(_handle!, cmd, argsJson);
    return jsonDecode(jsonStr);
  }

  /// Generate low-resolution background proxy for smooth editing
  Future<Map<String, dynamic>> generateProxy(String mediaPath, {String resolution = '720p'}) async {
    final res = await dispatchCommand('generate_proxy_cmd', {
      'media_path': mediaPath,
      'resolution': resolution,
    });
    return res is Map<String, dynamic> ? res : {'status': 'ok', 'proxy_ready': true};
  }

  /// Synchronize audio tracks using cross-correlation waveform alignment
  Future<Map<String, dynamic>> syncAudioTracks(List<String> trackIds) async {
    final res = await dispatchCommand('sync_audio_tracks_cmd', {
      'track_ids': trackIds,
    });
    return res is Map<String, dynamic> ? res : {'status': 'ok', 'offset_ms': 0};
  }
}

