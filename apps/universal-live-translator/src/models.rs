#![forbid(unsafe_code)]
//! Domain Data Models for Universal Live Translator
//!
//! Provides typed representations for 30+ supported languages, translation
//! requests and responses, bidirectional conversation turns, live streaming ASR
//! segments, formant/neural TTS requests, camera OCR boxes, subtitle cues,
//! offline phrasebook items, and downloadable language pack manifests.

use serde::{Deserialize, Serialize};

/// Text reading direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
}

/// Unicode writing script family
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptType {
    Latin,
    Cyrillic,
    HanziCJK,
    Kana,
    Hangul,
    ArabicScript,
    Devanagari,
    TamilScript,
    TeluguScript,
    BengaliScript,
    GreekScript,
    HebrewScript,
    ThaiScript,
}

/// ISO-639-1 two-letter language code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LanguageCode {
    En, // English
    Es, // Spanish
    Fr, // French
    De, // German
    It, // Italian
    Pt, // Portuguese
    Ru, // Russian
    Zh, // Chinese (Simplified)
    Ja, // Japanese
    Ko, // Korean
    Ar, // Arabic
    Hi, // Hindi
    Tr, // Turkish
    Nl, // Dutch
    Pl, // Polish
    Sv, // Swedish
    Uk, // Ukrainian
    El, // Greek
    He, // Hebrew
    Vi, // Vietnamese
    Th, // Thai
    Id, // Indonesian
    Ms, // Malay
    Cs, // Czech
    Ro, // Romanian
    Da, // Danish
    Fi, // Finnish
    No, // Norwegian
    Hu, // Hungarian
    Bg, // Bulgarian
    Fa, // Persian
    Ur, // Urdu
    Ta, // Tamil
    Te, // Telugu
    Bn, // Bengali
}

