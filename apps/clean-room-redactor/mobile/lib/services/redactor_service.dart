import '../bindings/redactor_ffi.dart';

class CleanRoomRedactorService {
  final CleanRoomRedactorFfiBindings _bindings;

  CleanRoomRedactorService({CleanRoomRedactorFfiBindings? bindings})
      : _bindings = bindings ?? CleanRoomRedactorFfiBindings();

  bool get isAvailable => _bindings.isAvailable;
  String? get initError => _bindings.initError;

  Map<String, dynamic> getStats() {
    return _bindings.dispatch('get_stats', {});
  }

  Map<String, dynamic> getDocument({String documentId = 'doc-demo-settlement'}) {
    return _bindings.dispatch('get_document', {'document_id': documentId});
  }

  Map<String, dynamic> loadDocument({
    required String filename,
    required String contentText,
  }) {
    return _bindings.dispatch('load_document', {
      'filename': filename,
      'content_text': contentText,
    });
  }

  Map<String, dynamic> addRedaction({
    String documentId = 'doc-demo-settlement',
    int pageIndex = 0,
    required String label,
    String burnStyle = 'solid_black',
    String? exemptionCode,
    double x = 45.0,
    double y = 80.0,
    double width = 180.0,
    double height = 22.0,
  }) {
    return _bindings.dispatch('add_redaction', {
      'document_id': documentId,
      'page_index': pageIndex,
      'label': label,
      'burn_style': burnStyle,
      if (exemptionCode != null) 'exemption_code': exemptionCode,
      'bbox': {
        'x': x,
        'y': y,
        'width': width,
        'height': height,
      },
    });
  }

  Map<String, dynamic> removeRedaction({
    String documentId = 'doc-demo-settlement',
    int pageIndex = 0,
    required String maskId,
  }) {
    return _bindings.dispatch('remove_redaction', {
      'document_id': documentId,
      'page_index': pageIndex,
      'mask_id': maskId,
    });
  }

  Map<String, dynamic> togglePiiApproval({
    String documentId = 'doc-demo-settlement',
    int pageIndex = 0,
    required String findingId,
    required bool approved,
  }) {
    return _bindings.dispatch('toggle_pii_approval', {
      'document_id': documentId,
      'page_index': pageIndex,
      'finding_id': findingId,
      'approved': approved,
    });
  }

  Map<String, dynamic> applyCleanRoomRedaction({
    String documentId = 'doc-demo-settlement',
    String batesPrefix = 'LIT-PROD-',
    int batesStart = 1,
    int batesDigits = 6,
    String batesPosition = 'bottom_right',
  }) {
    return _bindings.dispatch('apply_clean_room_redaction', {
      'document_id': documentId,
      'bates_config': {
        'prefix': batesPrefix,
        'suffix': '',
        'start_number': batesStart,
        'digits': batesDigits,
        'position': batesPosition,
        'font_size': 10,
        'include_date': false,
      },
    });
  }

  Map<String, dynamic> getLicenseStatus() {
    return _bindings.dispatch('get_license_status', {});
  }
}
