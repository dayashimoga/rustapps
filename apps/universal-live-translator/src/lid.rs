#![forbid(unsafe_code)]
//! Ultra-Fast Offline Automatic Language Identification (LID)
//!
//! Evaluates input text and accurately identifies the language among 32+ supported
//! languages in $<500\,\mu\text{s}$ with zero network egress.
//!
//! Employs a 2-tier classifier:
//! 1. Unicode Script & Orthographic Range Classifier (CJK, Kana, Hangul, Arabic,
//!    Devanagari, Cyrillic, Greek, Hebrew, Thai)
//! 2. Character Trigram Profile Frequency & Morpheme Scorer for Latin script variants.

use crate::models::{LanguageCode, ScriptType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of Automatic Language Identification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LidResult {
    pub language: LanguageCode,
    pub confidence: f32,
    pub script: ScriptType,
}

/// Offline Language Identifier Engine
pub struct LanguageDetector {
    latin_profiles: HashMap<LanguageCode, &'static [&'static str]>,
    stopwords: HashMap<LanguageCode, &'static [&'static str]>,
}

impl Default for LanguageDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageDetector {
    /// Initialize offline language detector with precomputed trigrams and stopwords
    pub fn new() -> Self {
        let mut latin_profiles = HashMap::new();
        let mut stopwords = HashMap::new();

        // Distinctive Character Trigrams for Latin Script Languages
        latin_profiles.insert(
            LanguageCode::En,
            &[
                "the", "and", "ing", "ion", "tion", "tha", "for", "wit", "ith", "all", "not",
                "ver", "ter", "ere", "ers", "ati", "ent", "ate", "rea",
            ][..],
        );
        latin_profiles.insert(
            LanguageCode::Es,
            &[
                "de ", "la ", "que", " el", " en", "los", "del", "las", "por", "con", "una", "est",
                "par", "ara", "aci", "ado", "nte", "ion", "oso",
            ][..],
        );
        latin_profiles.insert(
            LanguageCode::Fr,
            &[
                "de ", "le ", "et ", "la ", "les", "des", "en ", "que", "qui", "dan", "pou", "our",
                "pas", "une", "par", "sur", "ent", "est", "ait", "tio",
            ][..],
        );
        latin_profiles.insert(
            LanguageCode::De,
            &[
                "der", "die", "und", "den", "das", "von", "zu ", "mit", "ist", "für", "ein", "nic",
                "ich", "cht", "auf", "ine", "sch", "che", "ter", "gen",
            ][..],
        );
        latin_profiles.insert(
            LanguageCode::It,
            &[
                "di ", "il ", "che", " la", "per", " un", "con", "del", "son", "gli", "nel", "all",
                "non", "ell", "men", "ett", "zio", "ato", "ita", "que",
            ][..],
        );
        latin_profiles.insert(
            LanguageCode::Pt,
            &[
                "de ", "que", "do ", "da ", "em ", "um ", "par", "com", "não", "uma", "os ", "no ",
                "est", "ado", "ões", "nte", "ara", "ida", "cia",
            ][..],
        );
        latin_profiles.insert(
            LanguageCode::Nl,
            &[
                "van", "het", "de ", "een", "en ", "is ", "in ", "voo", "oor", "op ", "met", "die",
                "zijn", "erd", "aar", "ede", "cht", "ver", "gen",
            ][..],
        );
        latin_profiles.insert(
            LanguageCode::Sv,
            &[
                "och", "det", "att", "som", "en ", "på ", "är ", "av ", "för", "med", "til", "ill",
                "int", "nte", "nde", "ing", "ade", "ska",
            ][..],
        );
        latin_profiles.insert(
            LanguageCode::Pl,
            &[
                "nie", "się", "prz", "rze", "czy", "nie", "jest", "sta", "row", "prz", "prz",
                "ych", "ach", "ego", "nia", "dzi", "sko",
            ][..],
        );
        latin_profiles.insert(
            LanguageCode::Tr,
            &[
                "bir", "ve ", "bu ", "içi", "çin", "ile", "ola", "lar", "ler", "den", "dan", "nin",
                "nin", "yor", "dir", "mak", "mek",
            ][..],
        );
        latin_profiles.insert(
            LanguageCode::Vi,
            &[
                "ngh", "ngư", "ười", "tro", "ong", "nhữ", "ững", "khô", "ông", "đượ", "ược", "cho",
                "việ", "iệc", "này", "với",
            ][..],
        );
        latin_profiles.insert(
            LanguageCode::Id,
            &[
                "yan", "ang", "dan", "di ", "dar", "ari", "ini", "unt", "ntu", "tuk", "pad", "ada",
                "dah", "den", "eng", "nga", "gan", "ber",
            ][..],
        );
        latin_profiles.insert(
            LanguageCode::Ms,
            &[
                "yan", "ang", "dan", "di ", "dar", "ari", "ini", "unt", "tuk", "pad", "ada", "ada",
                "den", "gan", "ber", "ter", "kan",
            ][..],
        );
        latin_profiles.insert(
            LanguageCode::Cs,
            &[
                "pro", "kter", "jso", "sou", "byl", "byl", "neb", "ebo", "vše", "pře", "řed",
                "pod", "od ", "do ", "že ",
            ][..],
        );
        latin_profiles.insert(
            LanguageCode::Ro,
            &[
                "car", "are", "pen", "ntr", "tru", "est", "ste", "din", "pen", "tru", "lui", "lor",
                "ilo", "lor", "pri", "rin", "tre",
            ][..],
        );
        latin_profiles.insert(
            LanguageCode::Da,
            &[
                "og ", "i  ", "det", "på ", "for", "at ", "en ", "til", "er ", "som", "der", "med",
                "ikk", "kke", "hav", "ave",
            ][..],
        );
        latin_profiles.insert(
            LanguageCode::Fi,
            &[
                "ja ", "on ", "ei ", "oli", "ett", "ttä", "hän", "se ", "kun", "mut", "tta", "nii",
                "iin", "kui", "uin", "ssa", "ssä", "lla", "llä",
            ][..],
        );
        latin_profiles.insert(
            LanguageCode::No,
            &[
                "og ", "i  ", "det", "på ", "for", "at ", "en ", "til", "er ", "som", "ikk", "kke",
                "med", "har", "var", "av ", "om ",
            ][..],
        );
        latin_profiles.insert(
            LanguageCode::Hu,
            &[
                "hogy", "nem", "egy", "meg", "van", "volt", "vagy", "mint", "csak", "lett", "után",
                "arra", "ezek", "volt", "sze", "int",
            ][..],
        );

        // Frequent High-Confidence Stopwords & Core Vocabulary Lexicons
        stopwords.insert(
            LanguageCode::En,
            &[
                "the", "and", "is", "in", "it", "you", "that", "he", "was", "for", "on", "are",
                "with", "as", "his", "they", "be", "at", "one", "have", "this", "from", "hello",
                "world", "thank", "thanks", "please", "good", "morning", "evening", "night",
                "where", "what", "when", "why", "how", "much", "many", "help", "water", "food",
                "station", "airport", "hotel", "hospital", "doctor", "yes", "welcome", "friend",
            ][..],
        );
        stopwords.insert(
            LanguageCode::Es,
            &[
                "de",
                "la",
                "que",
                "el",
                "en",
                "y",
                "los",
                "del",
                "se",
                "las",
                "por",
                "un",
                "para",
                "con",
                "una",
                "su",
                "al",
                "lo",
                "como",
                "más",
                "pero",
                "hola",
                "mundo",
                "gracias",
                "muchas",
                "muchos",
                "mucho",
                "mucha",
                "favor",
                "buenos",
                "buenas",
                "días",
                "dias",
                "tardes",
                "noches",
                "cómo",
                "estás",
                "estas",
                "está",
                "esta",
                "dónde",
                "donde",
                "cuánto",
                "cuanto",
                "cuesta",
                "ayuda",
                "agua",
                "comida",
                "amigo",
                "amiga",
                "baño",
                "bano",
                "aeropuerto",
                "estación",
                "estacion",
                "médico",
                "medico",
                "bienvenido",
                "sí",
                "si",
                "señor",
                "senor",
                "tiempo",
            ][..],
        );
        stopwords.insert(
            LanguageCode::Fr,
            &[
                "de",
                "la",
                "le",
                "et",
                "les",
                "des",
                "en",
                "un",
                "du",
                "une",
                "que",
                "est",
                "pour",
                "qui",
                "dans",
                "par",
                "plus",
                "pas",
                "au",
                "sur",
                "ne",
                "bonjour",
                "bonsoir",
                "monde",
                "merci",
                "beaucoup",
                "oui",
                "non",
                "comment",
                "allez",
                "vous",
                "où",
                "gare",
                "aéroport",
                "eau",
                "aide",
                "ami",
                "bienvenue",
                "s'il",
                "plaît",
                "plait",
                "combien",
                "ça",
                "va",
                "très",
                "tres",
                "bien",
            ][..],
        );
        stopwords.insert(
            LanguageCode::De,
            &[
                "der",
                "die",
                "und",
                "in",
                "den",
                "von",
                "zu",
                "das",
                "mit",
                "sich",
                "des",
                "auf",
                "für",
                "ist",
                "im",
                "dem",
                "nicht",
                "ein",
                "eine",
                "als",
                "auch",
                "es",
                "an",
                "hallo",
                "welt",
                "guten",
                "morgen",
                "abend",
                "tag",
                "nacht",
                "danke",
                "vielen",
                "dank",
                "bitte",
                "ja",
                "nein",
                "wie",
                "geht",
                "wo",
                "bahnhof",
                "flughafen",
                "wasser",
                "essen",
                "hilfe",
                "freund",
                "willkommen",
            ][..],
        );
        stopwords.insert(
            LanguageCode::It,
            &[
                "di",
                "e",
                "il",
                "che",
                "la",
                "in",
                "per",
                "un",
                "del",
                "le",
                "non",
                "da",
                "si",
                "della",
                "dei",
                "delle",
                "al",
                "sono",
                "con",
                "gli",
                "nel",
                "ciao",
                "mondo",
                "buongiorno",
                "buonasera",
                "grazie",
                "mille",
                "prego",
                "perché",
                "come",
                "stai",
                "dove",
                "stazione",
                "aeroporto",
                "acqua",
                "cibo",
                "aiuto",
                "amico",
                "benvenuto",
                "quanto",
                "costa",
                "molto",
            ][..],
        );
        stopwords.insert(
            LanguageCode::Pt,
            &[
                "de",
                "o",
                "que",
                "do",
                "da",
                "em",
                "um",
                "para",
                "é",
                "com",
                "não",
                "nao",
                "uma",
                "os",
                "no",
                "se",
                "na",
                "por",
                "mais",
                "as",
                "dos",
                "como",
                "olá",
                "ola",
                "oi",
                "bom",
                "dia",
                "boa",
                "tarde",
                "noite",
                "obrigado",
                "obrigada",
                "muito",
                "muita",
                "muitos",
                "muitas",
                "você",
                "voce",
                "onde",
                "fica",
                "estação",
                "água",
                "comida",
                "ajuda",
                "bem-vindo",
            ][..],
        );
        stopwords.insert(
            LanguageCode::Nl,
            &[
                "van",
                "de",
                "en",
                "het",
                "een",
                "in",
                "is",
                "op",
                "te",
                "dat",
                "die",
                "voor",
                "met",
                "zijn",
                "er",
                "niet",
                "aan",
                "om",
                "ook",
                "als",
                "bij",
                "goedemorgen",
                "goedenavond",
                "bedankt",
                "dank",
                "alstublieft",
                "waar",
                "water",
                "welkom",
            ][..],
        );
        stopwords.insert(
            LanguageCode::Pl,
            &[
                "w",
                "z",
                "i",
                "na",
                "do",
                "nie",
                "się",
                "o",
                "to",
                "jak",
                "że",
                "od",
                "po",
                "dla",
                "przez",
                "co",
                "za",
                "ale",
                "tak",
                "jest",
                "cześć",
                "dzień",
                "dobry",
                "dziękuję",
                "bardzo",
                "proszę",
                "gdzie",
                "woda",
                "pomoc",
            ][..],
        );
        stopwords.insert(
            LanguageCode::Tr,
            &[
                "bir",
                "ve",
                "bu",
                "da",
                "de",
                "için",
                "ile",
                "çok",
                "daha",
                "en",
                "kadar",
                "olan",
                "gibi",
                "olarak",
                "var",
                "sonra",
                "ancak",
                "merhaba",
                "dünya",
                "teşekkürler",
                "teşekkür",
                "ederim",
                "lütfen",
                "evet",
                "hayır",
                "nasılsın",
                "nerede",
                "su",
                "yardım",
                "hoş",
                "geldiniz",
            ][..],
        );
        stopwords.insert(
            LanguageCode::Vi,
            &[
                "và", "của", "là", "có", "trong", "người", "không", "cho", "được", "các", "với",
                "đã", "một", "này", "khi", "những", "xin", "chào", "cảm", "ơn", "vâng", "tôi",
                "bạn", "nước", "giúp",
            ][..],
        );
        stopwords.insert(
            LanguageCode::Id,
            &[
                "dan", "di", "yang", "dari", "untuk", "ini", "pada", "ke", "adalah", "dengan",
                "dalam", "tidak", "akan", "juga", "itu", "halo", "dunia", "terima", "kasih",
                "tolong", "selamat", "pagi", "malam", "dimana", "air", "makanan", "bantuan",
            ][..],
        );
        stopwords.insert(
            LanguageCode::Cs,
            &[
                "a", "v", "se", "na", "že", "s", "z", "do", "pro", "je", "o", "k", "i", "když",
                "tak", "jako", "ale", "po", "byl", "ahoj", "dobrý", "den", "děkuji", "prosím",
            ][..],
        );
        stopwords.insert(
            LanguageCode::Ro,
            &[
                "de",
                "și",
                "în",
                "a",
                "la",
                "care",
                "cu",
                "o",
                "pe",
                "pentru",
                "din",
                "ce",
                "nu",
                "sunt",
                "mai",
                "este",
                "sau",
                "salut",
                "mulțumesc",
                "vă",
                "rog",
                "bună",
                "ziua",
            ][..],
        );
        stopwords.insert(
            LanguageCode::Hu,
            &[
                "a",
                "az",
                "és",
                "hogy",
                "nem",
                "egy",
                "is",
                "meg",
                "volt",
                "vagy",
                "csak",
                "mint",
                "már",
                "kell",
                "után",
                "szia",
                "köszönöm",
                "kérem",
                "jó",
                "reggelt",
                "igen",
            ][..],
        );
        stopwords.insert(
            LanguageCode::Fi,
            &[
                "ja", "on", "ei", "että", "oli", "hän", "se", "mutta", "kun", "niin", "kuin",
                "jos", "ovat", "myös", "hei", "kiitos", "kyllä", "missä", "hyvää", "huomenta",
            ][..],
        );
        stopwords.insert(
            LanguageCode::Sv,
            &[
                "och", "det", "att", "som", "en", "på", "är", "av", "för", "med", "till", "inte",
                "hej", "tack", "mycket", "god", "morgon", "var", "finns", "hjälp", "vatten",
            ][..],
        );

        Self {
            latin_profiles,
            stopwords,
        }
    }

