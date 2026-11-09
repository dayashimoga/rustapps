import 'dart:ffi';
import 'dart:io';
import 'dart:convert';
import 'package:ffi/ffi.dart';

typedef _c_contract_dispatch = Pointer<Utf8> Function(
    Pointer<Utf8> cmd, Pointer<Utf8> payload);
typedef _dart_contract_dispatch = Pointer<Utf8> Function(
    Pointer<Utf8> cmd, Pointer<Utf8> payload);

typedef _c_contract_free = Void Function(Pointer<Utf8> ptr);
typedef _dart_contract_free = void Function(Pointer<Utf8> ptr);

class LocalRagContractAuditorFfiBindings {
  DynamicLibrary? _lib;
  _dart_contract_dispatch? _dispatch;
  _dart_contract_free? _freeString;
  bool _isAvailable = false;
  String? _initError;

  bool get isAvailable => _isAvailable;
  String? get initError => _initError;

  LocalRagContractAuditorFfiBindings() {
    try {
      _lib = _loadLibrary();
      _dispatch = _lib!.lookupFunction<_c_contract_dispatch, _dart_contract_dispatch>(
          'local_rag_contract_auditor_ffi_dispatch_json');
      _freeString = _lib!.lookupFunction<_c_contract_free, _dart_contract_free>(
          'local_rag_contract_auditor_free_string');
      _isAvailable = true;
    } catch (e) {
      _isAvailable = false;
      _initError = e.toString();
    }
  }

  static DynamicLibrary _loadLibrary() {
    if (Platform.isAndroid) {
      return DynamicLibrary.open('liblocal_rag_contract_auditor.so');
    } else if (Platform.isIOS) {
      return DynamicLibrary.process();
    } else if (Platform.isWindows) {
      try {
        return DynamicLibrary.open('local_rag_contract_auditor.dll');
      } catch (_) {
        try {
          return DynamicLibrary.open('target/release/local_rag_contract_auditor.dll');
        } catch (_) {
          return DynamicLibrary.open('target/debug/local_rag_contract_auditor.dll');
        }
      }
    } else if (Platform.isMacOS) {
      return DynamicLibrary.open('liblocal_rag_contract_auditor.dylib');
    } else {
      return DynamicLibrary.open('liblocal_rag_contract_auditor.so');
    }
  }

  Map<String, dynamic> dispatch(String command, Map<String, dynamic> payload) {
    if (!_isAvailable || _dispatch == null || _freeString == null) {
      return {
        'error': true,
        'message': 'Local RAG Contract Auditor FFI bindings not initialized: $_initError',
      };
    }

    final cmdPtr = command.toNativeUtf8();
    final payloadJson = jsonEncode(payload);
    final payloadPtr = payloadJson.toNativeUtf8();

    Pointer<Utf8>? resPtr;
    try {
      resPtr = _dispatch!(cmdPtr, payloadPtr);
      if (resPtr.address == 0) {
        return {'error': true, 'message': 'Null pointer returned from native FFI'};
      }
      final jsonStr = resPtr.toDartString();
      return jsonDecode(jsonStr) as Map<String, dynamic>;
    } catch (e) {
      return {'error': true, 'message': 'FFI dispatch failure: $e'};
    } finally {
      calloc.free(cmdPtr);
      calloc.free(payloadPtr);
      if (resPtr != null && resPtr.address != 0) {
        _freeString!(resPtr);
      }
    }
  }
}
