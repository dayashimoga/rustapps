import 'dart:ffi';
import 'dart:io';
import 'dart:convert';
import 'package:ffi/ffi.dart';

typedef _c_synth_dispatch = Pointer<Utf8> Function(
    Pointer<Utf8> cmd, Pointer<Utf8> payload);
typedef _dart_synth_dispatch = Pointer<Utf8> Function(
    Pointer<Utf8> cmd, Pointer<Utf8> payload);

typedef _c_synth_free = Void Function(Pointer<Utf8> ptr);
typedef _dart_synth_free = void Function(Pointer<Utf8> ptr);

class LocalDataSynthesizerFfiBindings {
  DynamicLibrary? _lib;
  _dart_synth_dispatch? _dispatch;
  _dart_synth_free? _freeString;
  bool _isAvailable = false;
  String? _initError;

  bool get isAvailable => _isAvailable;
  String? get initError => _initError;

  LocalDataSynthesizerFfiBindings() {
    try {
      _lib = _loadLibrary();
      _dispatch = _lib!.lookupFunction<_c_synth_dispatch, _dart_synth_dispatch>(
          'local_data_synthesizer_ffi_dispatch_json');
      _freeString = _lib!.lookupFunction<_c_synth_free, _dart_synth_free>(
          'local_data_synthesizer_free_string');
      _isAvailable = true;
    } catch (e) {
      _isAvailable = false;
      _initError = e.toString();
    }
  }

  static DynamicLibrary _loadLibrary() {
    if (Platform.isAndroid) {
      return DynamicLibrary.open('liblocal_data_synthesizer.so');
    } else if (Platform.isIOS) {
      return DynamicLibrary.process();
    } else if (Platform.isWindows) {
      try {
        return DynamicLibrary.open('local_data_synthesizer.dll');
      } catch (_) {
        try {
          return DynamicLibrary.open('target/release/local_data_synthesizer.dll');
        } catch (_) {
          return DynamicLibrary.open('target/debug/local_data_synthesizer.dll');
        }
      }
    } else if (Platform.isMacOS) {
      return DynamicLibrary.open('liblocal_data_synthesizer.dylib');
    } else {
      return DynamicLibrary.open('liblocal_data_synthesizer.so');
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