impl LanguageCode {
    /// Two-letter ISO-639-1 code
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Es => "es",
            Self::Fr => "fr",
            Self::De => "de",
            Self::It => "it",
            Self::Pt => "pt",
            Self::Ru => "ru",
            Self::Zh => "zh",
            Self::Ja => "ja",
            Self::Ko => "ko",
            Self::Ar => "ar",
            Self::Hi => "hi",
            Self::Tr => "tr",
            Self::Nl => "nl",
            Self::Pl => "pl",
            Self::Sv => "sv",
            Self::Uk => "uk",
            Self::El => "el",
            Self::He => "he",
            Self::Vi => "vi",
            Self::Th => "th",
            Self::Id => "id",
            Self::Ms => "ms",
            Self::Cs => "cs",
            Self::Ro => "ro",
            Self::Da => "da",
            Self::Fi => "fi",
            Self::No => "no",
            Self::Hu => "hu",
            Self::Bg => "bg",
            Self::Fa => "fa",
            Self::Ur => "ur",
            Self::Ta => "ta",
            Self::Te => "te",
            Self::Bn => "bn",
        }
    }

    /// English display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Es => "Spanish",
            Self::Fr => "French",
            Self::De => "German",
            Self::It => "Italian",
            Self::Pt => "Portuguese",
            Self::Ru => "Russian",
            Self::Zh => "Chinese (Simplified)",
            Self::Ja => "Japanese",
            Self::Ko => "Korean",
            Self::Ar => "Arabic",
            Self::Hi => "Hindi",
            Self::Tr => "Turkish",
            Self::Nl => "Dutch",
            Self::Pl => "Polish",
            Self::Sv => "Swedish",
            Self::Uk => "Ukrainian",
            Self::El => "Greek",
            Self::He => "Hebrew",
            Self::Vi => "Vietnamese",
            Self::Th => "Thai",
            Self::Id => "Indonesian",
            Self::Ms => "Malay",
            Self::Cs => "Czech",
            Self::Ro => "Romanian",
            Self::Da => "Danish",
            Self::Fi => "Finnish",
            Self::No => "Norwegian",
            Self::Hu => "Hungarian",
            Self::Bg => "Bulgarian",
            Self::Fa => "Persian",
            Self::Ur => "Urdu",
            Self::Ta => "Tamil",
            Self::Te => "Telugu",
            Self::Bn => "Bengali",
        }
    }

    /// Autonym / Native language name
    pub fn native_name(&self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Es => "Español",
            Self::Fr => "Français",
            Self::De => "Deutsch",
            Self::It => "Italiano",
            Self::Pt => "Português",
            Self::Ru => "Русский",
            Self::Zh => "中文",
            Self::Ja => "日本語",
            Self::Ko => "한국어",
            Self::Ar => "العربية",
            Self::Hi => "हिन्दी",
            Self::Tr => "Türkçe",
            Self::Nl => "Nederlands",
            Self::Pl => "Polski",
            Self::Sv => "Svenska",
            Self::Uk => "Українська",
            Self::El => "Ελληνικά",
            Self::He => "עברית",
            Self::Vi => "Tiếng Việt",
            Self::Th => "ไทย",
            Self::Id => "Bahasa Indonesia",
            Self::Ms => "Bahasa Melayu",
            Self::Cs => "Čeština",
            Self::Ro => "Română",
            Self::Da => "Dansk",
            Self::Fi => "Suomi",
            Self::No => "Norsk",
            Self::Hu => "Magyar",
            Self::Bg => "Български",
            Self::Fa => "فارسی",
            Self::Ur => "اردو",
            Self::Ta => "தமிழ்",
            Self::Te => "తెలుగు",
            Self::Bn => "বাংলা",
        }
    }

    /// Writing script family
    pub fn script(&self) -> ScriptType {
        match self {
            Self::En
            | Self::Es
            | Self::Fr
            | Self::De
            | Self::It
            | Self::Pt
            | Self::Tr
            | Self::Nl
            | Self::Pl
            | Self::Sv
            | Self::Vi
            | Self::Id
            | Self::Ms
            | Self::Cs
            | Self::Ro
            | Self::Da
            | Self::Fi
            | Self::No
            | Self::Hu => ScriptType::Latin,

            Self::Ru | Self::Uk | Self::Bg => ScriptType::Cyrillic,
            Self::Zh => ScriptType::HanziCJK,
            Self::Ja => ScriptType::Kana,
            Self::Ko => ScriptType::Hangul,
            Self::Ar | Self::Fa | Self::Ur => ScriptType::ArabicScript,
            Self::Hi => ScriptType::Devanagari,
            Self::Ta => ScriptType::TamilScript,
            Self::Te => ScriptType::TeluguScript,
            Self::Bn => ScriptType::BengaliScript,
            Self::El => ScriptType::GreekScript,
            Self::He => ScriptType::HebrewScript,
            Self::Th => ScriptType::ThaiScript,
        }
    }

    /// Text reading direction
    pub fn direction(&self) -> TextDirection {
        match self {
            Self::Ar | Self::He | Self::Fa | Self::Ur => TextDirection::RightToLeft,
            _ => TextDirection::LeftToRight,
        }
    }

    /// Parse code from case-insensitive string (e.g. "en", "ES", "french")
    pub fn parse(s: &str) -> Option<Self> {
        let clean = s.trim().to_lowercase();
        match clean.as_str() {
            "en" | "eng" | "english" => Some(Self::En),
            "es" | "spa" | "spanish" | "español" => Some(Self::Es),
            "fr" | "fra" | "fre" | "french" | "français" => Some(Self::Fr),
            "de" | "deu" | "ger" | "german" | "deutsch" => Some(Self::De),
            "it" | "ita" | "italian" | "italiano" => Some(Self::It),
            "pt" | "por" | "portuguese" | "português" => Some(Self::Pt),
            "ru" | "rus" | "russian" | "русский" => Some(Self::Ru),
            "zh" | "zho" | "chi" | "chinese" | "中文" => Some(Self::Zh),
            "ja" | "jpn" | "japanese" | "日本語" => Some(Self::Ja),
            "ko" | "kor" | "korean" | "한국어" => Some(Self::Ko),
            "ar" | "ara" | "arabic" | "العربية" => Some(Self::Ar),
            "hi" | "hin" | "hindi" | "हिन्दी" => Some(Self::Hi),
            "tr" | "tur" | "turkish" | "türkçe" => Some(Self::Tr),
            "nl" | "nld" | "dut" | "dutch" | "nederlands" => Some(Self::Nl),
            "pl" | "pol" | "polish" | "polski" => Some(Self::Pl),
            "sv" | "swe" | "swedish" | "svenska" => Some(Self::Sv),
            "uk" | "ukr" | "ukrainian" | "українська" => Some(Self::Uk),
            "el" | "ell" | "gre" | "greek" | "ελληνικά" => Some(Self::El),
            "he" | "heb" | "hebrew" | "עברית" => Some(Self::He),
            "vi" | "vie" | "vietnamese" | "tiếng việt" => Some(Self::Vi),
            "th" | "tha" | "thai" | "ไทย" => Some(Self::Th),
            "id" | "ind" | "indonesian" => Some(Self::Id),
            "ms" | "msa" | "may" | "malay" => Some(Self::Ms),
            "cs" | "ces" | "cze" | "czech" | "čeština" => Some(Self::Cs),
            "ro" | "ron" | "rum" | "romanian" | "română" => Some(Self::Ro),
            "da" | "dan" | "danish" | "dansk" => Some(Self::Da),
            "fi" | "fin" | "finnish" | "suomi" => Some(Self::Fi),
            "no" | "nor" | "norwegian" | "norsk" => Some(Self::No),
            "hu" | "hun" | "hungarian" | "magyar" => Some(Self::Hu),
            "bg" | "bul" | "bulgarian" | "български" => Some(Self::Bg),
            "fa" | "fas" | "per" | "persian" | "فارسی" => Some(Self::Fa),
            "ur" | "urd" | "urdu" | "اردو" => Some(Self::Ur),
            "ta" | "tam" | "tamil" | "தமிழ்" => Some(Self::Ta),
            "te" | "tel" | "telugu" | "తెలుగు" => Some(Self::Te),
            "bn" | "ben" | "bengali" | "বাংলা" => Some(Self::Bn),
            _ => None,
        }
    }

    /// List of all supported language codes
    pub fn all() -> &'static [LanguageCode] {
        &[
            Self::En,
            Self::Es,
            Self::Fr,
            Self::De,
            Self::It,
            Self::Pt,
            Self::Ru,
            Self::Zh,
            Self::Ja,
            Self::Ko,
            Self::Ar,
            Self::Hi,
            Self::Tr,
            Self::Nl,
            Self::Pl,
            Self::Sv,
            Self::Uk,
            Self::El,
            Self::He,
            Self::Vi,
            Self::Th,
            Self::Id,
            Self::Ms,
            Self::Cs,
            Self::Ro,
            Self::Da,
            Self::Fi,
            Self::No,
            Self::Hu,
            Self::Bg,
            Self::Fa,
            Self::Ur,
            Self::Ta,
            Self::Te,
            Self::Bn,
        ]
    }
}

