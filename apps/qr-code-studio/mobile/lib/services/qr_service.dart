import '../bindings/qr_ffi.dart';

class QrStudioService {
  final QrStudioFfiBindings _bindings;

  QrStudioService({QrStudioFfiBindings? bindings})
      : _bindings = bindings ?? QrStudioFfiBindings();

  Map<String, dynamic> generateQr({
    required String payload,
    String eccLevel = 'M',
    String shape = 'Rounded',
    String finderPattern = 'Rounded',
    String foreground = '#06b6d4',
    String background = '#0a0e17',
  }) {
    return _bindings.dispatch('generate_qr', {
      'payload': payload,
      'config': {
        'ecc_level': eccLevel,
        'module_shape': shape,
        'finder_pattern': finderPattern,
        'foreground_color': foreground,
        'background_color': background,
      }
    });
  }

  Map<String, dynamic> scanQrImage(String base64Image) {
    return _bindings.dispatch('scan_qr_image', {
      'image_base64': base64Image,
    });
  }

  Map<String, dynamic> analyzeSecurity(String rawPayload) {
    return _bindings.dispatch('analyze_security', {
      'raw_payload': rawPayload,
    });
  }

  Map<String, dynamic> getHistory({int page = 1, int pageSize = 20}) {
    return _bindings.dispatch('get_history', {
      'page': page,
      'page_size': pageSize,
    });
  }
}

