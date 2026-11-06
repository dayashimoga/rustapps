import 'dart:ffi';
import 'dart:io';
import 'dart:convert';
import 'package:ffi/ffi.dart';

typedef _c_qr_dispatch = Pointer<Utf8> Function(
    Pointer<Utf8> cmd, Pointer<Utf8> payload);
typedef _dart_qr_dispatch = Pointer<Utf8> Function(
    Pointer<Utf8> cmd, Pointer<Utf8> payload);

typedef _c_qr_free = Void Function(Pointer<Utf8> ptr);
typedef _dart_qr_free = void Function(Pointer<Utf8> ptr);

class QrStudioFfiBindings {
  late final DynamicLibrary _lib;
  late final _dart_qr_dispatch _dispatch;
  late final _dart_qr_free _freeString;

  QrStudioFfiBindings() {
    _lib = _loadLibrary();
    _dispatch = _lib
        .lookupFunction<_c_qr_dispatch, _dart_qr_dispatch>(
            'qr_code_studio_ffi_dispatch_json');
    _freeString = _lib.lookupFunction<_c_qr_free, _dart_qr_free>(
        'qr_code_studio_free_string');
  }

  static DynamicLibrary _loadLibrary() {
    if (Platform.isAndroid) {
      return DynamicLibrary.open('libqr_code_studio.so');
    } else if (Platform.isIOS) {
      return DynamicLibrary.process();
    } else if (Platform.isWindows) {
      return DynamicLibrary.open('qr_code_studio.dll');
    } else if (Platform.isMacOS) {
      return DynamicLibrary.open('libqr_code_studio.dylib');
    } else {
      return DynamicLibrary.open('libqr_code_studio.so');
    }
  }

  Map<String, dynamic> dispatch(String command, Map<String, dynamic> payload) {
    final cmdPtr = command.toNativeUtf8();
    final payloadJson = jsonEncode(payload);
    final payloadPtr = payloadJson.toNativeUtf8();

    try {
      final resPtr = _dispatch(cmdPtr, payloadPtr);
      if (resPtr == nullptr) {
        return {'success': false, 'error': 'Null pointer returned from FFI'};
      }
      try {
        final resStr = resPtr.toDartString();
        return jsonDecode(resStr) as Map<String, dynamic>;
      } finally {
        _freeString(resPtr);
      }
    } finally {
      calloc.free(cmdPtr);
      calloc.free(payloadPtr);
    }
  }
}
