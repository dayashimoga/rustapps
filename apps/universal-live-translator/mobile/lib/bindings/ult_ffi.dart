/// Low-level C-ABI FFI bindings for Universal Live Translator
///
/// Communicates directly with `universal_live_translator` compiled native Rust library.
/// Conforms to ADR-0015 for cross-platform desktop and mobile integration.

import 'dart:ffi';
import 'dart:io';
import 'package:ffi/ffi.dart';

// Opaque struct representing Rust UltHandle
final class UltHandleOpaque extends Opaque {}

// Native C typedefs
typedef _c_ult_create_handle = Pointer<UltHandleOpaque> Function(
    Pointer<Utf8> dataDir);
typedef _dart_ult_create_handle = Pointer<UltHandleOpaque> Function(
    Pointer<Utf8> dataDir);

typedef _c_ult_free_handle = Void Function(Pointer<UltHandleOpaque> handle);
typedef _dart_ult_free_handle = void Function(Pointer<UltHandleOpaque> handle);

typedef _c_ult_free_string = Void Function(Pointer<Utf8> str);
typedef _dart_ult_free_string = void Function(Pointer<Utf8> str);

typedef _c_ult_detect_language = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle, Pointer<Utf8> text);
typedef _dart_ult_detect_language = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle, Pointer<Utf8> text);

typedef _c_ult_translate_text = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle, Pointer<Utf8> reqJson);
typedef _dart_ult_translate_text = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle, Pointer<Utf8> reqJson);

typedef _c_ult_synthesize_speech = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle, Pointer<Utf8> reqJson);
typedef _dart_ult_synthesize_speech = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle, Pointer<Utf8> reqJson);

typedef _c_ult_translate_camera_image = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle,
    Pointer<Uint8> imageBytes,
    IntPtr length,
    Pointer<Utf8> sourceLang,
    Pointer<Utf8> targetLang);
typedef _dart_ult_translate_camera_image = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle,
    Pointer<Uint8> imageBytes,
    int length,
    Pointer<Utf8> sourceLang,
    Pointer<Utf8> targetLang);

typedef _c_ult_translate_subtitles = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle,
    Pointer<Utf8> content,
    Pointer<Utf8> format,
    Pointer<Utf8> sourceLang,
    Pointer<Utf8> targetLang);
typedef _dart_ult_translate_subtitles = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle,
    Pointer<Utf8> content,
    Pointer<Utf8> format,
    Pointer<Utf8> sourceLang,
    Pointer<Utf8> targetLang);

typedef _c_ult_add_phrase = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle, Pointer<Utf8> itemJson);
typedef _dart_ult_add_phrase = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle, Pointer<Utf8> itemJson);

typedef _c_ult_list_phrases = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle, Pointer<Utf8> category, Int32 favoriteOnly);
typedef _dart_ult_list_phrases = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle, Pointer<Utf8> category, int favoriteOnly);

typedef _c_ult_toggle_favorite_phrase = Int32 Function(
    Pointer<UltHandleOpaque> handle, Pointer<Utf8> id);
typedef _dart_ult_toggle_favorite_phrase = int Function(
    Pointer<UltHandleOpaque> handle, Pointer<Utf8> id);

typedef _c_ult_delete_phrase = Int32 Function(
    Pointer<UltHandleOpaque> handle, Pointer<Utf8> id);
typedef _dart_ult_delete_phrase = int Function(
    Pointer<UltHandleOpaque> handle, Pointer<Utf8> id);

typedef _c_ult_get_history = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle, IntPtr limit);
typedef _dart_ult_get_history = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle, int limit);

typedef _c_ult_start_conversation_session = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle,
    Pointer<Utf8> partyALang,
    Pointer<Utf8> partyBLang);
typedef _dart_ult_start_conversation_session = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle,
    Pointer<Utf8> partyALang,
    Pointer<Utf8> partyBLang);

typedef _c_ult_add_conversation_turn = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle,
    Pointer<Utf8> sessionId,
    Pointer<Utf8> speaker,
    Pointer<Utf8> text);
typedef _dart_ult_add_conversation_turn = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle,
    Pointer<Utf8> sessionId,
    Pointer<Utf8> speaker,
    Pointer<Utf8> text);

typedef _c_ult_add_glossary_term = Int32 Function(
    Pointer<UltHandleOpaque> handle,
    Pointer<Utf8> domain,
    Pointer<Utf8> sourceTerm,
    Pointer<Utf8> targetTerm,
    Int32 caseSensitive);
typedef _dart_ult_add_glossary_term = int Function(
    Pointer<UltHandleOpaque> handle,
    Pointer<Utf8> domain,
    Pointer<Utf8> sourceTerm,
    Pointer<Utf8> targetTerm,
    int caseSensitive);

typedef _c_ult_get_glossary_terms = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle, Pointer<Utf8> domain);
typedef _dart_ult_get_glossary_terms = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle, Pointer<Utf8> domain);

typedef _c_ult_list_glossary_domains = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle);
typedef _dart_ult_list_glossary_domains = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle);

typedef _c_ult_dispatch_command = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle, Pointer<Utf8> cmd, Pointer<Utf8> argsJson);
typedef _dart_ult_dispatch_command = Pointer<Utf8> Function(
    Pointer<UltHandleOpaque> handle, Pointer<Utf8> cmd, Pointer<Utf8> argsJson);

