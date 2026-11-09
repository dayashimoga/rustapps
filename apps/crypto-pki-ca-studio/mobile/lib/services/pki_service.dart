import '../bindings/pki_ffi.dart';

class PkiService {
  final CryptoPkiCaStudioFfiBindings _bindings;

  PkiService({CryptoPkiCaStudioFfiBindings? bindings})
      : _bindings = bindings ?? CryptoPkiCaStudioFfiBindings();

  bool get isAvailable => _bindings.isAvailable;
  String? get initError => _bindings.initError;

  Map<String, dynamic> getStats() {
    return _bindings.dispatch('get_stats', {});
  }

  Map<String, dynamic> getHierarchy() {
    return _bindings.dispatch('get_hierarchy', {});
  }

  Map<String, dynamic> getCertificateDetail(String certId) {
    return _bindings.dispatch('get_certificate_detail', {'certificate_id': certId});
  }

  Map<String, dynamic> signCsr({
    required String caId,
    required String commonName,
    String certType = 'tls_server',
    List<String> sans = const [],
    int validityDays = 365,
  }) {
    return _bindings.dispatch('sign_csr', {
      'ca_id': caId,
      'common_name': commonName,
      'cert_type': certType,
      'sans': sans,
      'validity_days': validityDays,
    });
  }

  Map<String, dynamic> revokeCertificate(String certId, {String reason = 'KeyCompromise'}) {
    return _bindings.dispatch('revoke_certificate', {
      'certificate_id': certId,
      'reason': reason,
    });
  }

  Map<String, dynamic> getCrls() {
    return _bindings.dispatch('get_crls', {});
  }

  Map<String, dynamic> getShamirCeremonies() {
    return _bindings.dispatch('get_shamir_ceremonies', {});
  }

  Map<String, dynamic> reconstructShamirKey(List<Map<String, dynamic>> shards, int thresholdM) {
    return _bindings.dispatch('reconstruct_shamir_key', {
      'shards': shards,
      'threshold_m': thresholdM,
    });
  }

  Map<String, dynamic> getLicenseStatus() {
    return _bindings.dispatch('get_license_status', {});
  }

  Map<String, dynamic> applyLicenseKey(String licenseKey) {
    return _bindings.dispatch('apply_license_key', {'license_key': licenseKey});
  }
}
