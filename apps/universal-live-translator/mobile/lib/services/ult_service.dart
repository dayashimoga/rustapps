/// Service managing Universal Live Translator native operations for Flutter
///
/// Implements full offline FFI calls to `ult_*` functions with memory safety.

import 'dart:convert';
import 'dart:ffi';
import 'package:ffi/ffi.dart';
import 'package:flutter/foundation.dart';
import '../bindings/ult_ffi.dart';
import '../models/translation_models.dart';

class UltService extends ChangeNotifier {
  UltNativeBindings? _bindings;
  Pointer<UltHandleOpaque>? _handle;
  bool _isInitialized = false;

  bool _mockMode = false;

  UltService({bool mockMode = false}) : _mockMode = mockMode;

  bool get isInitialized => _isInitialized;
  bool get isMockMode => _mockMode;

  /// Initialize service with optional data directory
  Future<void> initialize({String? dataDir, String? dylibPath, bool? mockMode}) async {
    if (mockMode != null) {
      _mockMode = mockMode;
    }
    if (_mockMode) {
      _isInitialized = true;
      notifyListeners();
      return;
    }
    try {
      _bindings = UltNativeBindings.instance(dylibPath);
      final dirPtr = dataDir != null ? dataDir.toNativeUtf8() : nullptr;
      _handle = _bindings!.ultCreateHandle(dirPtr);
      if (dirPtr != nullptr) calloc.free(dirPtr);

      if (_handle != null && _handle != nullptr) {
        _isInitialized = true;
        notifyListeners();
      } else {
        throw StateError('ult_create_handle returned null pointer');
      }
    } catch (e) {
      _isInitialized = false;
      throw StateError('Failed to initialize native Universal Live Translator FFI library: $e');
    }
  }

  /// Detect language of input text in <500µs
  Future<LidResult?> detectLanguage(String text) async {
    if (_bindings == null || _handle == null || _handle == nullptr) {
      // Deterministic fallback for mock/preview mode
      return LidResult(
        language: text.contains('¿') || text.contains('ñ') ? 'es' : 'en',
        confidence: 0.95,
        detectedScript: 'Latin',
      );
    }

    final textPtr = text.toNativeUtf8();
    try {
      final resPtr = _bindings!.ultDetectLanguage(_handle!, textPtr);
      if (resPtr == nullptr) return null;
      final jsonStr = resPtr.toDartString();
      _bindings!.ultFreeString(resPtr);

      final map = jsonDecode(jsonStr) as Map<String, dynamic>;
      if (map.containsKey('error')) return null;
      return LidResult.fromJson(map);
    } finally {
      calloc.free(textPtr);
    }
  }

  /// Translate text in <150ms
  Future<TranslationResponse> translateText(TranslationRequest request) async {
    if (_bindings == null || _handle == null || _handle == nullptr) {
      // Deterministic fallback
      return TranslationResponse(
        originalText: request.text,
        translatedText: '[${request.targetLang.toUpperCase()}] ${request.text}',
        sourceLang: request.sourceLang ?? 'en',
        targetLang: request.targetLang,
        confidence: 0.94,
        latencyMs: 12.0,
        detectedSource: request.sourceLang == null,
        glossaryTermsApplied: [],
      );
    }

    final jsonStr = jsonEncode(request.toJson());
    final reqPtr = jsonStr.toNativeUtf8();
    try {
      final resPtr = _bindings!.ultTranslateText(_handle!, reqPtr);
      final outJson = resPtr.toDartString();
      _bindings!.ultFreeString(resPtr);

      final map = jsonDecode(outJson) as Map<String, dynamic>;
      return TranslationResponse.fromJson(map);
    } finally {
      calloc.free(reqPtr);
    }
  }

  /// Start a split-screen conversation session
  Future<ConversationSession> startConversationSession(
      String partyALang, String partyBLang) async {
    if (_bindings == null || _handle == null || _handle == nullptr) {
      return ConversationSession(
        id: 'session_${DateTime.now().millisecondsSinceEpoch}',
        partyALang: partyALang,
        partyBLang: partyBLang,
        turns: [],
        createdAt: DateTime.now().millisecondsSinceEpoch ~/ 1000,
      );
    }

    final aPtr = partyALang.toNativeUtf8();
    final bPtr = partyBLang.toNativeUtf8();
    try {
      final resPtr =
          _bindings!.ultStartConversationSession(_handle!, aPtr, bPtr);
      final outJson = resPtr.toDartString();
      _bindings!.ultFreeString(resPtr);

      final map = jsonDecode(outJson) as Map<String, dynamic>;
      return ConversationSession.fromJson(map);
    } finally {
      calloc.free(aPtr);
      calloc.free(bPtr);
    }
  }

  /// Add a turn to an active conversation
  Future<ConversationTurn> addConversationTurn(
      String sessionId, String speaker, String text) async {
    if (_bindings == null || _handle == null || _handle == nullptr) {
      return ConversationTurn(
        id: 'turn_${DateTime.now().millisecondsSinceEpoch}',
        sessionId: sessionId,
        speaker: speaker,
        originalText: text,
        translatedText: '[Translated] $text',
        sourceLang: speaker == 'party_a' ? 'en' : 'es',
        targetLang: speaker == 'party_a' ? 'es' : 'en',
        timestamp: DateTime.now().millisecondsSinceEpoch ~/ 1000,
      );
    }

    final sPtr = sessionId.toNativeUtf8();
    final spkPtr = speaker.toNativeUtf8();
    final tPtr = text.toNativeUtf8();
    try {
      final resPtr =
          _bindings!.ultAddConversationTurn(_handle!, sPtr, spkPtr, tPtr);
      final outJson = resPtr.toDartString();
      _bindings!.ultFreeString(resPtr);

      final map = jsonDecode(outJson) as Map<String, dynamic>;
      return ConversationTurn.fromJson(map);
    } finally {
      calloc.free(sPtr);
      calloc.free(spkPtr);
      calloc.free(tPtr);
    }
  }

  /// Fetch phrasebook items
  Future<List<PhrasebookItem>> listPhrases(
      {String? category, bool favoriteOnly = false}) async {
    if (_bindings == null || _handle == null || _handle == nullptr) {
      return [
        PhrasebookItem(
          id: 'p1',
          category: 'Greeting',
          sourceLang: 'en',
          targetLang: 'es',
          sourceText: 'Hello',
          targetText: 'Hola',
          favorite: true,
          createdAt: 1700000000,
        ),
      ];
    }

    final catPtr = category != null ? category.toNativeUtf8() : nullptr;
    try {
      final resPtr =
          _bindings!.ultListPhrases(_handle!, catPtr, favoriteOnly ? 1 : 0);
      final outJson = resPtr.toDartString();
      _bindings!.ultFreeString(resPtr);

      final list = jsonDecode(outJson) as List<dynamic>;
      return list
          .map((e) => PhrasebookItem.fromJson(e as Map<String, dynamic>))
          .toList();
    } finally {
      if (catPtr != nullptr) calloc.free(catPtr);
    }
  }

  @override
  void dispose() {
    if (_bindings != null && _handle != null && _handle != nullptr) {
      _bindings!.ultFreeHandle(_handle!);
      _handle = null;
    }
    super.dispose();
  }
}