/// Metadata description for a supported language
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub code: LanguageCode,
    pub display_name: String,
    pub native_name: String,
    pub script: ScriptType,
    pub direction: TextDirection,
    pub speech_supported: bool,
    pub ocr_supported: bool,
}

impl From<LanguageCode> for LanguageInfo {
    fn from(c: LanguageCode) -> Self {
        Self {
            code: c,
            display_name: c.display_name().to_string(),
            native_name: c.native_name().to_string(),
            script: c.script(),
            direction: c.direction(),
            speech_supported: matches!(
                c,
                LanguageCode::En
                    | LanguageCode::Es
                    | LanguageCode::Fr
                    | LanguageCode::De
                    | LanguageCode::It
                    | LanguageCode::Pt
                    | LanguageCode::Ru
                    | LanguageCode::Zh
                    | LanguageCode::Ja
                    | LanguageCode::Ko
                    | LanguageCode::Hi
                    | LanguageCode::Ar
            ),
            ocr_supported: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Translation Request & Response
// ---------------------------------------------------------------------------

/// Text translation request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranslationRequest {
    /// Source language code, or None for automatic language detection
    pub source_lang: Option<LanguageCode>,
    /// Target language code
    pub target_lang: LanguageCode,
    /// Raw input text to translate
    pub text: String,
    /// Whether to preserve whitespace and punctuation formatting
    pub preserve_formatting: bool,
    /// Optional domain glossary name (e.g. "medical", "legal", "devops")
    pub domain_glossary: Option<String>,
}

/// Text translation response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranslationResponse {
    pub source_lang: LanguageCode,
    pub target_lang: LanguageCode,
    pub original_text: String,
    pub translated_text: String,
    pub confidence: f32,
    pub detected_source: bool,
    pub latency_ms: f64,
    pub word_count: usize,
}

// ---------------------------------------------------------------------------
// Conversation Mode
// ---------------------------------------------------------------------------

/// Individual dialogue turn in a split-screen conversation session
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub id: String,
    pub speaker: String, // e.g. "speaker_a", "speaker_b"
    pub source_lang: LanguageCode,
    pub target_lang: LanguageCode,
    pub original_text: String,
    pub translated_text: String,
    pub timestamp_secs: u64,
    pub audio_duration_ms: Option<u32>,
}

