/// Low-level C-ABI FFI bindings for AI File Explorer
///
/// Communicates directly with `ai_file_explorer` compiled native Rust library.
/// Conforms to ADR-0015 for cross-platform desktop and mobile integration.

import 'dart:ffi';
import 'dart:io';
import 'package:ffi/ffi.dart';

// Opaque struct representing Rust AfeHandle
final class AfeHandleOpaque extends Opaque {}

// Native C typedefs
typedef _c_afe_create_handle = Pointer<AfeHandleOpaque> Function(
    Pointer<Utf8> dataDir);
typedef _dart_afe_create_handle = Pointer<AfeHandleOpaque> Function(
    Pointer<Utf8> dataDir);

typedef _c_afe_free_handle = Void Function(Pointer<AfeHandleOpaque> handle);
typedef _dart_afe_free_handle = void Function(Pointer<AfeHandleOpaque> handle);

typedef _c_afe_free_string = Void Function(Pointer<Utf8> str);
typedef _dart_afe_free_string = void Function(Pointer<Utf8> str);

typedef _c_afe_list_directory = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> path);
typedef _dart_afe_list_directory = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> path);

typedef _c_afe_search_files = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> query, IntPtr limit);
typedef _dart_afe_search_files = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> query, int limit);

typedef _c_afe_index_directory = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> path);
typedef _dart_afe_index_directory = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> path);

typedef _c_afe_scan_duplicates = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> rootsJson, Uint64 minSizeBytes);
typedef _dart_afe_scan_duplicates = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> rootsJson, int minSizeBytes);

typedef _c_afe_analyze_storage = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> path, IntPtr topLimit);
typedef _dart_afe_analyze_storage = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> path, int topLimit);

typedef _c_afe_generate_treemap = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle,
    Pointer<Utf8> path,
    Double width,
    Double height,
    IntPtr maxDepth);
typedef _dart_afe_generate_treemap = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle,
    Pointer<Utf8> path,
    double width,
    double height,
    int maxDepth);

typedef _c_afe_preview_batch_rename = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle,
    Pointer<Utf8> pathsJson,
    Pointer<Utf8> ruleJson);
typedef _dart_afe_preview_batch_rename = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle,
    Pointer<Utf8> pathsJson,
    Pointer<Utf8> ruleJson);

typedef _c_afe_execute_batch_rename = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> previewsJson);
typedef _dart_afe_execute_batch_rename = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> previewsJson);

typedef _c_afe_extract_document = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> path);
typedef _dart_afe_extract_document = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> path);

typedef _c_afe_search_content = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> queryJson);
typedef _dart_afe_search_content = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> queryJson);

typedef _c_afe_ocr_image = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> imagePath);
typedef _dart_afe_ocr_image = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> imagePath);

typedef _c_afe_index_screenshot = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> imagePath);
typedef _dart_afe_index_screenshot = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> imagePath);

typedef _c_afe_search_screenshots_by_hash = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle,
    Uint64 targetHash,
    Uint32 maxDistance,
    IntPtr limit);
typedef _dart_afe_search_screenshots_by_hash = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle,
    int targetHash,
    int maxDistance,
    int limit);

typedef _c_afe_search_screenshots_by_text = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> query, IntPtr limit);
typedef _dart_afe_search_screenshots_by_text = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> query, int limit);

typedef _c_afe_scan_compliance = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle,
    Pointer<Utf8> path,
    Pointer<Utf8> tier);
typedef _dart_afe_scan_compliance = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle,
    Pointer<Utf8> path,
    Pointer<Utf8> tier);

typedef _c_afe_compute_file_hash = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> path);
typedef _dart_afe_compute_file_hash = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> path);

typedef _c_afe_inspect_archive = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> path, IntPtr maxEntries);
typedef _dart_afe_inspect_archive = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> path, int maxEntries);

typedef _c_afe_dispatch_command = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> cmd, Pointer<Utf8> argsJson);
typedef _dart_afe_dispatch_command = Pointer<Utf8> Function(
    Pointer<AfeHandleOpaque> handle, Pointer<Utf8> cmd, Pointer<Utf8> argsJson);

/// Complete FFI Bindings for AI File Explorer Native Engine
class AfeFfiBindings {
  final DynamicLibrary _lib;

