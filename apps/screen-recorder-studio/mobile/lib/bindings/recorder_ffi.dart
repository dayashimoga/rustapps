import 'dart:ffi';
import 'dart:io';
import 'dart:convert';
import 'package:ffi/ffi.dart';

typedef _c_recorder_dispatch = Pointer<Utf8> Function(
    Pointer<Utf8> cmd, Pointer<Utf8> payload);
typedef _dart_recorder_dispatch = Pointer<Utf8> Function(
    Pointer<Utf8> cmd, Pointer<Utf8> payload);

typedef _c_recorder_free = Void Function(Pointer<Utf8> ptr);
typedef _dart_recorder_free = void Function(Pointer<Utf8> ptr);

class RecorderFfiBindings {
  DynamicLibrary? _lib;
  _dart_recorder_dispatch? _dispatch;
  _dart_recorder_free? _freeString;
  bool _isAvailable = false;
  String? _initError;

  bool get isAvailable => _isAvailable;
  String? get initError => _initError;

  RecorderFfiBindings() {
    try {
      _lib = _loadLibrary();
      _dispatch = _lib!.lookupFunction<_c_recorder_dispatch, _dart_recorder_dispatch>(
          'recorder_dispatch_command');
      _freeString = _lib!.lookupFunction<_c_recorder_free, _dart_recorder_free>(
          'recorder_free_string');
      _isAvailable = true;
    } catch (e) {
      _isAvailable = false;
      _initError = e.toString();
    }
  }

  static DynamicLibrary _loadLibrary() {
    if (Platform.isAndroid) {
      return DynamicLibrary.open('libscreen_recorder_studio.so');
    } else if (Platform.isIOS) {
      return DynamicLibrary.process();
    } else if (Platform.isWindows) {
      try {
        return DynamicLibrary.open('screen_recorder_studio.dll');
      } catch (_) {
        try {
          return DynamicLibrary.open('target/release/screen_recorder_studio.dll');
        } catch (_) {
          return DynamicLibrary.open('target/debug/screen_recorder_studio.dll');
        }
      }
    } else if (Platform.isMacOS) {
      return DynamicLibrary.open('libscreen_recorder_studio.dylib');
    } else {
      return DynamicLibrary.open('libscreen_recorder_studio.so');
    }
  }

  dynamic dispatch(String command, Map<String, dynamic> payload) {
    if (!_isAvailable || _dispatch == null || _freeString == null) {
      return {
        'error': 'Native Screen Recorder FFI library unavailable: ${_initError ?? "unknown"}'
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

