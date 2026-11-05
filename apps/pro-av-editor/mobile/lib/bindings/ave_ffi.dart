/// Low-level C-ABI FFI bindings for Professional Audio & Video Editor (PAVE)
///
/// Communicates directly with `pro_av_editor` compiled native Rust library.
/// Conforms to ADR-0015 for cross-platform desktop and mobile integration.

import 'dart:ffi';
import 'dart:io';
import 'package:ffi/ffi.dart';

// Opaque struct representing Rust AveHandle
final class AveHandleOpaque extends Opaque {}

// Native C typedefs
typedef _c_ave_create_handle = Pointer<AveHandleOpaque> Function(Pointer<Utf8> dataDir);
typedef _dart_ave_create_handle = Pointer<AveHandleOpaque> Function(Pointer<Utf8> dataDir);

typedef _c_ave_free_handle = Void Function(Pointer<AveHandleOpaque> handle);
typedef _dart_ave_free_handle = void Function(Pointer<AveHandleOpaque> handle);

typedef _c_ave_free_string = Void Function(Pointer<Utf8> str);
typedef _dart_ave_free_string = void Function(Pointer<Utf8> str);

typedef _c_ave_get_project_state = Pointer<Utf8> Function(Pointer<AveHandleOpaque> handle);
typedef _dart_ave_get_project_state = Pointer<Utf8> Function(Pointer<AveHandleOpaque> handle);

typedef _c_ave_create_project = Pointer<Utf8> Function(
    Pointer<AveHandleOpaque> handle, Pointer<Utf8> name, Pointer<Utf8> templateId);
typedef _dart_ave_create_project = Pointer<Utf8> Function(
    Pointer<AveHandleOpaque> handle, Pointer<Utf8> name, Pointer<Utf8> templateId);

typedef _c_ave_save_project = Int32 Function(Pointer<AveHandleOpaque> handle, Pointer<Utf8> path);
typedef _dart_ave_save_project = int Function(Pointer<AveHandleOpaque> handle, Pointer<Utf8> path);

typedef _c_ave_load_project = Pointer<Utf8> Function(Pointer<AveHandleOpaque> handle, Pointer<Utf8> path);
typedef _dart_ave_load_project = Pointer<Utf8> Function(Pointer<AveHandleOpaque> handle, Pointer<Utf8> path);

typedef _c_ave_add_clip = Pointer<Utf8> Function(Pointer<AveHandleOpaque> handle, Pointer<Utf8> clipJson);
typedef _dart_ave_add_clip = Pointer<Utf8> Function(Pointer<AveHandleOpaque> handle, Pointer<Utf8> clipJson);

typedef _c_ave_remove_clip = Int32 Function(Pointer<AveHandleOpaque> handle, Pointer<Utf8> clipId);
typedef _dart_ave_remove_clip = int Function(Pointer<AveHandleOpaque> handle, Pointer<Utf8> clipId);

typedef _c_ave_split_clip = Int32 Function(
    Pointer<AveHandleOpaque> handle, Pointer<Utf8> clipId, Uint64 timestampMs);
typedef _dart_ave_split_clip = int Function(
    Pointer<AveHandleOpaque> handle, Pointer<Utf8> clipId, int timestampMs);

typedef _c_ave_ripple_delete = Int32 Function(Pointer<AveHandleOpaque> handle, Pointer<Utf8> clipId);
typedef _dart_ave_ripple_delete = int Function(Pointer<AveHandleOpaque> handle, Pointer<Utf8> clipId);

typedef _c_ave_undo = Int32 Function(Pointer<AveHandleOpaque> handle);
typedef _dart_ave_undo = int Function(Pointer<AveHandleOpaque> handle);

typedef _c_ave_redo = Int32 Function(Pointer<AveHandleOpaque> handle);
typedef _dart_ave_redo = int Function(Pointer<AveHandleOpaque> handle);

typedef _c_ave_render_preview_bmp = Pointer<Utf8> Function(
    Pointer<AveHandleOpaque> handle, Uint64 timestampMs, Uint32 width, Uint32 height);
typedef _dart_ave_render_preview_bmp = Pointer<Utf8> Function(
    Pointer<AveHandleOpaque> handle, int timestampMs, int width, int height);