/// Conversation session metadata and history
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationSession {
    pub id: String,
    pub title: String,
    pub party_a_lang: LanguageCode,
    pub party_b_lang: LanguageCode,
    pub created_at_secs: u64,
    pub updated_at_secs: u64,
    pub turns: Vec<ConversationTurn>,
}

// ---------------------------------------------------------------------------
// Speech & Audio Processing
// ---------------------------------------------------------------------------

/// Streaming transcription segment from offline ASR engine
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub start_ms: u32,
    pub end_ms: u32,
    pub text: String,
    pub confidence: f32,
    pub is_final: bool,
}

/// Real-time speech event emitted during streaming microphone capture
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeechRecognitionEvent {
    pub segment: TranscriptSegment,
    pub is_speech_active: bool,
}

/// Formant / neural TTS speech synthesis request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TtsSynthesisRequest {
    pub text: String,
    pub language: LanguageCode,
    pub speaking_rate: f32, // 0.5 to 2.0 (default 1.0)
    pub pitch: f32,         // 0.5 to 2.0 (default 1.0)
    pub sample_rate: u32,   // 16000 or 22050
}

impl Default for TtsSynthesisRequest {
    fn default() -> Self {
        Self {
            text: String::new(),
            language: LanguageCode::En,
            speaking_rate: 1.0,
            pitch: 1.0,
            sample_rate: 22050,
        }
    }
}

/// TTS synthesis audio buffer output
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TtsSynthesisResponse {
    pub sample_rate: u32,
    pub duration_secs: f32,
    pub wav_bytes: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Camera OCR Overlay Translation
// ---------------------------------------------------------------------------

/// Bounding rectangle for OCR region on camera frame
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraOcrBox {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub original_text: String,
    pub translated_text: String,
    pub confidence: f32,
}

/// Full camera translation frame analysis result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraTranslationResult {
    pub image_width: u32,
    pub image_height: u32,
    pub source_lang: LanguageCode,
    pub target_lang: LanguageCode,
    pub boxes: Vec<CameraOcrBox>,
    pub processing_time_ms: f64,
}

// ---------------------------------------------------------------------------
// Subtitles & Documents
// ---------------------------------------------------------------------------

/// Subtitle time-stamped cue
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubtitleCue {
    pub index: usize,
    pub start_ms: u64,
    pub end_ms: u64,
    pub original_text: String,
    pub translated_text: String,
}

/// Subtitle format specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubtitleFormat {
    Srt,
    Vtt,
}