    /// Automatically identify language from input text snippet
    pub fn detect(&self, text: &str) -> Option<LidResult> {
        let results = self.detect_top_n(text, 1);
        results.into_iter().next()
    }

    /// Detect top N candidate languages with confidence scores
    pub fn detect_top_n(&self, text: &str, n: usize) -> Vec<LidResult> {
        let clean = text.trim();
        if clean.is_empty() {
            return Vec::new();
        }

        // Tier 1: Unicode script and orthographic block classification
        if let Some(script_result) = self.classify_by_script(clean) {
            return vec![script_result];
        }

        // Tier 2: Latin script classification via n-grams, distinct diacritics, and stopwords
        self.classify_latin(clean, n)
    }

    /// Tier 1: Classify non-Latin scripts using character code points
    fn classify_by_script(&self, text: &str) -> Option<LidResult> {
        let mut hangul_count = 0usize;
        let mut kana_count = 0usize;
        let mut hanzi_count = 0usize;
        let mut arabic_count = 0usize;
        let mut devanagari_count = 0usize;
        let mut tamil_count = 0usize;
        let mut telugu_count = 0usize;
        let mut bengali_count = 0usize;
        let mut cyrillic_count = 0usize;
        let mut greek_count = 0usize;
        let mut hebrew_count = 0usize;
        let mut thai_count = 0usize;

        // Script-specific letter counts
        let mut persian_markers = 0usize;
        let mut urdu_markers = 0usize;
        let mut ukrainian_markers = 0usize;
        let mut bulgarian_markers = 0usize;

        let total_chars = text.chars().count();
        if total_chars == 0 {
            return None;
        }

        for c in text.chars() {
            match c {
                // Hangul (Korean)
                '\u{AC00}'..='\u{D7AF}' | '\u{1100}'..='\u{11FF}' | '\u{3130}'..='\u{318F}' => {
                    hangul_count += 1;
                }
                // Hiragana & Katakana (Japanese)
                '\u{3040}'..='\u{309F}' | '\u{30A0}'..='\u{30FF}' => {
                    kana_count += 1;
                }
                // Hanzi / Kanji (CJK Unified Ideographs)
                '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}' => {
                    hanzi_count += 1;
                }
                // Arabic Script
                '\u{0600}'..='\u{06FF}' | '\u{0750}'..='\u{077F}' | '\u{FB50}'..='\u{FDFF}' => {
                    arabic_count += 1;
                    if matches!(c, 'گ' | 'چ' | 'پ' | 'ژ' | 'ک' | 'ی') {
                        persian_markers += 1;
                    }
                    if matches!(c, 'ٹ' | 'ڈ' | 'ڑ' | 'ں' | 'ے' | 'ہ') {
                        urdu_markers += 1;
                    }
                }
                // Devanagari (Hindi)
                '\u{0900}'..='\u{097F}' => {
                    devanagari_count += 1;
                }
                // Tamil
                '\u{0B80}'..='\u{0BFF}' => {
                    tamil_count += 1;
                }
                // Telugu
                '\u{0C00}'..='\u{0C7F}' => {
                    telugu_count += 1;
                }
                // Bengali
                '\u{0980}'..='\u{09FF}' => {
                    bengali_count += 1;
                }
                // Cyrillic
                '\u{0400}'..='\u{04FF}' => {
                    cyrillic_count += 1;
                    if matches!(c, 'ї' | 'і' | 'є' | 'ґ' | 'Ї' | 'І' | 'Є' | 'Ґ') {
                        ukrainian_markers += 1;
                    }
                    if matches!(c, 'ъ' | 'щ' | 'Ъ' | 'Щ') {
                        bulgarian_markers += 1;
                    }
                }
                // Greek
                '\u{0370}'..='\u{03FF}' => {
                    greek_count += 1;
                }
                // Hebrew
                '\u{0590}'..='\u{05FF}' => {
                    hebrew_count += 1;
                }
                // Thai
                '\u{0E00}'..='\u{0E7F}' => {
                    thai_count += 1;
                }
                _ => {}
            }
        }

        // Japanese check (has Kana, or Hanzi mixed with Kana)
        if kana_count > 0 {
            return Some(LidResult {
                language: LanguageCode::Ja,
                confidence: 0.99,
                script: ScriptType::Kana,
            });
        }

        // Korean check
        if hangul_count > 0 {
            return Some(LidResult {
                language: LanguageCode::Ko,
                confidence: 0.99,
                script: ScriptType::Hangul,
            });
        }

        // Chinese check (pure CJK without Kana/Hangul)
        if hanzi_count > 0 && (hanzi_count as f32 / total_chars as f32) > 0.3 {
            return Some(LidResult {
                language: LanguageCode::Zh,
                confidence: 0.98,
                script: ScriptType::HanziCJK,
            });
        }

        // Arabic / Persian / Urdu check
        if arabic_count > 0 {
            if urdu_markers > 0 {
                return Some(LidResult {
                    language: LanguageCode::Ur,
                    confidence: 0.95,
                    script: ScriptType::ArabicScript,
                });
            }
            if persian_markers > 0
                || text.contains("است")
                || text.contains("این")
                || text.contains("برای")
                || text.contains("یک")
            {
                return Some(LidResult {
                    language: LanguageCode::Fa,
                    confidence: 0.95,
                    script: ScriptType::ArabicScript,
                });
            }
            return Some(LidResult {
                language: LanguageCode::Ar,
                confidence: 0.96,
                script: ScriptType::ArabicScript,
            });
        }

        // Devanagari (Hindi)
        if devanagari_count > 0 {
            return Some(LidResult {
                language: LanguageCode::Hi,
                confidence: 0.99,
                script: ScriptType::Devanagari,
            });
        }

        // Tamil
        if tamil_count > 0 {
            return Some(LidResult {
                language: LanguageCode::Ta,
                confidence: 0.99,
                script: ScriptType::TamilScript,
            });
        }

        // Telugu
        if telugu_count > 0 {
            return Some(LidResult {
                language: LanguageCode::Te,
                confidence: 0.99,
                script: ScriptType::TeluguScript,
            });
        }

        // Bengali
        if bengali_count > 0 {
            return Some(LidResult {
                language: LanguageCode::Bn,
                confidence: 0.99,
                script: ScriptType::BengaliScript,
            });
        }

        // Greek
        if greek_count > 0 {
            return Some(LidResult {
                language: LanguageCode::El,
                confidence: 0.99,
                script: ScriptType::GreekScript,
            });
        }

        // Hebrew
        if hebrew_count > 0 {
            return Some(LidResult {
                language: LanguageCode::He,
                confidence: 0.99,
                script: ScriptType::HebrewScript,
            });
        }

        // Thai
        if thai_count > 0 {
            return Some(LidResult {
                language: LanguageCode::Th,
                confidence: 0.99,
                script: ScriptType::ThaiScript,
            });
        }

        // Cyrillic (Ukrainian, Bulgarian, or Russian)
        if cyrillic_count > 0 {
            if ukrainian_markers > 0 {
                return Some(LidResult {
                    language: LanguageCode::Uk,
                    confidence: 0.96,
                    script: ScriptType::Cyrillic,
                });
            }
            // Check for Bulgarian stop words or markers
            let lower = text.to_lowercase();
            if bulgarian_markers > 0 && (lower.contains(" и ") || lower.contains(" на ")) {
                return Some(LidResult {
                    language: LanguageCode::Bg,
                    confidence: 0.90,
                    script: ScriptType::Cyrillic,
                });
            }
            return Some(LidResult {
                language: LanguageCode::Ru,
                confidence: 0.95,
                script: ScriptType::Cyrillic,
            });
        }

        None
    }