typedef _c_ave_get_video_scopes = Pointer<Utf8> Function(
    Pointer<AveHandleOpaque> handle, Uint64 timestampMs, Uint32 width, Uint32 height);
typedef _dart_ave_get_video_scopes = Pointer<Utf8> Function(
    Pointer<AveHandleOpaque> handle, int timestampMs, int width, int height);

typedef _c_ave_auto_cut_silence = Int32 Function(Pointer<AveHandleOpaque> handle, Float thresholdDbfs);
typedef _dart_ave_auto_cut_silence = int Function(Pointer<AveHandleOpaque> handle, double thresholdDbfs);

typedef _c_ave_export_edl = Pointer<Utf8> Function(Pointer<AveHandleOpaque> handle);
typedef _dart_ave_export_edl = Pointer<Utf8> Function(Pointer<AveHandleOpaque> handle);

typedef _c_ave_export_fcpxml = Pointer<Utf8> Function(Pointer<AveHandleOpaque> handle);
typedef _dart_ave_export_fcpxml = Pointer<Utf8> Function(Pointer<AveHandleOpaque> handle);

typedef _c_ave_get_templates = Pointer<Utf8> Function(Pointer<AveHandleOpaque> handle);
typedef _dart_ave_get_templates = Pointer<Utf8> Function(Pointer<AveHandleOpaque> handle);

typedef _c_ave_get_export_presets = Pointer<Utf8> Function(Pointer<AveHandleOpaque> handle);
typedef _dart_ave_get_export_presets = Pointer<Utf8> Function(Pointer<AveHandleOpaque> handle);

typedef _c_ave_dispatch_command = Pointer<Utf8> Function(
    Pointer<AveHandleOpaque> handle, Pointer<Utf8> cmd, Pointer<Utf8> argsJson);
typedef _dart_ave_dispatch_command = Pointer<Utf8> Function(
    Pointer<AveHandleOpaque> handle, Pointer<Utf8> cmd, Pointer<Utf8> argsJson);

/// Low-level FFI binding class
class AveFfiBindings {
  late final DynamicLibrary _lib;

  late final _dart_ave_create_handle _createHandle;
  late final _dart_ave_free_handle _freeHandle;
  late final _dart_ave_free_string _freeString;
  late final _dart_ave_get_project_state _getProjectState;
  late final _dart_ave_create_project _createProject;
  late final _dart_ave_save_project _saveProject;
  late final _dart_ave_load_project _loadProject;
  late final _dart_ave_add_clip _addClip;
  late final _dart_ave_remove_clip _removeClip;
  late final _dart_ave_split_clip _splitClip;
  late final _dart_ave_ripple_delete _rippleDelete;
  late final _dart_ave_undo _undo;
  late final _dart_ave_redo _redo;
  late final _dart_ave_render_preview_bmp _renderPreviewBmp;
  late final _dart_ave_get_video_scopes _getVideoScopes;
  late final _dart_ave_auto_cut_silence _autoCutSilence;
  late final _dart_ave_export_edl _exportEdl;
  late final _dart_ave_export_fcpxml _exportFcpxml;
  late final _dart_ave_get_templates _getTemplates;
  late final _dart_ave_get_export_presets _getExportPresets;
  late final _dart_ave_dispatch_command _dispatchCommand;

  AveFfiBindings([DynamicLibrary? lib]) {
    _lib = lib ?? _openLibrary();
    _bindFunctions();
  }

  static DynamicLibrary _openLibrary() {
    if (Platform.isWindows) {
      // Check debug/release paths or fallback to system search
      const paths = [
        'pro_av_editor.dll',
        'target/debug/pro_av_editor.dll',
        'target/release/pro_av_editor.dll',
      ];
      for (final p in paths) {
        try {
          return DynamicLibrary.open(p);
        } catch (_) {}
      }
      return DynamicLibrary.open('pro_av_editor.dll');
    } else if (Platform.isAndroid) {
      return DynamicLibrary.open('libpro_av_editor.so');
    } else if (Platform.isIOS || Platform.isMacOS) {
      return DynamicLibrary.process();
    } else if (Platform.isLinux) {
      return DynamicLibrary.open('libpro_av_editor.so');
    }
    throw UnsupportedError('Unsupported platform: ${Platform.operatingSystem}');
  }

