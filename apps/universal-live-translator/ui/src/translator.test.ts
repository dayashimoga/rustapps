import { describe, it, expect, beforeEach } from 'vitest';
import {
  SUPPORTED_LANGUAGES,
  tauriInvoke,
  type TranslationRequest,
  type TranslationResponse,
  type LidResult,
  type PhrasebookItem,
} from './types';

describe('Universal Live Translator UI Tests', () => {
  beforeEach(() => {
    (globalThis as any).window = {
      __TAURI__: {
        invoke: async (cmd: string, args?: any) => {
          if (cmd === 'detect_language_cmd') {
            return { language: 'ru', confidence: 0.96 } as LidResult;
          }
          if (cmd === 'translate_text_cmd') {
            const req = args?.request as TranslationRequest;
            const res: TranslationResponse = {
              originalText: req.text,
              translatedText: `${req.text} (traducido)`,
              sourceLang: req.sourceLang || 'en',
              targetLang: req.targetLang,
              confidence: 0.98,
              latencyMs: 35,
              detectedSource: false,
              glossaryTermsApplied: [],
            };
            return res;
          }
          if (cmd === 'list_phrases_cmd') {
            const list: PhrasebookItem[] = [
              {
                id: '1',
                sourceText: 'Good morning',
                targetText: 'Buenos días',
                sourceLang: 'en',
                targetLang: 'es',
                category: 'Greeting',
                favorite: true,
                createdAt: Date.now(),
              },
            ];
            return list;
          }
          throw new Error(`Unhandled mock command: ${cmd}`);
        },
      },
    };
  });
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