  late final _dart_afe_create_handle createHandle;
  late final _dart_afe_free_handle freeHandle;
  late final _dart_afe_free_string freeString;
  late final _dart_afe_list_directory listDirectory;
  late final _dart_afe_search_files searchFiles;
  late final _dart_afe_index_directory indexDirectory;
  late final _dart_afe_scan_duplicates scanDuplicates;
  late final _dart_afe_analyze_storage analyzeStorage;
  late final _dart_afe_generate_treemap generateTreemap;
  late final _dart_afe_preview_batch_rename previewBatchRename;
  late final _dart_afe_execute_batch_rename executeBatchRename;
  late final _dart_afe_extract_document extractDocument;
  late final _dart_afe_search_content searchContent;
  late final _dart_afe_ocr_image ocrImage;
  late final _dart_afe_index_screenshot indexScreenshot;
  late final _dart_afe_search_screenshots_by_hash searchScreenshotsByHash;
  late final _dart_afe_search_screenshots_by_text searchScreenshotsByText;
  late final _dart_afe_scan_compliance scanCompliance;
  late final _dart_afe_compute_file_hash computeFileHash;
  late final _dart_afe_inspect_archive inspectArchive;
  late final _dart_afe_dispatch_command dispatchCommand;

  AfeFfiBindings(this._lib) {
    createHandle = _lib
        .lookup<NativeFunction<_c_afe_create_handle>>('afe_create_handle')
        .asFunction();
    freeHandle = _lib
        .lookup<NativeFunction<_c_afe_free_handle>>('afe_free_handle')
        .asFunction();
    freeString = _lib
        .lookup<NativeFunction<_c_afe_free_string>>('afe_free_string')
        .asFunction();
    listDirectory = _lib
        .lookup<NativeFunction<_c_afe_list_directory>>('afe_list_directory')
        .asFunction();
    searchFiles = _lib
        .lookup<NativeFunction<_c_afe_search_files>>('afe_search_files')
        .asFunction();
    indexDirectory = _lib
        .lookup<NativeFunction<_c_afe_index_directory>>('afe_index_directory')
        .asFunction();
    scanDuplicates = _lib
        .lookup<NativeFunction<_c_afe_scan_duplicates>>('afe_scan_duplicates')
        .asFunction();
    analyzeStorage = _lib
        .lookup<NativeFunction<_c_afe_analyze_storage>>('afe_analyze_storage')
        .asFunction();
    generateTreemap = _lib
        .lookup<NativeFunction<_c_afe_generate_treemap>>('afe_generate_treemap')
        .asFunction();
    previewBatchRename = _lib
        .lookup<NativeFunction<_c_afe_preview_batch_rename>>('afe_preview_batch_rename')
        .asFunction();
    executeBatchRename = _lib
        .lookup<NativeFunction<_c_afe_execute_batch_rename>>('afe_execute_batch_rename')
        .asFunction();
    extractDocument = _lib
        .lookup<NativeFunction<_c_afe_extract_document>>('afe_extract_document')
        .asFunction();
    searchContent = _lib
        .lookup<NativeFunction<_c_afe_search_content>>('afe_search_content')
        .asFunction();
    ocrImage = _lib
        .lookup<NativeFunction<_c_afe_ocr_image>>('afe_ocr_image')
        .asFunction();
    indexScreenshot = _lib
        .lookup<NativeFunction<_c_afe_index_screenshot>>('afe_index_screenshot')
        .asFunction();
    searchScreenshotsByHash = _lib
        .lookup<NativeFunction<_c_afe_search_screenshots_by_hash>>('afe_search_screenshots_by_hash')
        .asFunction();
    searchScreenshotsByText = _lib
        .lookup<NativeFunction<_c_afe_search_screenshots_by_text>>('afe_search_screenshots_by_text')
        .asFunction();
    scanCompliance = _lib
        .lookup<NativeFunction<_c_afe_scan_compliance>>('afe_scan_compliance')
        .asFunction();
    computeFileHash = _lib
        .lookup<NativeFunction<_c_afe_compute_file_hash>>('afe_compute_file_hash')
        .asFunction();
    inspectArchive = _lib
        .lookup<NativeFunction<_c_afe_inspect_archive>>('afe_inspect_archive')
        .asFunction();
    dispatchCommand = _lib
        .lookup<NativeFunction<_c_afe_dispatch_command>>('afe_dispatch_command')
        .asFunction();
  }

  /// Load native library based on runtime host platform
  static AfeFfiBindings load() {
    DynamicLibrary lib;
    if (Platform.isWindows) {
      lib = DynamicLibrary.open('ai_file_explorer.dll');
    } else if (Platform.isLinux) {
      lib = DynamicLibrary.open('libai_file_explorer.so');
    } else if (Platform.isMacOS) {
      lib = DynamicLibrary.open('libai_file_explorer.dylib');
    } else if (Platform.isAndroid) {
      lib = DynamicLibrary.open('libai_file_explorer.so');
    } else if (Platform.isIOS) {
      lib = DynamicLibrary.process();
    } else {
      throw UnsupportedError('Unsupported operating system for AfeFfiBindings: ${Platform.operatingSystem}');
    }
    return AfeFfiBindings(lib);
  }
}