  void _bindFunctions() {
    _createHandle = _lib.lookupFunction<_c_ave_create_handle, _dart_ave_create_handle>('ave_create_handle');
    _freeHandle = _lib.lookupFunction<_c_ave_free_handle, _dart_ave_free_handle>('ave_free_handle');
    _freeString = _lib.lookupFunction<_c_ave_free_string, _dart_ave_free_string>('ave_free_string');
    _getProjectState = _lib.lookupFunction<_c_ave_get_project_state, _dart_ave_get_project_state>('ave_get_project_state');
    _createProject = _lib.lookupFunction<_c_ave_create_project, _dart_ave_create_project>('ave_create_project');
    _saveProject = _lib.lookupFunction<_c_ave_save_project, _dart_ave_save_project>('ave_save_project');
    _loadProject = _lib.lookupFunction<_c_ave_load_project, _dart_ave_load_project>('ave_load_project');
    _addClip = _lib.lookupFunction<_c_ave_add_clip, _dart_ave_add_clip>('ave_add_clip');
    _removeClip = _lib.lookupFunction<_c_ave_remove_clip, _dart_ave_remove_clip>('ave_remove_clip');
    _splitClip = _lib.lookupFunction<_c_ave_split_clip, _dart_ave_split_clip>('ave_split_clip');
    _rippleDelete = _lib.lookupFunction<_c_ave_ripple_delete, _dart_ave_ripple_delete>('ave_ripple_delete');
    _undo = _lib.lookupFunction<_c_ave_undo, _dart_ave_undo>('ave_undo');
    _redo = _lib.lookupFunction<_c_ave_redo, _dart_ave_redo>('ave_redo');
    _renderPreviewBmp = _lib.lookupFunction<_c_ave_render_preview_bmp, _dart_ave_render_preview_bmp>('ave_render_preview_bmp');
    _getVideoScopes = _lib.lookupFunction<_c_ave_get_video_scopes, _dart_ave_get_video_scopes>('ave_get_video_scopes');
    _autoCutSilence = _lib.lookupFunction<_c_ave_auto_cut_silence, _dart_ave_auto_cut_silence>('ave_auto_cut_silence');
    _exportEdl = _lib.lookupFunction<_c_ave_export_edl, _dart_ave_export_edl>('ave_export_edl');
    _exportFcpxml = _lib.lookupFunction<_c_ave_export_fcpxml, _dart_ave_export_fcpxml>('ave_export_fcpxml');
    _getTemplates = _lib.lookupFunction<_c_ave_get_templates, _dart_ave_get_templates>('ave_get_templates');
    _getExportPresets = _lib.lookupFunction<_c_ave_get_export_presets, _dart_ave_get_export_presets>('ave_get_export_presets');
    _dispatchCommand = _lib.lookupFunction<_c_ave_dispatch_command, _dart_ave_dispatch_command>('ave_dispatch_command');
  }

  Pointer<AveHandleOpaque> createHandle([String? dataDir]) {
    if (dataDir != null) {
      final ptr = dataDir.toNativeUtf8();
      try {
        return _createHandle(ptr);
      } finally {
        calloc.free(ptr);
      }
    }
    return _createHandle(nullptr);
  }

  void freeHandle(Pointer<AveHandleOpaque> handle) => _freeHandle(handle);

  String getProjectState(Pointer<AveHandleOpaque> handle) {
    final ptr = _getProjectState(handle);
    if (ptr == nullptr) return '{}';
    try {
      return ptr.toDartString();
    } finally {
      _freeString(ptr);
    }
  }

  String createProject(Pointer<AveHandleOpaque> handle, String name, [String? templateId]) {
    final namePtr = name.toNativeUtf8();
    final tplPtr = templateId != null ? templateId.toNativeUtf8() : nullptr;
    try {
      final resPtr = _createProject(handle, namePtr, tplPtr);
      if (resPtr == nullptr) return '{}';
      try {
        return resPtr.toDartString();
      } finally {
        _freeString(resPtr);
      }
    } finally {
      calloc.free(namePtr);
      if (tplPtr != nullptr) calloc.free(tplPtr);
    }
  }

  bool saveProject(Pointer<AveHandleOpaque> handle, String path) {
    final pathPtr = path.toNativeUtf8();
    try {
      return _saveProject(handle, pathPtr) == 0;
    } finally {
      calloc.free(pathPtr);
    }
  }

