import 'dart:ffi';
import 'dart:io';
import 'dart:convert';
import 'package:ffi/ffi.dart';

typedef _c_press_dispatch = Pointer<Utf8> Function(
    Pointer<Utf8> cmd, Pointer<Utf8> payload);
typedef _dart_press_dispatch = Pointer<Utf8> Function(
    Pointer<Utf8> cmd, Pointer<Utf8> payload);

typedef _c_press_free = Void Function(Pointer<Utf8> ptr);
typedef _dart_press_free = void Function(Pointer<Utf8> ptr);

class PublishingPressFfiBindings {
  DynamicLibrary? _lib;
  _dart_press_dispatch? _dispatch;
  _dart_press_free? _freeString;
  bool _isAvailable = false;
  String? _initError;

  bool get isAvailable => _isAvailable;
  String? get initError => _initError;

  PublishingPressFfiBindings() {
    try {
      _lib = _loadLibrary();
      _dispatch = _lib!.lookupFunction<_c_press_dispatch, _dart_press_dispatch>(
          'markdown_publishing_press_ffi_dispatch_json');
      _freeString = _lib!.lookupFunction<_c_press_free, _dart_press_free>(
          'markdown_publishing_press_free_string');
      _isAvailable = true;
    } catch (e) {
      _isAvailable = false;
      _initError = e.toString();
    }
  }

  static DynamicLibrary _loadLibrary() {
    if (Platform.isAndroid) {
      return DynamicLibrary.open('libmarkdown_publishing_press.so');
    } else if (Platform.isIOS) {
      return DynamicLibrary.process();
    } else if (Platform.isWindows) {
      try {
        return DynamicLibrary.open('markdown_publishing_press.dll');
      } catch (_) {
        try {
          return DynamicLibrary.open('target/release/markdown_publishing_press.dll');
        } catch (_) {
          return DynamicLibrary.open('target/debug/markdown_publishing_press.dll');
        }
      }
    } else if (Platform.isLinux) {
      return DynamicLibrary.open('libmarkdown_publishing_press.so');
    } else if (Platform.isMacOS) {
      return DynamicLibrary.open('libmarkdown_publishing_press.dylib');
    }
    throw UnsupportedError('Unsupported platform: ${Platform.operatingSystem}');
  }

  Map<String, dynamic> dispatch(String command, Map<String, dynamic> payload) {
    if (!_isAvailable || _dispatch == null || _freeString == null) {
      return {
        'error': 'Native publishing press library not loaded: $_initError',
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
