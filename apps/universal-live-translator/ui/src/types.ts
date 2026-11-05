// TypeScript models and IPC bridges for Universal Live Translator

export type LanguageCode =
  | 'en' | 'es' | 'fr' | 'de' | 'it' | 'pt' | 'nl' | 'ru'
  | 'uk' | 'pl' | 'zh' | 'ja' | 'ko' | 'ar' | 'fa' | 'hi'
  | 'bn' | 'ta' | 'te' | 'tr' | 'vi' | 'th' | 'id' | 'ms'
  | 'sw' | 'el' | 'he' | 'sv' | 'no' | 'da' | 'fi' | 'cs';

export interface LanguageInfo {
  code: LanguageCode;
  displayName: string;
  nativeName: string;
  script: string;
  isRtl: boolean;
}

export interface LidResult {
  language: LanguageCode;
  confidence: number;
  detectedScript: string;
}

export interface TranslationRequest {
  text: string;
  sourceLang?: LanguageCode | null;
  targetLang: LanguageCode;
  preserveFormatting?: boolean;
  domainGlossary?: string | null;
  recordHistory?: boolean;
}

export interface TranslationResponse {
  originalText: string;
  translatedText: string;
  sourceLang: LanguageCode;
  targetLang: LanguageCode;
  confidence: number;
  latencyMs: number;
  detectedSource: boolean;
  glossaryTermsApplied: string[];
}

export interface TtsSynthesisRequest {
  text: string;
  language: LanguageCode;
  speakingRate?: number;
  pitchSemitones?: number;
}

export interface TtsSynthesisResponse {
  sampleRate: number;
  channels: number;
  durationMs: number;
  audioWavBase64: string;
}

export interface SpeechRecognitionResponse {
  text: string;
  confidence: number;
  segments: Array<{
    start_ms: number;
    end_ms: number;
    text: string;
    confidence: number;
    is_final: boolean;
  }>;
}

export interface ConversationTurn {
  id: string;
  sessionId: string;
  speaker: string;
  originalText: string;
  translatedText: string;
  sourceLang: LanguageCode;
  targetLang: LanguageCode;
  timestamp: number;
}

export interface ConversationSession {
  id: string;
  partyALang: LanguageCode;
  partyBLang: LanguageCode;
  turns: ConversationTurn[];
  createdAt: number;
}

export type PhraseCategory =
  | 'Greeting'
  | 'Travel'
  | 'Dining'
  | 'Emergency'
  | 'Business'
  | 'Medical'
  | 'Custom';

export interface PhrasebookItem {
  id: string;
  category: PhraseCategory;
  sourceLang: LanguageCode;
  targetLang: LanguageCode;
  sourceText: string;
  targetText: string;
  notes?: string | null;
  favorite: boolean;
  createdAt: number;
}

export interface TranslationHistoryRecord {
  id: string;
  sourceLang: LanguageCode;
  targetLang: LanguageCode;
  sourceText: string;
  targetText: string;
  timestamp: number;
  favorite: boolean;
}

export interface GlossaryTerm {
  sourceTerm: string;
  targetTerm: string;
  caseSensitive: boolean;
}

export interface CameraOcrBox {
  x: number;
  y: number;
  width: number;
  height: number;
  originalText: string;
  translatedText: string;
  confidence: number;
}

export interface CameraTranslationResult {
  boxes: CameraOcrBox[];
  overlayImageBase64: string;
  sourceLang: LanguageCode;
  targetLang: LanguageCode;
  latencyMs: number;
}

export type SubtitleFormat = 'Srt' | 'Vtt';

export interface BatchDocumentItem {
  filename: string;
  content: string;
  format?: string;
}

export interface BatchTranslationResult {
  filename: string;
  translated_filename: string;
  translated_content: string;
  original_chars: number;
  translated_chars: number;
  elapsed_ms: number;
  success: boolean;
  error?: string | null;
}

