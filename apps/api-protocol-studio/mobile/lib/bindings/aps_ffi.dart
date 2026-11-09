import 'dart:ffi';
import 'dart:io';
import 'dart:convert';
import 'package:ffi/ffi.dart';

typedef _c_aps_dispatch = Pointer<Utf8> Function(
    Pointer<Utf8> cmd, Pointer<Utf8> payload);
typedef _dart_aps_dispatch = Pointer<Utf8> Function(
    Pointer<Utf8> cmd, Pointer<Utf8> payload);

typedef _c_aps_free = Void Function(Pointer<Utf8> ptr);
typedef _dart_aps_free = void Function(Pointer<Utf8> ptr);

class ApsFfiBindings {
  DynamicLibrary? _lib;
  _dart_aps_dispatch? _dispatch;
  _dart_aps_free? _freeString;
  bool _isAvailable = false;
  String? _initError;

  bool get isAvailable => _isAvailable;
  String? get initError => _initError;

  ApsFfiBindings() {
    try {
      _lib = _loadLibrary();
      _dispatch = _lib!.lookupFunction<_c_aps_dispatch, _dart_aps_dispatch>(
          'aps_dispatch_command');
      _freeString = _lib!.lookupFunction<_c_aps_free, _dart_aps_free>(
          'aps_free_string');
      _isAvailable = true;
    } catch (e) {
      _isAvailable = false;
      _initError = e.toString();
    }
  }

  static DynamicLibrary _loadLibrary() {
    if (Platform.isAndroid) {
      return DynamicLibrary.open('libapi_protocol_studio.so');
    } else if (Platform.isIOS) {
      return DynamicLibrary.process();
    } else if (Platform.isWindows) {
      try {
        return DynamicLibrary.open('api_protocol_studio.dll');
      } catch (_) {
        try {
          return DynamicLibrary.open('target/release/api_protocol_studio.dll');
        } catch (_) {
          return DynamicLibrary.open('target/debug/api_protocol_studio.dll');
        }
      }
    } else if (Platform.isMacOS) {
      return DynamicLibrary.open('libapi_protocol_studio.dylib');
    } else {
      return DynamicLibrary.open('libapi_protocol_studio.so');
    }
  }

  dynamic dispatch(String command, Map<String, dynamic> payload) {
    if (!_isAvailable || _dispatch == null || _freeString == null) {
      return {
        'error': 'Native API Protocol Studio FFI library unavailable: ${_initError ?? "unknown"}'
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
        return jsonDecode(resStr);
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

