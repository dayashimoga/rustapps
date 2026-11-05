/// Low-level C-ABI FFI bindings for Universal Log Viewer
///
/// Direct communication with `universal_log_viewer` compiled Rust library.
/// Manages memory safety, raw pointer lifecycles, and UTF-8 string allocations/frees.

import 'dart:ffi';
import 'dart:io';
import 'package:ffi/ffi.dart';

// Opaque struct representing Rust LogViewerHandle
final class UlvHandleOpaque extends Opaque {}

// C Native function typedefs
typedef _c_ulv_create_handle = Pointer<UlvHandleOpaque> Function();
typedef _dart_ulv_create_handle = Pointer<UlvHandleOpaque> Function();

typedef _c_ulv_free_handle = Void Function(Pointer<UlvHandleOpaque> handle);
typedef _dart_ulv_free_handle = void Function(Pointer<UlvHandleOpaque> handle);

typedef _c_ulv_open_file = Pointer<Utf8> Function(
    Pointer<UlvHandleOpaque> handle, Pointer<Utf8> path);
typedef _dart_ulv_open_file = Pointer<Utf8> Function(
    Pointer<UlvHandleOpaque> handle, Pointer<Utf8> path);

typedef _c_ulv_get_lines_slice = Pointer<Utf8> Function(
    Pointer<UlvHandleOpaque> handle, IntPtr startLine, IntPtr count);
typedef _dart_ulv_get_lines_slice = Pointer<Utf8> Function(
    Pointer<UlvHandleOpaque> handle, int startLine, int count);

typedef _c_ulv_search = Pointer<Utf8> Function(
    Pointer<UlvHandleOpaque> handle,
    Pointer<Utf8> queryPattern,
    Bool isRegex,
    Bool caseSensitive,
    IntPtr maxResults);
typedef _dart_ulv_search = Pointer<Utf8> Function(
    Pointer<UlvHandleOpaque> handle,
    Pointer<Utf8> queryPattern,
    bool isRegex,
    bool caseSensitive,
    int maxResults);

typedef _c_ulv_get_timeline_histogram = Pointer<Utf8> Function(
    Pointer<UlvHandleOpaque> handle, IntPtr targetBuckets);
typedef _dart_ulv_get_timeline_histogram = Pointer<Utf8> Function(
    Pointer<UlvHandleOpaque> handle, int targetBuckets);

typedef _c_ulv_mine_templates = Pointer<Utf8> Function(
    Pointer<UlvHandleOpaque> handle, IntPtr maxRecords);
typedef _dart_ulv_mine_templates = Pointer<Utf8> Function(
    Pointer<UlvHandleOpaque> handle, int maxRecords);

typedef _c_ulv_cluster_errors = Pointer<Utf8> Function(
    Pointer<UlvHandleOpaque> handle, IntPtr maxRecords);
typedef _dart_ulv_cluster_errors = Pointer<Utf8> Function(
    Pointer<UlvHandleOpaque> handle, int maxRecords);

typedef _c_ulv_free_string = Void Function(Pointer<Utf8> ptr);
typedef _dart_ulv_free_string = void Function(Pointer<Utf8> ptr);

typedef _c_ulv_dispatch_command = Pointer<Utf8> Function(
    Pointer<UlvHandleOpaque> handle, Pointer<Utf8> cmd, Pointer<Utf8> argsJson);
typedef _dart_ulv_dispatch_command = Pointer<Utf8> Function(
    Pointer<UlvHandleOpaque> handle, Pointer<Utf8> cmd, Pointer<Utf8> argsJson);

/// Dynamic loader and FFI interface to Rust `universal_log_viewer`
class UlvFfiBindings {
  static UlvFfiBindings? _instance;
  late DynamicLibrary _lib;

  late _dart_ulv_create_handle createHandle;
  late _dart_ulv_free_handle freeHandle;
  late _dart_ulv_open_file openFile;
  late _dart_ulv_get_lines_slice getLinesSlice;
  late _dart_ulv_search search;
  late _dart_ulv_get_timeline_histogram getTimelineHistogram;
  late _dart_ulv_mine_templates mineTemplates;
  late _dart_ulv_cluster_errors clusterErrors;
  late _dart_ulv_free_string freeString;
  late _dart_ulv_dispatch_command dispatchCommand;

  UlvFfiBindings._(DynamicLibrary lib) {
    _lib = lib;
    createHandle = _lib
        .lookupFunction<_c_ulv_create_handle, _dart_ulv_create_handle>('ulv_create_handle');
    freeHandle = _lib
        .lookupFunction<_c_ulv_free_handle, _dart_ulv_free_handle>('ulv_free_handle');
    openFile = _lib
        .lookupFunction<_c_ulv_open_file, _dart_ulv_open_file>('ulv_open_file');
    getLinesSlice = _lib.lookupFunction<_c_ulv_get_lines_slice,
        _dart_ulv_get_lines_slice>('ulv_get_lines_slice');
    search = _lib
        .lookupFunction<_c_ulv_search, _dart_ulv_search>('ulv_search');
    getTimelineHistogram = _lib.lookupFunction<
        _c_ulv_get_timeline_histogram,
        _dart_ulv_get_timeline_histogram>('ulv_get_timeline_histogram');
    mineTemplates = _lib.lookupFunction<_c_ulv_mine_templates,
        _dart_ulv_mine_templates>('ulv_mine_templates');
    clusterErrors = _lib.lookupFunction<_c_ulv_cluster_errors,
        _dart_ulv_cluster_errors>('ulv_cluster_errors');
    freeString = _lib
        .lookupFunction<_c_ulv_free_string, _dart_ulv_free_string>('ulv_free_string');
    dispatchCommand = _lib
        .lookupFunction<_c_ulv_dispatch_command, _dart_ulv_dispatch_command>('ulv_dispatch_command');
  }

  /// Factory resolving OS-specific dynamic library binary path
  static UlvFfiBindings load([String? customPath]) {
    if (_instance != null) return _instance!;

    DynamicLibrary library;
    if (customPath != null) {
      library = DynamicLibrary.open(customPath);
    } else if (Platform.isAndroid) {
      library = DynamicLibrary.open('libuniversal_log_viewer.so');
    } else if (Platform.isIOS || Platform.isMacOS) {
      library = DynamicLibrary.process();
    } else if (Platform.isWindows) {
      final candidates = [
        'universal_log_viewer.dll',
        'target/debug/universal_log_viewer.dll',
        'target/release/universal_log_viewer.dll',
        '../../target/debug/universal_log_viewer.dll',
        '../../target/release/universal_log_viewer.dll',
      ];
      DynamicLibrary? loaded;
      for (final p in candidates) {
        if (File(p).existsSync()) {
          loaded = DynamicLibrary.open(p);
          break;
        }
      }
      library = loaded ?? DynamicLibrary.open('universal_log_viewer.dll');
    } else if (Platform.isLinux) {
      library = DynamicLibrary.open('libuniversal_log_viewer.so');
    } else {
      throw UnsupportedError('Platform not supported for universal_log_viewer FFI');
    }

    _instance = UlvFfiBindings._(library);
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