/// High-level Dart wrapper around native `ult_*` FFI symbols
class UltNativeBindings {
  static UltNativeBindings? _instance;
  final DynamicLibrary _dylib;

  late final _dart_ult_create_handle ultCreateHandle;
  late final _dart_ult_free_handle ultFreeHandle;
  late final _dart_ult_free_string ultFreeString;
  late final _dart_ult_detect_language ultDetectLanguage;
  late final _dart_ult_translate_text ultTranslateText;
  late final _dart_ult_synthesize_speech ultSynthesizeSpeech;
  late final _dart_ult_translate_camera_image ultTranslateCameraImage;
  late final _dart_ult_translate_subtitles ultTranslateSubtitles;
  late final _dart_ult_add_phrase ultAddPhrase;
  late final _dart_ult_list_phrases ultListPhrases;
  late final _dart_ult_toggle_favorite_phrase ultToggleFavoritePhrase;
  late final _dart_ult_delete_phrase ultDeletePhrase;
  late final _dart_ult_get_history ultGetHistory;
  late final _dart_ult_start_conversation_session ultStartConversationSession;
  late final _dart_ult_add_conversation_turn ultAddConversationTurn;
  late final _dart_ult_add_glossary_term ultAddGlossaryTerm;
  late final _dart_ult_get_glossary_terms ultGetGlossaryTerms;
  late final _dart_ult_list_glossary_domains ultListGlossaryDomains;
  late final _dart_ult_dispatch_command ultDispatchCommand;

  UltNativeBindings._(this._dylib) {
    ultCreateHandle = _dylib
        .lookup<NativeFunction<_c_ult_create_handle>>('ult_create_handle')
        .asFunction();
    ultFreeHandle = _dylib
        .lookup<NativeFunction<_c_ult_free_handle>>('ult_free_handle')
        .asFunction();
    ultFreeString = _dylib
        .lookup<NativeFunction<_c_ult_free_string>>('ult_free_string')
        .asFunction();
    ultDetectLanguage = _dylib
        .lookup<NativeFunction<_c_ult_detect_language>>('ult_detect_language')
        .asFunction();
    ultTranslateText = _dylib
        .lookup<NativeFunction<_c_ult_translate_text>>('ult_translate_text')
        .asFunction();
    ultSynthesizeSpeech = _dylib
        .lookup<NativeFunction<_c_ult_synthesize_speech>>('ult_synthesize_speech')
        .asFunction();
    ultTranslateCameraImage = _dylib
        .lookup<NativeFunction<_c_ult_translate_camera_image>>('ult_translate_camera_image')
        .asFunction();
    ultTranslateSubtitles = _dylib
        .lookup<NativeFunction<_c_ult_translate_subtitles>>('ult_translate_subtitles')
        .asFunction();
    ultAddPhrase = _dylib
        .lookup<NativeFunction<_c_ult_add_phrase>>('ult_add_phrase')
        .asFunction();
    ultListPhrases = _dylib
        .lookup<NativeFunction<_c_ult_list_phrases>>('ult_list_phrases')
        .asFunction();
    ultToggleFavoritePhrase = _dylib
        .lookup<NativeFunction<_c_ult_toggle_favorite_phrase>>('ult_toggle_favorite_phrase')
        .asFunction();
    ultDeletePhrase = _dylib
        .lookup<NativeFunction<_c_ult_delete_phrase>>('ult_delete_phrase')
        .asFunction();
    ultGetHistory = _dylib
        .lookup<NativeFunction<_c_ult_get_history>>('ult_get_history')
        .asFunction();
    ultStartConversationSession = _dylib
        .lookup<NativeFunction<_c_ult_start_conversation_session>>('ult_start_conversation_session')
        .asFunction();
    ultAddConversationTurn = _dylib
        .lookup<NativeFunction<_c_ult_add_conversation_turn>>('ult_add_conversation_turn')
        .asFunction();
    ultAddGlossaryTerm = _dylib
        .lookup<NativeFunction<_c_ult_add_glossary_term>>('ult_add_glossary_term')
        .asFunction();
    ultGetGlossaryTerms = _dylib
        .lookup<NativeFunction<_c_ult_get_glossary_terms>>('ult_get_glossary_terms')
        .asFunction();
    ultListGlossaryDomains = _dylib
        .lookup<NativeFunction<_c_ult_list_glossary_domains>>('ult_list_glossary_domains')
        .asFunction();
    ultDispatchCommand = _dylib
        .lookup<NativeFunction<_c_ult_dispatch_command>>('ult_dispatch_command')
        .asFunction();
  }

  static UltNativeBindings instance([String? testLibraryPath]) {
    if (_instance != null) return _instance!;

    DynamicLibrary dylib;
    if (testLibraryPath != null) {
      dylib = DynamicLibrary.open(testLibraryPath);
    } else if (Platform.isWindows) {
      dylib = DynamicLibrary.open('universal_live_translator.dll');
    } else if (Platform.isLinux || Platform.isAndroid) {
      dylib = DynamicLibrary.open('libuniversal_live_translator.so');
    } else if (Platform.isMacOS || Platform.isIOS) {
      dylib = DynamicLibrary.open('libuniversal_live_translator.dylib');
    } else {
      throw UnsupportedError('Unsupported operating system for ULT FFI');
    }

    _instance = UltNativeBindings._(dylib);
    return _instance!;
  }
}

