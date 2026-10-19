/// Low-level C-ABI FFI bindings for DevOps Secret Vault
///
/// Communicates directly with `vault_core` compiled Rust library.
/// Manages memory safety, raw pointer lifecycles, and UTF-8 string deallocations.

import 'dart:ffi';
import 'dart:io';
import 'package:ffi/ffi.dart';

// Opaque struct representing Rust VaultHandle
final class VaultHandleOpaque extends Opaque {}

// C Native function typedefs
typedef _c_vault_create_handle = Pointer<VaultHandleOpaque> Function(
    Pointer<Utf8> dbPath);
typedef _dart_vault_create_handle = Pointer<VaultHandleOpaque> Function(
    Pointer<Utf8> dbPath);

typedef _c_vault_free_handle = Void Function(Pointer<VaultHandleOpaque> handle);
typedef _dart_vault_free_handle = void Function(
    Pointer<VaultHandleOpaque> handle);

typedef _c_vault_is_initialized = Int32 Function(
    Pointer<VaultHandleOpaque> handle);
typedef _dart_vault_is_initialized = int Function(
    Pointer<VaultHandleOpaque> handle);

typedef _c_vault_initialize = Int32 Function(
    Pointer<VaultHandleOpaque> handle, Pointer<Utf8> masterPassword);
typedef _dart_vault_initialize = int Function(
    Pointer<VaultHandleOpaque> handle, Pointer<Utf8> masterPassword);

typedef _c_vault_unlock = Int32 Function(
    Pointer<VaultHandleOpaque> handle, Pointer<Utf8> masterPassword);
typedef _dart_vault_unlock = int Function(
    Pointer<VaultHandleOpaque> handle, Pointer<Utf8> masterPassword);

typedef _c_vault_lock = Int32 Function(Pointer<VaultHandleOpaque> handle);
typedef _dart_vault_lock = int Function(Pointer<VaultHandleOpaque> handle);

typedef _c_vault_is_locked = Int32 Function(Pointer<VaultHandleOpaque> handle);
typedef _dart_vault_is_locked = int Function(Pointer<VaultHandleOpaque> handle);

typedef _c_vault_search = Pointer<Utf8> Function(
    Pointer<VaultHandleOpaque> handle, Pointer<Utf8> query, IntPtr limit);
typedef _dart_vault_search = Pointer<Utf8> Function(
    Pointer<VaultHandleOpaque> handle, Pointer<Utf8> query, int limit);

typedef _c_vault_create_secret = Pointer<Utf8> Function(
    Pointer<VaultHandleOpaque> handle, Pointer<Utf8> inputJson);
typedef _dart_vault_create_secret = Pointer<Utf8> Function(
    Pointer<VaultHandleOpaque> handle, Pointer<Utf8> inputJson);

typedef _c_vault_get_secret = Pointer<Utf8> Function(
    Pointer<VaultHandleOpaque> handle, Pointer<Utf8> id);
typedef _dart_vault_get_secret = Pointer<Utf8> Function(
    Pointer<VaultHandleOpaque> handle, Pointer<Utf8> id);

typedef _c_vault_generate_totp = Pointer<Utf8> Function(
    Pointer<VaultHandleOpaque> handle, Pointer<Utf8> secretId);
typedef _dart_vault_generate_totp = Pointer<Utf8> Function(
    Pointer<VaultHandleOpaque> handle, Pointer<Utf8> secretId);

typedef _c_vault_get_health_report = Pointer<Utf8> Function(
    Pointer<VaultHandleOpaque> handle);
typedef _dart_vault_get_health_report = Pointer<Utf8> Function(
    Pointer<VaultHandleOpaque> handle);

typedef _c_vault_inspect_certificate = Pointer<Utf8> Function(
    Pointer<VaultHandleOpaque> handle, Pointer<Utf8> certPem);
typedef _dart_vault_inspect_certificate = Pointer<Utf8> Function(
    Pointer<VaultHandleOpaque> handle, Pointer<Utf8> certPem);

typedef _c_vault_export_backup = Pointer<Utf8> Function(
    Pointer<VaultHandleOpaque> handle,
    Pointer<Utf8> backupPassword,
    Pointer<Utf8> outputPath);
typedef _dart_vault_export_backup = Pointer<Utf8> Function(
    Pointer<VaultHandleOpaque> handle,
    Pointer<Utf8> backupPassword,
    Pointer<Utf8> outputPath);

typedef _c_vault_import_backup = Pointer<Utf8> Function(
    Pointer<VaultHandleOpaque> handle,
    Pointer<Utf8> backupPassword,
    Pointer<Utf8> inputPath,
    Int32 overwrite);
typedef _dart_vault_import_backup = Pointer<Utf8> Function(
    Pointer<VaultHandleOpaque> handle,
    Pointer<Utf8> backupPassword,
    Pointer<Utf8> inputPath,
    int overwrite);

