import 'dart:ffi';
import 'dart:io';
import 'dart:convert';
import 'package:ffi/ffi.dart';

typedef _c_launcher_dispatch = Pointer<Utf8> Function(
    Pointer<Utf8> cmd, Pointer<Utf8> payload);
typedef _dart_launcher_dispatch = Pointer<Utf8> Function(
    Pointer<Utf8> cmd, Pointer<Utf8> payload);

typedef _c_launcher_free = Void Function(Pointer<Utf8> ptr);
typedef _dart_launcher_free = void Function(Pointer<Utf8> ptr);

class SuiteLauncherFfiBindings {
  DynamicLibrary? _lib;
  _dart_launcher_dispatch? _dispatch;
  _dart_launcher_free? _freeString;
  bool _isAvailable = false;
  String? _initError;

  bool get isAvailable => _isAvailable;
  String? get initError => _initError;

  SuiteLauncherFfiBindings() {
    try {
      _lib = _loadLibrary();
      _dispatch = _lib!.lookupFunction<_c_launcher_dispatch, _dart_launcher_dispatch>(
          'antigravity_suite_launcher_ffi_dispatch_json');
      _freeString = _lib!.lookupFunction<_c_launcher_free, _dart_launcher_free>(
          'antigravity_suite_launcher_free_string');
      _isAvailable = true;
    } catch (e) {
      _isAvailable = false;
      _initError = e.toString();
    }
  }

  static DynamicLibrary _loadLibrary() {
    if (Platform.isAndroid) {
      return DynamicLibrary.open('libantigravity_suite_launcher.so');
    } else if (Platform.isIOS) {
      return DynamicLibrary.process();
    } else if (Platform.isWindows) {
      try {
        return DynamicLibrary.open('antigravity_suite_launcher.dll');
      } catch (_) {
        try {
          return DynamicLibrary.open('target/release/antigravity_suite_launcher.dll');
        } catch (_) {
          return DynamicLibrary.open('target/debug/antigravity_suite_launcher.dll');
        }
      }
    } else if (Platform.isLinux) {
      return DynamicLibrary.open('libantigravity_suite_launcher.so');
    } else if (Platform.isMacOS) {
      return DynamicLibrary.open('libantigravity_suite_launcher.dylib');
    }
    throw UnsupportedError('Unsupported platform: ${Platform.operatingSystem}');
  }

  Map<String, dynamic> dispatch(String command, Map<String, dynamic> payload) {
    if (!_isAvailable || _dispatch == null || _freeString == null) {
      return {
        'error': 'Native suite launcher library not loaded: $_initError',
        'success': false,
      };
    }

    final cmdPtr = command.toNativeUtf8();
    final payloadJson = jsonEncode(payload);
    final payloadPtr = payloadJson.toNativeUtf8();

    try {
      final resultPtr = _dispatch!(cmdPtr, payloadPtr);
      if (resultPtr.address == 0) {
        return {'error': 'Null pointer returned from native FFI boundary'};
      }
      final resultStr = resultPtr.toDartString();
      _freeString!(resultPtr);

      return jsonDecode(resultStr) as Map<String, dynamic>;
    } catch (e) {
      return {'error': 'FFI dispatch error: $e'};
    } finally {
      calloc.free(cmdPtr);
      calloc.free(payloadPtr);
    }
  }
}
