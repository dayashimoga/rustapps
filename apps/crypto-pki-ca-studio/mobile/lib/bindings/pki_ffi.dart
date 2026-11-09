import 'dart:ffi';
import 'dart:io';
import 'dart:convert';
import 'package:ffi/ffi.dart';

typedef _c_pki_dispatch = Pointer<Utf8> Function(
    Pointer<Utf8> cmd, Pointer<Utf8> payload);
typedef _dart_pki_dispatch = Pointer<Utf8> Function(
    Pointer<Utf8> cmd, Pointer<Utf8> payload);

typedef _c_pki_free = Void Function(Pointer<Utf8> ptr);
typedef _dart_pki_free = void Function(Pointer<Utf8> ptr);

class CryptoPkiCaStudioFfiBindings {
  DynamicLibrary? _lib;
  _dart_pki_dispatch? _dispatch;
  _dart_pki_free? _freeString;
  bool _isAvailable = false;
  String? _initError;

  bool get isAvailable => _isAvailable;
  String? get initError => _initError;

  CryptoPkiCaStudioFfiBindings() {
    try {
      _lib = _loadLibrary();
      _dispatch = _lib!.lookupFunction<_c_pki_dispatch, _dart_pki_dispatch>(
          'crypto_pki_ca_studio_ffi_dispatch_json');
      _freeString = _lib!.lookupFunction<_c_pki_free, _dart_pki_free>(
          'crypto_pki_ca_studio_free_string');
      _isAvailable = true;
    } catch (e) {
      _isAvailable = false;
      _initError = e.toString();
    }
  }

  static DynamicLibrary _loadLibrary() {
    if (Platform.isAndroid) {
      return DynamicLibrary.open('libcrypto_pki_ca_studio.so');
    } else if (Platform.isIOS) {
      return DynamicLibrary.process();
    } else if (Platform.isWindows) {
      try {
        return DynamicLibrary.open('crypto_pki_ca_studio.dll');
      } catch (_) {
        try {
          return DynamicLibrary.open('target/release/crypto_pki_ca_studio.dll');
        } catch (_) {
          return DynamicLibrary.open('target/debug/crypto_pki_ca_studio.dll');
        }
      }
    } else if (Platform.isMacOS) {
      return DynamicLibrary.open('libcrypto_pki_ca_studio.dylib');
    } else {
      return DynamicLibrary.open('libcrypto_pki_ca_studio.so');
    }
  }

  Map<String, dynamic> dispatch(String command, Map<String, dynamic> payload) {
    if (!_isAvailable || _dispatch == null) {
      return {'error': true, 'message': 'FFI library not initialized: $_initError'};
    }

    final cmdPtr = command.toNativeUtf8();
    final payloadJson = jsonEncode(payload);
    final payloadPtr = payloadJson.toNativeUtf8();

    try {
      final resPtr = _dispatch!(cmdPtr, payloadPtr);
      if (resPtr == nullptr) {
        return {'error': true, 'message': 'Null response from native FFI'};
      }

      final resStr = resPtr.toDartString();
      if (_freeString != null) {
        _freeString!(resPtr);
      }

      return jsonDecode(resStr) as Map<String, dynamic>;
    } catch (e) {
      return {'error': true, 'message': e.toString()};
    } finally {
      calloc.free(cmdPtr);
      calloc.free(payloadPtr);
    }
  }
}