// Standard 32 supported languages dictionary
export const SUPPORTED_LANGUAGES: LanguageInfo[] = [
  { code: 'en', displayName: 'English', nativeName: 'English', script: 'Latin', isRtl: false },
  { code: 'es', displayName: 'Spanish', nativeName: 'Español', script: 'Latin', isRtl: false },
  { code: 'fr', displayName: 'French', nativeName: 'Français', script: 'Latin', isRtl: false },
  { code: 'de', displayName: 'German', nativeName: 'Deutsch', script: 'Latin', isRtl: false },
  { code: 'it', displayName: 'Italian', nativeName: 'Italiano', script: 'Latin', isRtl: false },
  { code: 'pt', displayName: 'Portuguese', nativeName: 'Português', script: 'Latin', isRtl: false },
  { code: 'nl', displayName: 'Dutch', nativeName: 'Nederlands', script: 'Latin', isRtl: false },
  { code: 'ru', displayName: 'Russian', nativeName: 'Русский', script: 'Cyrillic', isRtl: false },
  { code: 'uk', displayName: 'Ukrainian', nativeName: 'Українська', script: 'Cyrillic', isRtl: false },
  { code: 'pl', displayName: 'Polish', nativeName: 'Polski', script: 'Latin', isRtl: false },
  { code: 'zh', displayName: 'Chinese (Simplified)', nativeName: '简体中文', script: 'Han', isRtl: false },
  { code: 'ja', displayName: 'Japanese', nativeName: '日本語', script: 'Hiragana/Katakana/Kanji', isRtl: false },
  { code: 'ko', displayName: 'Korean', nativeName: '한국어', script: 'Hangul', isRtl: false },
  { code: 'ar', displayName: 'Arabic', nativeName: 'العربية', script: 'Arabic', isRtl: true },
  { code: 'fa', displayName: 'Persian', nativeName: 'فارسی', script: 'Persian', isRtl: true },
  { code: 'hi', displayName: 'Hindi', nativeName: 'हिन्दी', script: 'Devanagari', isRtl: false },
  { code: 'bn', displayName: 'Bengali', nativeName: 'বাংলা', script: 'Bengali', isRtl: false },
  { code: 'ta', displayName: 'Tamil', nativeName: 'தமிழ்', script: 'Tamil', isRtl: false },
  { code: 'te', displayName: 'Telugu', nativeName: 'తెలుగు', script: 'Telugu', isRtl: false },
  { code: 'tr', displayName: 'Turkish', nativeName: 'Türkçe', script: 'Latin', isRtl: false },
  { code: 'vi', displayName: 'Vietnamese', nativeName: 'Tiếng Việt', script: 'Latin', isRtl: false },
  { code: 'th', displayName: 'Thai', nativeName: 'ไทย', script: 'Thai', isRtl: false },
  { code: 'id', displayName: 'Indonesian', nativeName: 'Bahasa Indonesia', script: 'Latin', isRtl: false },
  { code: 'ms', displayName: 'Malay', nativeName: 'Bahasa Melayu', script: 'Latin', isRtl: false },
  { code: 'sw', displayName: 'Swahili', nativeName: 'Kiswahili', script: 'Latin', isRtl: false },
  { code: 'el', displayName: 'Greek', nativeName: 'Ελληνικά', script: 'Greek', isRtl: false },
  { code: 'he', displayName: 'Hebrew', nativeName: 'עברית', script: 'Hebrew', isRtl: true },
  { code: 'sv', displayName: 'Swedish', nativeName: 'Svenska', script: 'Latin', isRtl: false },
  { code: 'no', displayName: 'Norwegian', nativeName: 'Norsk', script: 'Latin', isRtl: false },
  { code: 'da', displayName: 'Danish', nativeName: 'Dansk', script: 'Latin', isRtl: false },
  { code: 'fi', displayName: 'Finnish', nativeName: 'Suomi', script: 'Latin', isRtl: false },
  { code: 'cs', displayName: 'Czech', nativeName: 'Čeština', script: 'Latin', isRtl: false },
];

import { invokeIpc } from './lib/ipc';

export async function tauriInvoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  return await invokeIpc<T>(cmd, args);
}