typedef _c_vault_free_string = Void Function(Pointer<Utf8> s);
typedef _dart_vault_free_string = void Function(Pointer<Utf8> s);

/// Dynamic loader and FFI interface to Rust `vault_core`
class VaultFfiBindings {
  static VaultFfiBindings? _instance;
  late DynamicLibrary _lib;

  late _dart_vault_create_handle createHandle;
  late _dart_vault_free_handle freeHandle;
  late _dart_vault_is_initialized isInitialized;
  late _dart_vault_initialize initialize;
  late _dart_vault_unlock unlock;
  late _dart_vault_lock lock;
  late _dart_vault_is_locked isLocked;
  late _dart_vault_search search;
  late _dart_vault_create_secret createSecret;
  late _dart_vault_get_secret getSecret;
  late _dart_vault_generate_totp generateTotp;
  late _dart_vault_get_health_report getHealthReport;
  late _dart_vault_inspect_certificate inspectCertificate;
  late _dart_vault_export_backup exportBackup;
  late _dart_vault_import_backup importBackup;
  late _dart_vault_free_string freeString;

  VaultFfiBindings._(DynamicLibrary lib) {
    _lib = lib;
    createHandle = _lib.lookupFunction<_c_vault_create_handle,
        _dart_vault_create_handle>('vault_create_handle');
    freeHandle = _lib.lookupFunction<_c_vault_free_handle,
        _dart_vault_free_handle>('vault_free_handle');
    isInitialized = _lib.lookupFunction<_c_vault_is_initialized,
        _dart_vault_is_initialized>('vault_is_initialized');
    initialize = _lib.lookupFunction<_c_vault_initialize,
        _dart_vault_initialize>('vault_initialize');
    unlock = _lib
        .lookupFunction<_c_vault_unlock, _dart_vault_unlock>('vault_unlock');
    lock = _lib.lookupFunction<_c_vault_lock, _dart_vault_lock>('vault_lock');
    isLocked = _lib
        .lookupFunction<_c_vault_is_locked, _dart_vault_is_locked>('vault_is_locked');
    search = _lib
        .lookupFunction<_c_vault_search, _dart_vault_search>('vault_search');
    createSecret = _lib.lookupFunction<_c_vault_create_secret,
        _dart_vault_create_secret>('vault_create_secret');
    getSecret = _lib.lookupFunction<_c_vault_get_secret,
        _dart_vault_get_secret>('vault_get_secret');
    generateTotp = _lib.lookupFunction<_c_vault_generate_totp,
        _dart_vault_generate_totp>('vault_generate_totp');
    getHealthReport = _lib.lookupFunction<_c_vault_get_health_report,
        _dart_vault_get_health_report>('vault_get_health_report');
    inspectCertificate = _lib.lookupFunction<_c_vault_inspect_certificate,
        _dart_vault_inspect_certificate>('vault_inspect_certificate');
    exportBackup = _lib.lookupFunction<_c_vault_export_backup,
        _dart_vault_export_backup>('vault_export_backup');
    importBackup = _lib.lookupFunction<_c_vault_import_backup,
        _dart_vault_import_backup>('vault_import_backup');
    freeString = _lib.lookupFunction<_c_vault_free_string,
        _dart_vault_free_string>('vault_free_string');
  }

  /// Factory resolving OS-specific dynamic library binary path
  static VaultFfiBindings load([String? customPath]) {
    if (_instance != null) return _instance!;

    DynamicLibrary library;
    if (customPath != null) {
      library = DynamicLibrary.open(customPath);
    } else if (Platform.isAndroid) {
      library = DynamicLibrary.open('libvault_core.so');
    } else if (Platform.isIOS || Platform.isMacOS) {
      library = DynamicLibrary.process();
    } else if (Platform.isWindows) {
      // Look in current or target build directory
      final candidates = [
        'vault_core.dll',
        'target/debug/vault_core.dll',
        'target/release/vault_core.dll',
      ];
      DynamicLibrary? loaded;
      for (final p in candidates) {
        if (File(p).existsSync()) {
          loaded = DynamicLibrary.open(p);
          break;
        }
      }
      library = loaded ?? DynamicLibrary.open('vault_core.dll');
    } else if (Platform.isLinux) {
      library = DynamicLibrary.open('libvault_core.so');
    } else {
      throw UnsupportedError('Platform not supported for vault_core FFI');
    }

    _instance = VaultFfiBindings._(library);
    return _instance!;
  }

  /// Helper to safely read a C-string, convert to Dart string, and release Rust heap memory
  String? readAndFreeString(Pointer<Utf8> ptr) {
    if (ptr == nullptr) return null;
    try {
      final dartStr = ptr.toDartString();
      return dartStr;
    } finally {
      freeString(ptr);
    }
  }
}
