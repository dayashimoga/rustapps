import '../bindings/press_ffi.dart';

class PressService {
  final PublishingPressFfiBindings _bindings;

  PressService({PublishingPressFfiBindings? bindings})
      : _bindings = bindings ?? PublishingPressFfiBindings();

  bool get isAvailable => _bindings.isAvailable;
  String? get initError => _bindings.initError;

  Map<String, dynamic> getStats() {
    return _bindings.dispatch('get_stats', {});
  }

  Map<String, dynamic> listProjects() {
    return _bindings.dispatch('list_projects', {});
  }

  Map<String, dynamic> getProject(String projectId) {
    return _bindings.dispatch('get_project', {'project_id': projectId});
  }

  Map<String, dynamic> getArtifact(String projectId) {
    return _bindings.dispatch('get_artifact', {'project_id': projectId});
  }

  Map<String, dynamic> compileProject({
    required String projectId,
    String? template,
    String? format,
  }) {
    return _bindings.dispatch('compile_project', {
      'project_id': projectId,
      if (template != null) 'template': template,
      if (format != null) 'format': format,
    });
  }

  Map<String, dynamic> saveProject({
    required String projectId,
    required String title,
    required String authors,
    required String abstractText,
    required String template,
    required String targetFormat,
    required String sourceMarkdown,
    required String bibtexRaw,
  }) {
    return _bindings.dispatch('save_project', {
      'project_id': projectId,
      'title': title,
      'authors': authors,
      'abstract_text': abstractText,
      'template': template,
      'target_format': targetFormat,
      'source_markdown': sourceMarkdown,
      'bibtex_raw': bibtexRaw,
    });
  }

  Map<String, dynamic> signCopyrightCertificate(String projectId) {
    return _bindings.dispatch('sign_copyright_certificate', {'project_id': projectId});
  }

  Map<String, dynamic> getLicenseStatus() {
    return _bindings.dispatch('get_license_status', {});
  }

  Map<String, dynamic> applyLicenseKey(String key) {
    return _bindings.dispatch('apply_license_key', {'license_key': key});
  }
}
