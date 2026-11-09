import '../bindings/sanitizer_ffi.dart';

class SanitizerService {
  final PrivacyMediaSanitizerFfiBindings _bindings;

  SanitizerService({PrivacyMediaSanitizerFfiBindings? bindings})
      : _bindings = bindings ?? PrivacyMediaSanitizerFfiBindings();

  bool get isAvailable => _bindings.isAvailable;
  String? get initError => _bindings.initError;

  Map<String, dynamic> getStats() {
    return _bindings.dispatch('get_stats', {});
  }

  Map<String, dynamic> listMedia() {
    return _bindings.dispatch('list_media', {});
  }

  Map<String, dynamic> getMediaReport(String fileId) {
    return _bindings.dispatch('get_media_report', {'file_id': fileId});
  }

  Map<String, dynamic> inspectUpload({
    required String filename,
    required String rawText,
  }) {
    return _bindings.dispatch('inspect_upload', {
      'filename': filename,
      'raw_text': rawText,
    });
  }

  Map<String, dynamic> sanitizeFile(String fileId) {
    return _bindings.dispatch('sanitize_file', {'file_id': fileId});
  }

  Map<String, dynamic> getSanitizedResult(String fileId) {
    return _bindings.dispatch('get_sanitized_result', {'file_id': fileId});
  }

  Map<String, dynamic> generatePrivacyCertificate(String fileId) {
    return _bindings.dispatch('generate_privacy_certificate', {'file_id': fileId});
  }

  Map<String, dynamic> getLicenseStatus() {
    return _bindings.dispatch('get_license_status', {});
  }

  Map<String, dynamic> applyLicenseKey(String key) {
    return _bindings.dispatch('apply_license_key', {'license_key': key});
  }
}
