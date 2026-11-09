import 'dart:ffi';
import 'dart:io';
import 'dart:convert';
import 'package:ffi/ffi.dart';

typedef _c_sql_dispatch = Pointer<Utf8> Function(
    Pointer<Utf8> cmd, Pointer<Utf8> payload);
typedef _dart_sql_dispatch = Pointer<Utf8> Function(
    Pointer<Utf8> cmd, Pointer<Utf8> payload);

typedef _c_sql_free = Void Function(Pointer<Utf8> ptr);
typedef _dart_sql_free = void Function(Pointer<Utf8> ptr);

class SqlMigrationDiffStudioFfiBindings {
  DynamicLibrary? _lib;
  _dart_sql_dispatch? _dispatch;
  _dart_sql_free? _freeString;
  bool _isAvailable = false;
  String? _initError;

  bool get isAvailable => _isAvailable;
  String? get initError => _initError;

  SqlMigrationDiffStudioFfiBindings() {
    try {
      _lib = _loadLibrary();
      _dispatch = _lib!.lookupFunction<_c_sql_dispatch, _dart_sql_dispatch>(
          'sql_migration_diff_studio_ffi_dispatch_json');
      _freeString = _lib!.lookupFunction<_c_sql_free, _dart_sql_free>(
          'sql_migration_diff_studio_free_string');
      _isAvailable = true;
    } catch (e) {
      _isAvailable = false;
      _initError = e.toString();
    }
  }

  static DynamicLibrary _loadLibrary() {
    if (Platform.isAndroid) {
      return DynamicLibrary.open('libsql_migration_diff_studio.so');
    } else if (Platform.isIOS) {
      return DynamicLibrary.process();
    } else if (Platform.isWindows) {
      try {
        return DynamicLibrary.open('sql_migration_diff_studio.dll');
      } catch (_) {
        try {
          return DynamicLibrary.open('target/release/sql_migration_diff_studio.dll');
        } catch (_) {
          return DynamicLibrary.open('target/debug/sql_migration_diff_studio.dll');
        }
      }
    } else if (Platform.isMacOS) {
      return DynamicLibrary.open('libsql_migration_diff_studio.dylib');
    } else {
      return DynamicLibrary.open('libsql_migration_diff_studio.so');
    }
  }

  Map<String, dynamic> dispatch(String command, Map<String, dynamic> payload) {
    if (!_isAvailable || _dispatch == null || _freeString == null) {
      return {
        'error': true,
        'message': 'SQL Migration Diff Studio FFI bindings not initialized: $_initError',
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