    /// Tier 2: Classify Latin script languages using distinctive characters, trigrams, and stopwords
    fn classify_latin(&self, text: &str, n: usize) -> Vec<LidResult> {
        let lower = text.to_lowercase();

        // 1. Check for unambiguous language-defining diacritics
        if lower.contains('ơ')
            || lower.contains('ư')
            || lower.contains('đ')
            || lower.contains('ả')
            || lower.contains('ã') && (lower.contains("ng") || lower.contains("th"))
        {
            return vec![LidResult {
                language: LanguageCode::Vi,
                confidence: 0.98,
                script: ScriptType::Latin,
            }];
        }
        if lower.contains('ą')
            || lower.contains('ć')
            || lower.contains('ę')
            || lower.contains('ł')
            || lower.contains('ń')
            || lower.contains('ś')
            || lower.contains('ź')
            || lower.contains('ż')
        {
            return vec![LidResult {
                language: LanguageCode::Pl,
                confidence: 0.98,
                script: ScriptType::Latin,
            }];
        }
        if lower.contains('ě')
            || lower.contains('ř')
            || lower.contains('ů')
            || lower.contains('ť')
            || lower.contains('ď')
        {
            return vec![LidResult {
                language: LanguageCode::Cs,
                confidence: 0.98,
                script: ScriptType::Latin,
            }];
        }
        if lower.contains('ğ') || lower.contains('ı') || lower.contains('ş') {
            return vec![LidResult {
                language: LanguageCode::Tr,
                confidence: 0.98,
                script: ScriptType::Latin,
            }];
        }
        if lower.contains('ő') || lower.contains('ű') {
            return vec![LidResult {
                language: LanguageCode::Hu,
                confidence: 0.98,
                script: ScriptType::Latin,
            }];
        }
        if lower.contains('ș') || lower.contains('ț') || lower.contains('ă') || lower.contains('â')
        {
            return vec![LidResult {
                language: LanguageCode::Ro,
                confidence: 0.97,
                script: ScriptType::Latin,
            }];
        }
        if lower.contains('ß') {
            return vec![LidResult {
                language: LanguageCode::De,
                confidence: 0.98,
                script: ScriptType::Latin,
            }];
        }
        if lower.contains('ñ') || lower.contains('¿') || lower.contains('¡') {
            return vec![LidResult {
                language: LanguageCode::Es,
                confidence: 0.98,
                script: ScriptType::Latin,
            }];
        }
        if lower.contains('ã') || lower.contains('õ') {
            return vec![LidResult {
                language: LanguageCode::Pt,
                confidence: 0.98,
                script: ScriptType::Latin,
            }];
        }
        if lower.contains('œ') || lower.contains(" c'est") || lower.contains(" d'") {
            return vec![LidResult {
                language: LanguageCode::Fr,
                confidence: 0.97,
                script: ScriptType::Latin,
            }];
        }

        // 2. Score candidate Latin languages by trigrams + stopwords
        let words: Vec<&str> = lower
            .split(|c: char| !c.is_alphabetic())
            .filter(|w| !w.is_empty())
            .collect();

        let mut scores: HashMap<LanguageCode, f32> = HashMap::new();

        // Score based on exact stopword hits
        for word in &words {
            for (&lang, &word_list) in &self.stopwords {
                if word_list.contains(word) {
                    *scores.entry(lang).or_insert(0.0) += 3.0;
                }
            }
        }

        // Score based on trigram frequency matches
        for (&lang, &trigrams) in &self.latin_profiles {
            for &tri in trigrams {
                if lower.contains(tri) {
                    *scores.entry(lang).or_insert(0.0) += 1.0;
                }
            }
        }

        // If no scores matched, default to English as global lingua franca fallback
        if scores.is_empty() {
            return vec![LidResult {
                language: LanguageCode::En,
                confidence: 0.50,
                script: ScriptType::Latin,
            }];
        }

        // Sort descending by score
        let mut sorted: Vec<(LanguageCode, f32)> = scores.into_iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let max_score = sorted.first().map(|s| s.1).unwrap_or(1.0).max(1.0);

        sorted
            .into_iter()
            .take(n)
            .map(|(lang, score)| {
                let conf = (score / (max_score * 1.2)).clamp(0.55, 0.99);
                LidResult {
                    language: lang,
                    confidence: conf,
                    script: ScriptType::Latin,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_asian_scripts_detection() {
        let detector = LanguageDetector::new();

        // Japanese
        let res = detector
            .detect("こんにちは世界、リアルタイム翻訳テスト")
            .unwrap();
        assert_eq!(res.language, LanguageCode::Ja);
        assert_eq!(res.script, ScriptType::Kana);

        // Korean
        let res = detector
            .detect("안녕하세요 오프라인 번역기 테스트입니다")
            .unwrap();
        assert_eq!(res.language, LanguageCode::Ko);
        assert_eq!(res.script, ScriptType::Hangul);

        // Chinese
        let res = detector.detect("欢迎使用离线全能实时翻译系统").unwrap();
        assert_eq!(res.language, LanguageCode::Zh);
        assert_eq!(res.script, ScriptType::HanziCJK);

        // Thai
        let res = detector.detect("สวัสดีชาวโลก ยินดีต้อนรับสู่ระบบแปลภาษา").unwrap();
        assert_eq!(res.language, LanguageCode::Th);
        assert_eq!(res.script, ScriptType::ThaiScript);
    }

    #[test]
    fn test_middle_eastern_and_indic_scripts() {
        let detector = LanguageDetector::new();

        // Arabic
        let res = detector
            .detect("مرحبا بكم في نظام الترجمة الفورية دون اتصال بالإنترنت")
            .unwrap();
        assert_eq!(res.language, LanguageCode::Ar);
        assert_eq!(res.script, ScriptType::ArabicScript);

        // Persian
        let res = detector
            .detect("سلام به همه، این یک آزمایش ترجمه آفلاین است")
            .unwrap();
        assert_eq!(res.language, LanguageCode::Fa);

        // Hindi
        let res = detector
            .detect("नमस्ते दुनिया, ऑफ़लाइन अनुवादक में आपका स्वागत है")
            .unwrap();
        assert_eq!(res.language, LanguageCode::Hi);
        assert_eq!(res.script, ScriptType::Devanagari);

        // Hebrew
        let res = detector
            .detect("שלום עולם, ברוכים הבאים למערכת התרגום")
            .unwrap();
        assert_eq!(res.language, LanguageCode::He);
        assert_eq!(res.script, ScriptType::HebrewScript);
    }

    #[test]
    fn test_cyrillic_and_greek_scripts() {
        let detector = LanguageDetector::new();

        // Russian
        let res = detector
            .detect("Привет мир, это система офлайн перевода")
            .unwrap();
        assert_eq!(res.language, LanguageCode::Ru);
        assert_eq!(res.script, ScriptType::Cyrillic);

        // Ukrainian (distinctive letters: ї, є)
        let res = detector
            .detect("Привіт світ, це український переклад без мережі")
            .unwrap();
        assert_eq!(res.language, LanguageCode::Uk);

        // Greek
        let res = detector
            .detect("Γεια σου κόσμε, καλώς ήρθατε στο μεταφραστή")
            .unwrap();
        assert_eq!(res.language, LanguageCode::El);
        assert_eq!(res.script, ScriptType::GreekScript);
    }

    #[test]
    fn test_latin_languages_detection() {
        let detector = LanguageDetector::new();

        // English
        let res = detector
            .detect(
                "This is an offline real-time translation application with zero network egress.",
            )
            .unwrap();
        assert_eq!(res.language, LanguageCode::En);

        // Spanish
        let res = detector
            .detect("Esta es una aplicación de traducción en tiempo real sin conexión a internet.")
            .unwrap();
        assert_eq!(res.language, LanguageCode::Es);

        // French
        let res = detector
            .detect("Ceci est une application de traduction en temps réel hors ligne.")
            .unwrap();
        assert_eq!(res.language, LanguageCode::Fr);

        // German
        let res = detector
            .detect("Das ist eine Echtzeit-Übersetzungsanwendung für den Offline-Einsatz.")
            .unwrap();
        assert_eq!(res.language, LanguageCode::De);

        // Italian
        let res = detector
            .detect("Questa è un'applicazione di traduzione in tempo reale per uso offline.")
            .unwrap();
        assert_eq!(res.language, LanguageCode::It);

        // Polish
        let res = detector
            .detect(
                "To jest aplikacja do tłumaczenia w czasie rzeczywistym bez połączenia z siecią.",
            )
            .unwrap();
        assert_eq!(res.language, LanguageCode::Pl);

        // Turkish
        let res = detector
            .detect("Bu çevrimdışı gerçek zamanlı bir dil çeviri uygulamasıdır.")
            .unwrap();
        assert_eq!(res.language, LanguageCode::Tr);

        // Vietnamese
        let res = detector
            .detect("Đây là ứng dụng dịch thuật thời gian thực ngoại tuyến không cần internet.")
            .unwrap();
        assert_eq!(res.language, LanguageCode::Vi);
    }

    #[test]
    fn test_lid_latency_sub_500us_sla() {
        let detector = LanguageDetector::new();
        let sentence = "The quick brown fox jumps over the lazy dog and tests offline language identification performance.";

        // Warmup
        let _ = detector.detect(sentence);

        let start = Instant::now();
        let iters = 1000;
        for _ in 0..iters {
            let res = detector.detect(sentence);
            assert!(res.is_some());
        }
        let elapsed = start.elapsed();
        let per_op_micros = elapsed.as_micros() as f64 / iters as f64;

        // SLA is <500µs, typically <50µs in Rust
        assert!(
            per_op_micros < 500.0,
            "LID latency was {:.2}µs, exceeded 500µs SLA",
            per_op_micros
        );
    }
}
