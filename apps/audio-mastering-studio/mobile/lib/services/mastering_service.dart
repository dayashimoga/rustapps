import '../bindings/mastering_ffi.dart';

class MasteringService {
  final MasteringFfiBindings _bindings;

  MasteringService({MasteringFfiBindings? bindings})
      : _bindings = bindings ?? MasteringFfiBindings();

  bool get isAvailable => _bindings.isAvailable;
  String? get initError => _bindings.initError;

  dynamic getStudioStats() {
    return _bindings.dispatch('get_studio_stats', {});
  }

  dynamic listProjects() {
    return _bindings.dispatch('list_projects', {});
  }

  dynamic getProject({String projectId = 'latest'}) {
    return _bindings.dispatch('get_project', {'project_id': projectId});
  }

  dynamic createProject({
    String title = 'Mobile Stem Separation Session',
    int sampleRate = 44100,
    Map<String, dynamic>? config,
  }) {
    return _bindings.dispatch('create_project', {
      'title': title,
      'sample_rate': sampleRate,
      if (config != null) 'config': config,
    });
  }

  dynamic remasterProject(String projectId, Map<String, dynamic> config) {
    return _bindings.dispatch('remaster_project', {
      'project_id': projectId,
      'config': config,
    });
  }

  dynamic getLicenseStatus() {
    return _bindings.dispatch('get_license_status', {});
  }
}
