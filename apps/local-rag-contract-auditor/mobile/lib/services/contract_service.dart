import '../bindings/contract_ffi.dart';

class ContractService {
  final LocalRagContractAuditorFfiBindings _bindings;

  ContractService({LocalRagContractAuditorFfiBindings? bindings})
      : _bindings = bindings ?? LocalRagContractAuditorFfiBindings();

  bool get isAvailable => _bindings.isAvailable;
  String? get initError => _bindings.initError;

  Map<String, dynamic> getStats() {
    return _bindings.dispatch('get_stats', {});
  }

  Map<String, dynamic> listContracts() {
    return _bindings.dispatch('list_contracts', {});
  }

  Map<String, dynamic> getContract(String documentId) {
    return _bindings.dispatch('get_contract', {'document_id': documentId});
  }

  Map<String, dynamic> auditContractText({
    required String title,
    required String rawText,
    String contractType = 'saas_master_services_agreement',
  }) {
    return _bindings.dispatch('audit_contract_text', {
      'title': title,
      'raw_text': rawText,
      'contract_type': contractType,
    });
  }

  Map<String, dynamic> compareContracts({
    required String baseId,
    required String revisedId,
  }) {
    return _bindings.dispatch('compare_contracts', {
      'base_id': baseId,
      'revised_id': revisedId,
    });
  }

  Map<String, dynamic> generateAuditCertificate(String documentId) {
    return _bindings.dispatch('generate_audit_certificate', {
      'document_id': documentId,
    });
  }

  Map<String, dynamic> getLicenseStatus() {
    return _bindings.dispatch('get_license_status', {});
  }

  Map<String, dynamic> applyLicenseKey(String key) {
    return _bindings.dispatch('apply_license_key', {'license_key': key});
  }
}