  String loadProject(Pointer<AveHandleOpaque> handle, String path) {
    final pathPtr = path.toNativeUtf8();
    try {
      final resPtr = _loadProject(handle, pathPtr);
      if (resPtr == nullptr) return '{}';
      try {
        return resPtr.toDartString();
      } finally {
        _freeString(resPtr);
      }
    } finally {
      calloc.free(pathPtr);
    }
  }

  String addClip(Pointer<AveHandleOpaque> handle, String clipJson) {
    final jsonPtr = clipJson.toNativeUtf8();
    try {
      final resPtr = _addClip(handle, jsonPtr);
      if (resPtr == nullptr) return '{}';
      try {
        return resPtr.toDartString();
      } finally {
        _freeString(resPtr);
      }
    } finally {
      calloc.free(jsonPtr);
    }
  }

  bool removeClip(Pointer<AveHandleOpaque> handle, String clipId) {
    final idPtr = clipId.toNativeUtf8();
    try {
      return _removeClip(handle, idPtr) == 0;
    } finally {
      calloc.free(idPtr);
    }
  }

  bool splitClip(Pointer<AveHandleOpaque> handle, String clipId, int timestampMs) {
    final idPtr = clipId.toNativeUtf8();
    try {
      return _splitClip(handle, idPtr, timestampMs) == 0;
    } finally {
      calloc.free(idPtr);
    }
  }

  bool rippleDelete(Pointer<AveHandleOpaque> handle, String clipId) {
    final idPtr = clipId.toNativeUtf8();
    try {
      return _rippleDelete(handle, idPtr) == 0;
    } finally {
      calloc.free(idPtr);
    }
  }

  int undo(Pointer<AveHandleOpaque> handle) => _undo(handle);
  int redo(Pointer<AveHandleOpaque> handle) => _redo(handle);

  String renderPreviewBmp(Pointer<AveHandleOpaque> handle, int timestampMs, int width, int height) {
    final ptr = _renderPreviewBmp(handle, timestampMs, width, height);
    if (ptr == nullptr) return '{}';
    try {
      return ptr.toDartString();
    } finally {
      _freeString(ptr);
    }
  }

  String getVideoScopes(Pointer<AveHandleOpaque> handle, int timestampMs, int width, int height) {
    final ptr = _getVideoScopes(handle, timestampMs, width, height);
    if (ptr == nullptr) return '{}';
    try {
      return ptr.toDartString();
    } finally {
      _freeString(ptr);
    }
  }

  int autoCutSilence(Pointer<AveHandleOpaque> handle, double thresholdDbfs) =>
      _autoCutSilence(handle, thresholdDbfs);

  String exportEdl(Pointer<AveHandleOpaque> handle) {
    final ptr = _exportEdl(handle);
    if (ptr == nullptr) return '{}';
    try {
      return ptr.toDartString();
    } finally {
      _freeString(ptr);
    }
  }

  String exportFcpxml(Pointer<AveHandleOpaque> handle) {
    final ptr = _exportFcpxml(handle);
    if (ptr == nullptr) return '{}';
    try {
      return ptr.toDartString();
    } finally {
      _freeString(ptr);
    }
  }

  String getTemplates(Pointer<AveHandleOpaque> handle) {
    final ptr = _getTemplates(handle);
    if (ptr == nullptr) return '[]';
    try {
      return ptr.toDartString();
    } finally {
      _freeString(ptr);
    }
  }

  String getExportPresets(Pointer<AveHandleOpaque> handle) {
    final ptr = _getExportPresets(handle);
    if (ptr == nullptr) return '[]';
    try {
      return ptr.toDartString();
    } finally {
      _freeString(ptr);
    }
  }

  String dispatchCommand(Pointer<AveHandleOpaque> handle, String cmd, String argsJson) {
    final cmdPtr = cmd.toNativeUtf8();
    final argsPtr = argsJson.toNativeUtf8();
    try {
      final ptr = _dispatchCommand(handle, cmdPtr, argsPtr);
      if (ptr == nullptr) return '{}';
      try {
        return ptr.toDartString();
      } finally {
        _freeString(ptr);
      }
    } finally {
      calloc.free(cmdPtr);
      calloc.free(argsPtr);
    }
  }
}

