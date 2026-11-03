import 'package:flutter_test/flutter_test.dart';
import 'package:universal_live_translator_mobile/models/translation_models.dart';
import 'package:universal_live_translator_mobile/services/ult_service.dart';

void main() {
  group('Universal Live Translator Mobile Unit Tests', () {
    test('Supported languages dictionary sanity check', () {
      expect(supportedLanguagesList.length, greaterThanOrEqualTo(15));
      final en = supportedLanguagesList.firstWhere((l) => l.code == 'en');
      expect(en.displayName, 'English');
      expect(en.isRtl, false);

      final ar = supportedLanguagesList.firstWhere((l) => l.code == 'ar');
      expect(ar.displayName, 'Arabic');
      expect(ar.isRtl, true);
    });

    test('TranslationRequest and Response JSON serialization', () {
      final req = TranslationRequest(
        text: 'Hello world',
        sourceLang: 'en',
        targetLang: 'es',
      );
      final json = req.toJson();
      expect(json['text'], 'Hello world');
      expect(json['source_lang'], 'en');
      expect(json['target_lang'], 'es');

      final respJson = {
        'original_text': 'Hello world',
        'translated_text': 'Hola mundo',
        'source_lang': 'en',
        'target_lang': 'es',
        'confidence': 0.98,
        'latency_ms': 12.4,
        'detected_source': false,
        'glossary_terms_applied': ['hello'],
      };

      final resp = TranslationResponse.fromJson(respJson);
      expect(resp.translatedText, 'Hola mundo');
      expect(resp.latencyMs, lessThan(150.0));
      expect(resp.glossaryTermsApplied, contains('hello'));
    });

    test('ConversationSession and Turn parsing', () {
      final turnJson = {
        'id': 'turn-1',
        'session_id': 'sess-1',
        'speaker': 'party_a',
        'original_text': 'Good morning',
        'translated_text': 'Buenos días',
        'source_lang': 'en',
        'target_lang': 'es',
        'timestamp': 1700000000,
      };

      final turn = ConversationTurn.fromJson(turnJson);
      expect(turn.id, 'turn-1');
      expect(turn.speaker, 'party_a');
      expect(turn.translatedText, 'Buenos días');

      final sessionJson = {
        'id': 'sess-1',
        'party_a_lang': 'en',
        'party_b_lang': 'es',
        'turns': [turnJson],
        'created_at': 1700000000,
      };

      final session = ConversationSession.fromJson(sessionJson);
      expect(session.turns.length, 1);
      expect(session.turns.first.translatedText, 'Buenos días');
    });

    test('UltService fallback in test environment', () async {
      final service = UltService();
      await service.initialize();

      // In unit test environment without loading dynamic library, fallback is triggered
      final lid = await service.detectLanguage('Hola amigo');
      expect(lid, isNotNull);
      expect(lid!.language, 'es');

      final req = TranslationRequest(text: 'Welcome', targetLang: 'es');
      final resp = await service.translateText(req);
      expect(resp.translatedText, contains('Welcome'));
      expect(resp.latencyMs, lessThan(150.0));

      final phrases = await service.listPhrases();
      expect(phrases.isNotEmpty, true);
    });
  });
}

