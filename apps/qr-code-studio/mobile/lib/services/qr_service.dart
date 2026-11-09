import '../bindings/qr_ffi.dart';

class QrStudioService {
  final QrStudioFfiBindings _bindings;

  QrStudioService({QrStudioFfiBindings? bindings})
      : _bindings = bindings ?? QrStudioFfiBindings();

  bool get isAvailable => _bindings.isAvailable;
  String? get initError => _bindings.initError;

  Map<String, dynamic> generateQr({
    required String rawText,
    String? label,
    String eccLevel = 'Medium',
    String moduleShape = 'Rounded',
    String finderStyle = 'Rounded',
    String fgColor = '#06b6d4',
    String bgColor = '#0a0e17',
    int sizePx = 512,
    String? passphrase,
    bool signEd25519 = false,
  }) {
    return _bindings.dispatch('generate_qr', {
      'raw_text': rawText,
      if (label != null && label.isNotEmpty) 'label': label,
      'style': {
        'ecc_level': eccLevel,
        'module_shape': moduleShape,
        'finder_style': finderStyle,
        'fg_color': fgColor,
        'fg_gradient_end': null,
        'bg_color': bgColor,
        'quiet_zone_modules': 4,
        'size_px': sizePx,
        'center_logo_text': null,
      },
      if (passphrase != null && passphrase.isNotEmpty) 'passphrase': passphrase,
      'sign_ed25519': signEd25519,
    });
  }

  Map<String, dynamic> scanQrImage({
    String? pngBase64,
    String? filePath,
    String? passphrase,
  }) {
    return _bindings.dispatch('scan_qr_image', {
      if (pngBase64 != null) 'png_base64': pngBase64,
      if (filePath != null) 'file_path': filePath,
      if (passphrase != null) 'passphrase': passphrase,
    });
  }

  Map<String, dynamic> analyzeSecurity(String rawPayload) {
    return _bindings.dispatch('analyze_payload_security', {
      'payload': rawPayload,
    });
  }

  Map<String, dynamic> listHistory({
    String? searchQuery,
    bool favoritesOnly = false,
    int limit = 50,
  }) {
    return _bindings.dispatch('list_history', {
      if (searchQuery != null && searchQuery.isNotEmpty) 'search_query': searchQuery,
      'favorites_only': favoritesOnly,
      'limit': limit,
    });
  }

  Map<String, dynamic> getStudioStats() {
    return _bindings.dispatch('get_studio_stats', {});
  }
}
