import '../bindings/debugger_ffi.dart';

class DebuggerService {
  final IndustrialDebuggerFfiBindings _bindings;

  DebuggerService({IndustrialDebuggerFfiBindings? bindings})
      : _bindings = bindings ?? IndustrialDebuggerFfiBindings();

  bool get isAvailable => _bindings.isAvailable;
  String? get initError => _bindings.initError;

  Map<String, dynamic> getStats() {
    return _bindings.dispatch('get_stats', {});
  }

  Map<String, dynamic> listSessions() {
    return _bindings.dispatch('list_sessions', {});
  }

  Map<String, dynamic> getSession(String sessionId) {
    return _bindings.dispatch('get_session', {'session_id': sessionId});
  }

  Map<String, dynamic> decodeRawFrame({
    required String protocol,
    required String rawHex,
    bool isRequest = true,
  }) {
    return _bindings.dispatch('decode_raw_frame', {
      'protocol': protocol,
      'raw_hex': rawHex,
      'is_request': isRequest,
    });
  }

  Map<String, dynamic> generateAuditManifest(String sessionId) {
    return _bindings.dispatch('generate_audit_manifest', {'session_id': sessionId});
  }

  Map<String, dynamic> getLicenseStatus() {
    return _bindings.dispatch('get_license_status', {});
  }

  Map<String, dynamic> applyLicenseKey(String key) {
    return _bindings.dispatch('apply_license_key', {'license_key': key});
  }
}
