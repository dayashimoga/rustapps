import '../bindings/launcher_ffi.dart';

class LauncherService {
  final SuiteLauncherFfiBindings _bindings;

  LauncherService({SuiteLauncherFfiBindings? bindings})
      : _bindings = bindings ?? SuiteLauncherFfiBindings();

  bool get isAvailable => _bindings.isAvailable;
  String? get initError => _bindings.initError;

  Map<String, dynamic> getStats() {
    return _bindings.dispatch('get_stats', {});
  }

  Map<String, dynamic> listApps([String? category]) {
    return _bindings.dispatch('list_apps', {
      if (category != null) 'category': category,
    });
  }

  Map<String, dynamic> getApp(String appId) {
    return _bindings.dispatch('get_app', {'app_id': appId});
  }

  Map<String, dynamic> runDiagnostic(String appId) {
    return _bindings.dispatch('run_diagnostic', {'app_id': appId});
  }

  Map<String, dynamic> signSuiteManifest() {
    return _bindings.dispatch('sign_suite_manifest', {});
  }

  Map<String, dynamic> getLicenseStatus() {
    return _bindings.dispatch('get_license_status', {});
  }

  Map<String, dynamic> applyLicenseKey(String key) {
    return _bindings.dispatch('apply_license_key', {'license_key': key});
  }
}
