/// Data models for Universal Live Translator Mobile Client

class LanguageInfo {
  final String code;
  final String displayName;
  final String nativeName;
  final String script;
  final bool isRtl;

  const LanguageInfo({
    required this.code,
    required this.displayName,
    required this.nativeName,
    required this.script,
    required this.isRtl,
  });

  factory LanguageInfo.fromJson(Map<String, dynamic> json) {
    return LanguageInfo(
      code: json['code'] as String,
      displayName: json['displayName'] as String? ?? json['display_name'] as String,
      nativeName: json['nativeName'] as String? ?? json['native_name'] as String,
      script: json['script'] as String,
      isRtl: json['isRtl'] as bool? ?? json['is_rtl'] as bool? ?? false,
    );
  }

  Map<String, dynamic> toJson() => {
    'code': code,
    'displayName': displayName,
    'nativeName': nativeName,
    'script': script,
    'isRtl': isRtl,
  };
}

class LidResult {
  final String language;
  final double confidence;
  final String detectedScript;

  LidResult({
    required this.language,
    required this.confidence,
    required this.detectedScript,
  });

  factory LidResult.fromJson(Map<String, dynamic> json) {
    return LidResult(
      language: json['language'] as String,
      confidence: (json['confidence'] as num).toDouble(),
      detectedScript: json['detected_script'] as String? ?? json['detectedScript'] as String? ?? 'Unknown',
    );
  }
}

class TranslationRequest {
  final String text;
  final String? sourceLang;
  final String targetLang;
  final bool preserveFormatting;
  final String? domainGlossary;

  TranslationRequest({
    required this.text,
    this.sourceLang,
    required this.targetLang,
    this.preserveFormatting = false,
    this.domainGlossary,
  });

  Map<String, dynamic> toJson() => {
    'text': text,
    'source_lang': sourceLang,
    'target_lang': targetLang,
    'preserve_formatting': preserveFormatting,
    'domain_glossary': domainGlossary,
  };
}

class TranslationResponse {
  final String originalText;
  final String translatedText;
  final String sourceLang;
  final String targetLang;
  final double confidence;
  final double latencyMs;
  final bool detectedSource;
  final List<String> glossaryTermsApplied;

  TranslationResponse({
    required this.originalText,
    required this.translatedText,
    required this.sourceLang,
    required this.targetLang,
    required this.confidence,
    required this.latencyMs,
    required this.detectedSource,
    required this.glossaryTermsApplied,
  });

  factory TranslationResponse.fromJson(Map<String, dynamic> json) {
    return TranslationResponse(
      originalText: json['original_text'] as String? ?? '',
      translatedText: json['translated_text'] as String? ?? '',
      sourceLang: json['source_lang'] as String? ?? 'en',
      targetLang: json['target_lang'] as String? ?? 'es',
      confidence: (json['confidence'] as num?)?.toDouble() ?? 0.9,
      latencyMs: (json['latency_ms'] as num?)?.toDouble() ?? 10.0,
      detectedSource: json['detected_source'] as bool? ?? false,
      glossaryTermsApplied: (json['glossary_terms_applied'] as List<dynamic>?)
              ?.map((e) => e.toString())
              .toList() ??
          [],
    );
  }
}

class ConversationTurn {
  final String id;
  final String sessionId;
  final String speaker;
  final String originalText;
  final String translatedText;
  final String sourceLang;
  final String targetLang;
  final int timestamp;

  ConversationTurn({
    required this.id,
    required this.sessionId,
    required this.speaker,
    required this.originalText,
    required this.translatedText,
    required this.sourceLang,
    required this.targetLang,
    required this.timestamp,
  });

  factory ConversationTurn.fromJson(Map<String, dynamic> json) {
    return ConversationTurn(
      id: json['id'] as String,
      sessionId: json['session_id'] as String,
      speaker: json['speaker'] as String,
      originalText: json['original_text'] as String,
      translatedText: json['translated_text'] as String,
      sourceLang: json['source_lang'] as String,
      targetLang: json['target_lang'] as String,
      timestamp: (json['timestamp'] as num).toInt(),
    );
  }
}

class ConversationSession {
  final String id;
  final String partyALang;
  final String partyBLang;
  final List<ConversationTurn> turns;
  final int createdAt;

  ConversationSession({
    required this.id,
    required this.partyALang,
    required this.partyBLang,
    required this.turns,
    required this.createdAt,
  });

