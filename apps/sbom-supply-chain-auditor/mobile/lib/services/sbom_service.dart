import '../bindings/sbom_ffi.dart';

class SbomService {
  final SbomSupplyChainAuditorFfiBindings _bindings;

  SbomService({SbomSupplyChainAuditorFfiBindings? bindings})
      : _bindings = bindings ?? SbomSupplyChainAuditorFfiBindings();

  bool get isAvailable => _bindings.isAvailable;
  String? get initError => _bindings.initError;

  Map<String, dynamic> getStats() {
    return _bindings.dispatch('get_stats', {});
  }

  Map<String, dynamic> getAuditReport() {
    return _bindings.dispatch('get_audit_report', {});
  }

  Map<String, dynamic> auditCycloneDxJson(String jsonContent) {
    return _bindings.dispatch('audit_cyclonedx_json', {'json_content': jsonContent});
  }

  Map<String, dynamic> getLicenseStatus() {
    return _bindings.dispatch('get_license_status', {});
  }

  Map<String, dynamic> applyLicenseKey(String licenseKey) {
    return _bindings.dispatch('apply_license_key', {'license_key': licenseKey});
  }
}
