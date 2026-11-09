import '../bindings/dfir_ffi.dart';

class DfirService {
  final DfirFfiBindings _bindings;

  DfirService({DfirFfiBindings? bindings})
      : _bindings = bindings ?? DfirFfiBindings();

  bool get isAvailable => _bindings.isAvailable;
  String? get initError => _bindings.initError;

  dynamic getDfirStats() {
    return _bindings.dispatch('get_dfir_stats', {});
  }

  dynamic executeTriage({String? targetDir}) {
    return _bindings.dispatch('execute_triage', {
      if (targetDir != null && targetDir.isNotEmpty) 'target_dir': targetDir,
    });
  }

  dynamic listTriageRuns() {
    return _bindings.dispatch('list_triage_runs', {});
  }

  dynamic getTriagePackage(String triageId) {
    return _bindings.dispatch('get_triage_package', {'triage_id': triageId});
  }

  dynamic scanDirectory(String path, {int maxDepth = 3}) {
    return _bindings.dispatch('scan_directory', {
      'path': path,
      'max_depth': maxDepth,
    });
  }

  dynamic getLicenseStatus() {
    return _bindings.dispatch('get_license_status', {});
  }
}

