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
  DynamicLibrary? _lib;
  _dart_qr_dispatch? _dispatch;
  _dart_qr_free? _freeString;
  bool _isAvailable = false;
  String? _initError;

  bool get isAvailable => _isAvailable;
  String? get initError => _initError;

  QrStudioFfiBindings() {
    try {
      _lib = _loadLibrary();
      _dispatch = _lib!.lookupFunction<_c_qr_dispatch, _dart_qr_dispatch>(
          'qr_code_studio_ffi_dispatch_json');
      _freeString = _lib!.lookupFunction<_c_qr_free, _dart_qr_free>(
          'qr_code_studio_free_string');
      _isAvailable = true;
    } catch (e) {
      _isAvailable = false;
      _initError = e.toString();
    }
  }

  static DynamicLibrary _loadLibrary() {
    if (Platform.isAndroid) {
      return DynamicLibrary.open('libqr_code_studio.so');
    } else if (Platform.isIOS) {
      return DynamicLibrary.process();
    } else if (Platform.isWindows) {
      try {
        return DynamicLibrary.open('qr_code_studio.dll');
      } catch (_) {
        try {
          return DynamicLibrary.open('target/release/qr_code_studio.dll');
        } catch (_) {
          return DynamicLibrary.open('target/debug/qr_code_studio.dll');
        }
      }
    } else if (Platform.isMacOS) {
      return DynamicLibrary.open('libqr_code_studio.dylib');
    } else {
      return DynamicLibrary.open('libqr_code_studio.so');
    }
  }

  Map<String, dynamic> dispatch(String command, Map<String, dynamic> payload) {
    if (!_isAvailable || _dispatch == null || _freeString == null) {
      return {
        'error': 'Native QR Studio FFI library unavailable: ${_initError ?? "unknown"}'
      };
    }

    final cmdPtr = command.toNativeUtf8();
    final payloadJson = jsonEncode(payload);
    final payloadPtr = payloadJson.toNativeUtf8();

    try {
      final resPtr = _dispatch!(cmdPtr, payloadPtr);
      if (resPtr == nullptr) {
        return {'error': 'Null pointer returned from native FFI boundary'};
      }
      try {
        final resStr = resPtr.toDartString();
        final decoded = jsonDecode(resStr);
        if (decoded is Map<String, dynamic>) {
          return decoded;
        } else if (decoded is List) {
          return {'items': decoded, 'success': true};
        } else {
          return {'value': decoded, 'success': true};
        }
      } finally {
        _freeString!(resPtr);
      }
    } catch (e) {
      return {'error': 'FFI dispatch failure: $e'};
    } finally {
      calloc.free(cmdPtr);
      calloc.free(payloadPtr);
    }
  }
}