/// Single document item for batch translation (.txt, .md, .srt, .vtt)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchDocumentItem {
    pub filename: String,
    pub content: String,
    pub format: Option<String>,
}

/// Output result for an item in batch translation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchTranslationResult {
    pub filename: String,
    pub translated_filename: String,
    pub translated_content: String,
    pub original_chars: usize,
    pub translated_chars: usize,
    pub elapsed_ms: f64,
    pub success: bool,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Phrasebook & Glossary
// ---------------------------------------------------------------------------

/// Predefined or custom phrasebook categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhraseCategory {
    Greetings,
    Travel,
    Dining,
    Emergency,
    Medical,
    Business,
    Technical,
    Custom,
}

/// Offline stored phrasebook entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhrasebookItem {
    pub id: String,
    pub category: PhraseCategory,
    pub source_text: String,
    pub source_lang: LanguageCode,
    pub target_text: String,
    pub target_lang: LanguageCode,
    pub notes: Option<String>,
    pub is_favorite: bool,
    pub created_at_secs: u64,
}

/// Domain terminology glossary replacement entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlossaryTerm {
    pub source_term: String,
    pub target_term: String,
    pub case_sensitive: bool,
}

// ---------------------------------------------------------------------------
// Model Hub & Offline Language Packs
// ---------------------------------------------------------------------------

/// Metadata for offline language packs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguagePackInfo {
    pub id: String,
    pub pair_name: String, // e.g. "English - Spanish"
    pub source_code: LanguageCode,
    pub target_code: LanguageCode,
    pub size_bytes: u64,
    pub is_installed: bool,
    pub version: String,
    pub sha256_hash: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_languages_have_valid_metadata() {
        let all = LanguageCode::all();
        assert_eq!(all.len(), 35);

        for lang in all {
            let code_str = lang.as_str();
            assert_eq!(code_str.len(), 2);
            assert_eq!(LanguageCode::parse(code_str), Some(*lang));
            assert!(!lang.display_name().is_empty());
            assert!(!lang.native_name().is_empty());
        }
    }

    #[test]
    fn test_rtl_detection() {
        assert_eq!(LanguageCode::Ar.direction(), TextDirection::RightToLeft);
        assert_eq!(LanguageCode::He.direction(), TextDirection::RightToLeft);
        assert_eq!(LanguageCode::Fa.direction(), TextDirection::RightToLeft);
        assert_eq!(LanguageCode::Ur.direction(), TextDirection::RightToLeft);
        assert_eq!(LanguageCode::En.direction(), TextDirection::LeftToRight);
        assert_eq!(LanguageCode::Zh.direction(), TextDirection::LeftToRight);
    }

    #[test]
    fn test_script_classification() {
        assert_eq!(LanguageCode::Zh.script(), ScriptType::HanziCJK);
        assert_eq!(LanguageCode::Ja.script(), ScriptType::Kana);
        assert_eq!(LanguageCode::Ko.script(), ScriptType::Hangul);
        assert_eq!(LanguageCode::Ru.script(), ScriptType::Cyrillic);
        assert_eq!(LanguageCode::El.script(), ScriptType::GreekScript);
        assert_eq!(LanguageCode::Hi.script(), ScriptType::Devanagari);
        assert_eq!(LanguageCode::Th.script(), ScriptType::ThaiScript);
        assert_eq!(LanguageCode::En.script(), ScriptType::Latin);
    }

    #[test]
    fn test_language_info_conversion() {
        let info = LanguageInfo::from(LanguageCode::Es);
        assert_eq!(info.code, LanguageCode::Es);
        assert_eq!(info.display_name, "Spanish");
        assert_eq!(info.native_name, "Español");
        assert_eq!(info.direction, TextDirection::LeftToRight);
        assert!(info.speech_supported);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let req = TranslationRequest {
            source_lang: Some(LanguageCode::En),
            target_lang: LanguageCode::Es,
            text: "Hello World".to_string(),
            preserve_formatting: true,
            domain_glossary: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        let decoded: TranslationRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, decoded);
    }
}
