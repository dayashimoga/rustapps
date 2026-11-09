import '../bindings/recorder_ffi.dart';

class RecorderService {
  final RecorderFfiBindings _bindings;

  RecorderService({RecorderFfiBindings? bindings})
      : _bindings = bindings ?? RecorderFfiBindings();

  bool get isAvailable => _bindings.isAvailable;
  String? get initError => _bindings.initError;

  dynamic getStudioStats() {
    return _bindings.dispatch('get_studio_stats', {});
  }

  dynamic listRecordings() {
    return _bindings.dispatch('list_recordings', {});
  }

  dynamic getRecording({String sessionId = 'latest'}) {
    return _bindings.dispatch('get_recording', {'session_id': sessionId});
  }

  dynamic createRecording({
    String title = 'Mobile App Tutorial & Interaction Walkthrough',
    int durationMs = 15000,
    List<Map<String, dynamic>>? events,
    List<Map<String, dynamic>>? subtitles,
  }) {
    return _bindings.dispatch('create_recording', {
      'title': title,
      'duration_ms': durationMs,
      'events': events ?? [],
      'subtitles': subtitles ?? [],
    });
  }

  dynamic adjustAutoZooms(String sessionId, double scale) {
    return _bindings.dispatch('adjust_auto_zooms', {
      'session_id': sessionId,
      'scale': scale,
    });
  }

  dynamic exportTutorial(String sessionId, {String format = 'Markdown', bool includeCaptions = true}) {
    return _bindings.dispatch('export_tutorial', {
      'session_id': sessionId,
      'config': {
        'format': format,
        'include_captions': includeCaptions,
      }
    });
  }

  dynamic getLicenseStatus() {
    return _bindings.dispatch('get_license_status', {});
  }
}