  factory ConversationSession.fromJson(Map<String, dynamic> json) {
    return ConversationSession(
      id: json['id'] as String,
      partyALang: json['party_a_lang'] as String,
      partyBLang: json['party_b_lang'] as String,
      turns: (json['turns'] as List<dynamic>?)
              ?.map((t) => ConversationTurn.fromJson(t as Map<String, dynamic>))
              .toList() ??
          [],
      createdAt: (json['created_at'] as num).toInt(),
    );
  }
}

class PhrasebookItem {
  final String id;
  final String category;
  final String sourceLang;
  final String targetLang;
  final String sourceText;
  final String targetText;
  final String? notes;
  final bool favorite;
  final int createdAt;

  PhrasebookItem({
    required this.id,
    required this.category,
    required this.sourceLang,
    required this.targetLang,
    required this.sourceText,
    required this.targetText,
    this.notes,
    required this.favorite,
    required this.createdAt,
  });

  factory PhrasebookItem.fromJson(Map<String, dynamic> json) {
    return PhrasebookItem(
      id: json['id'] as String,
      category: json['category'] as String,
      sourceLang: json['source_lang'] as String,
      targetLang: json['target_lang'] as String,
      sourceText: json['source_text'] as String,
      targetText: json['target_text'] as String,
      notes: json['notes'] as String?,
      favorite: json['favorite'] as bool? ?? false,
      createdAt: (json['created_at'] as num).toInt(),
    );
  }
}

class TranslationHistoryRecord {
  final String id;
  final String sourceLang;
  final String targetLang;
  final String sourceText;
  final String targetText;
  final int timestamp;
  final bool favorite;

  TranslationHistoryRecord({
    required this.id,
    required this.sourceLang,
    required this.targetLang,
    required this.sourceText,
    required this.targetText,
    required this.timestamp,
    required this.favorite,
  });

  factory TranslationHistoryRecord.fromJson(Map<String, dynamic> json) {
    return TranslationHistoryRecord(
      id: json['id'] as String,
      sourceLang: json['source_lang'] as String,
      targetLang: json['target_lang'] as String,
      sourceText: json['source_text'] as String,
      targetText: json['target_text'] as String,
      timestamp: (json['timestamp'] as num).toInt(),
      favorite: json['favorite'] as bool? ?? false,
    );
  }
}

const List<LanguageInfo> supportedLanguagesList = [
  LanguageInfo(code: 'en', displayName: 'English', nativeName: 'English', script: 'Latin', isRtl: false),
  LanguageInfo(code: 'es', displayName: 'Spanish', nativeName: 'Español', script: 'Latin', isRtl: false),
  LanguageInfo(code: 'fr', displayName: 'French', nativeName: 'Français', script: 'Latin', isRtl: false),
  LanguageInfo(code: 'de', displayName: 'German', nativeName: 'Deutsch', script: 'Latin', isRtl: false),
  LanguageInfo(code: 'it', displayName: 'Italian', nativeName: 'Italiano', script: 'Latin', isRtl: false),
  LanguageInfo(code: 'pt', displayName: 'Portuguese', nativeName: 'Português', script: 'Latin', isRtl: false),
  LanguageInfo(code: 'nl', displayName: 'Dutch', nativeName: 'Nederlands', script: 'Latin', isRtl: false),
  LanguageInfo(code: 'ru', displayName: 'Russian', nativeName: 'Русский', script: 'Cyrillic', isRtl: false),
  LanguageInfo(code: 'zh', displayName: 'Chinese', nativeName: '中文', script: 'Han', isRtl: false),
  LanguageInfo(code: 'ja', displayName: 'Japanese', nativeName: '日本語', script: 'Japanese', isRtl: false),
  LanguageInfo(code: 'ko', displayName: 'Korean', nativeName: '한국어', script: 'Hangul', isRtl: false),
  LanguageInfo(code: 'ar', displayName: 'Arabic', nativeName: 'العربية', script: 'Arabic', isRtl: true),
  LanguageInfo(code: 'fa', displayName: 'Persian', nativeName: 'فارسی', script: 'Persian', isRtl: true),
  LanguageInfo(code: 'hi', displayName: 'Hindi', nativeName: 'हिन्दी', script: 'Devanagari', isRtl: false),
  LanguageInfo(code: 'tr', displayName: 'Turkish', nativeName: 'Türkçe', script: 'Latin', isRtl: false),
];

