import { describe, it, expect } from 'vitest';
import {
  SUPPORTED_LANGUAGES,
  tauriInvoke,
  type TranslationRequest,
  type TranslationResponse,
  type LidResult,
  type PhrasebookItem,
} from './types';

describe('Universal Live Translator UI Tests', () => {
  it('should include 32 supported languages with valid metadata', () => {
    expect(SUPPORTED_LANGUAGES.length).toBe(32);

    const codes = new Set(SUPPORTED_LANGUAGES.map((l) => l.code));
    expect(codes.has('en')).toBe(true);
    expect(codes.has('es')).toBe(true);
    expect(codes.has('zh')).toBe(true);
    expect(codes.has('ar')).toBe(true);
    expect(codes.has('ja')).toBe(true);
    expect(codes.has('de')).toBe(true);
    expect(codes.has('hi')).toBe(true);

    const arabic = SUPPORTED_LANGUAGES.find((l) => l.code === 'ar');
    expect(arabic?.isRtl).toBe(true);
    const persian = SUPPORTED_LANGUAGES.find((l) => l.code === 'fa');
    expect(persian?.isRtl).toBe(true);
    const english = SUPPORTED_LANGUAGES.find((l) => l.code === 'en');
    expect(english?.isRtl).toBe(false);
  });

  it('should detect language via offline fallback invoke', async () => {
    const res = await tauriInvoke<LidResult>('detect_language_cmd', {
      text: 'Привет мир',
    });
    expect(res).toBeDefined();
    expect(res.language).toBe('ru');
    expect(res.confidence).toBeGreaterThan(0.9);
  });

  it('should translate text via offline fallback invoke', async () => {
    const req: TranslationRequest = {
      text: 'Good morning',
      sourceLang: 'en',
      targetLang: 'es',
    };
    const res = await tauriInvoke<TranslationResponse>('translate_text_cmd', {
      request: req,
    });
    expect(res).toBeDefined();
    expect(res.translatedText).toContain('Good morning');
    expect(res.targetLang).toBe('es');
    expect(res.latencyMs).toBeLessThan(150);
  });

  it('should retrieve offline phrasebook list via fallback invoke', async () => {
    const phrases = await tauriInvoke<PhrasebookItem[]>('list_phrases_cmd', {
      categoryStr: null,
      favoriteOnly: false,
    });
    expect(phrases.length).toBeGreaterThan(0);
    expect(phrases[0]!.sourceLang).toBe('en');
    expect(phrases[0]!.targetLang).toBe('es');
    expect(phrases[0]!.favorite).toBe(true);
  });
});

