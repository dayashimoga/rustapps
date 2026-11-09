#![forbid(unsafe_code)]
//! Central Orchestrator Service for Universal Live Translator
//!
//! Unifies offline Automatic Language Detection (LID), Neural Machine Translation (NMT),
//! Speech Recognition (ASR), Formant Speech Synthesis (TTS), Camera OCR overlay translation,
//! document & subtitle processing, and SQLite WAL phrasebook storage into a coherent service.

use crate::camera_translate::{CameraTranslateError, CameraTranslator};
use crate::lid::{LanguageDetector, LidResult};
use crate::models::{
    BatchDocumentItem, BatchTranslationResult, CameraTranslationResult, ConversationSession,
    ConversationTurn, GlossaryTerm, LanguageCode, LanguagePackInfo, MeetingSessionConfig,
    MeetingSessionState, MeetingTranscriptEntry, PhraseCategory, PhrasebookItem, SubtitleCue,
    SubtitleFormat, TranscriptSegment, TranslationRequest, TranslationResponse,
    TtsSynthesisRequest, TtsSynthesisResponse,
};
use crate::nmt::{OfflineTranslationEngine, TranslationError};
use crate::phrasebook::{PhrasebookError, PhrasebookStore, TranslationHistoryRecord};
use crate::speech::{BatchSpeechRecognizer, SpeechError};
use crate::subtitles::{SubtitleError, SubtitleTranslator};
use crate::tts::{TtsError, TtsSynthesizer};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Central service error definitions
#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Translation error: {0}")]
    Translation(#[from] TranslationError),
    #[error("Phrasebook storage error: {0}")]
    Phrasebook(#[from] PhrasebookError),
    #[error("Speech recognition error: {0}")]
    Speech(#[from] SpeechError),
    #[error("Speech synthesis error: {0}")]
    Tts(#[from] TtsError),
    #[error("Camera translation error: {0}")]
    Camera(#[from] CameraTranslateError),
    #[error("Subtitle or document error: {0}")]
    Subtitle(#[from] SubtitleError),
    #[error("Conversation session '{0}' not found")]
    ConversationNotFound(String),
}

/// Thread-safe master service orchestrating all translation capabilities
pub struct TranslatorService {
    detector: Arc<LanguageDetector>,
    nmt: Arc<OfflineTranslationEngine>,
    phrasebook: Arc<PhrasebookStore>,
    speech_batch: Arc<BatchSpeechRecognizer>,
    tts: Arc<TtsSynthesizer>,
    camera: Arc<CameraTranslator>,
    subtitles: Arc<SubtitleTranslator>,
    conversations: Arc<RwLock<HashMap<String, ConversationSession>>>,
    meeting_sessions: Arc<RwLock<HashMap<String, MeetingSessionState>>>,
}

impl TranslatorService {
    /// Initialize service with persistent disk directory for phrasebook and history
    pub fn open<P: AsRef<Path>>(data_dir: P) -> Result<Self, ServiceError> {
        let db_path = data_dir.as_ref().join("translator_phrasebook.db");
        let phrasebook = Arc::new(PhrasebookStore::open(db_path)?);
        Self::create_with_store(phrasebook)
    }

    /// Initialize service with in-memory database (suitable for unit tests or ephemeral mode)
    pub fn open_in_memory() -> Result<Self, ServiceError> {
        let phrasebook = Arc::new(PhrasebookStore::open_in_memory()?);
        Self::create_with_store(phrasebook)
    }

    fn create_with_store(phrasebook: Arc<PhrasebookStore>) -> Result<Self, ServiceError> {
        let detector = Arc::new(LanguageDetector::new());
        let nmt = Arc::new(OfflineTranslationEngine::new());
        let speech_batch = Arc::new(BatchSpeechRecognizer::new());
        let tts = Arc::new(TtsSynthesizer::new());
        let camera = Arc::new(CameraTranslator::new(nmt.clone()));
        let subtitles = Arc::new(SubtitleTranslator::new(nmt.clone(), speech_batch.clone()));

        // Populate stored domain glossaries into NMT engine
        if let Ok(domains) = phrasebook.list_glossary_domains() {
            for d in domains {
                if let Ok(terms) = phrasebook.get_glossary_terms(&d) {
                    nmt.register_glossary(&d, terms);
                }
            }
        }

        Ok(Self {
            detector,
            nmt,
            phrasebook,
            speech_batch,
            tts,
            camera,
            subtitles,
            conversations: Arc::new(RwLock::new(HashMap::new())),
            meeting_sessions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    // -----------------------------------------------------------------------
    // Language Identification (LID)
    // -----------------------------------------------------------------------

    /// Automatically identify language from input text snippet in <500µs
    pub fn detect_language(&self, text: &str) -> Option<LidResult> {
        self.detector.detect(text)
    }

    /// Detect top N candidate languages with confidence scores
    pub fn detect_languages_top_n(&self, text: &str, n: usize) -> Vec<LidResult> {
        self.detector.detect_top_n(text, n)
    }

    // -----------------------------------------------------------------------
    // Text Translation (NMT)
    // -----------------------------------------------------------------------

    /// Translate text and automatically record to local SQLite audit history
    pub fn translate_text(
        &self,
        request: &TranslationRequest,
    ) -> Result<TranslationResponse, ServiceError> {
        self.translate_text_with_options(request, true)
    }

    /// Translate text with explicit control over whether to record in SQLite audit history
    pub fn translate_text_with_options(
        &self,
        request: &TranslationRequest,
        record_history: bool,
    ) -> Result<TranslationResponse, ServiceError> {
        let response = self.nmt.translate(request)?;

        let clean_orig = response.original_text.trim();
        let clean_trans = response.translated_text.trim();

        // Only record in translation history when explicitly requested and translation is non-trivial
        if record_history
            && clean_orig.len() >= 2
            && !clean_trans.is_empty()
            && clean_orig.to_lowercase() != clean_trans.to_lowercase()
        {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let hist_rec = TranslationHistoryRecord {
                id: blake3::hash(
                    format!(
                        "{}:{}:{}",
                        response.source_lang.as_str(),
                        response.target_lang.as_str(),
                        clean_orig.to_lowercase()
                    )
                    .as_bytes(),
                )
                .to_hex()[..16]
                    .to_string(),
                source_lang: response.source_lang,
                target_lang: response.target_lang,
                original_text: response.original_text.clone(),
                translated_text: response.translated_text.clone(),
                timestamp_secs: timestamp,
                confidence: response.confidence,
            };

            let _ = self.phrasebook.add_history(&hist_rec);
        }

        Ok(response)
    }

    // -----------------------------------------------------------------------
    // Speech Recognition & Synthesis (ASR / TTS)
    // -----------------------------------------------------------------------

    /// Synthesize speech to playable 22.05kHz WAV byte buffer
    pub fn synthesize_speech(
        &self,
        request: &TtsSynthesisRequest,
    ) -> Result<TtsSynthesisResponse, ServiceError> {
        Ok(self.tts.synthesize(request)?)
    }

    /// Recognize speech from 16kHz mono float PCM buffer
    pub fn recognize_speech(
        &self,
        pcm: &[f32],
        language: LanguageCode,
    ) -> Result<Vec<TranscriptSegment>, ServiceError> {
        Ok(self.speech_batch.recognize(pcm, language)?)
    }

    // -----------------------------------------------------------------------
    // Camera OCR Overlay Translation
    // -----------------------------------------------------------------------

    /// Perform OCR on image bytes and translate recognized text blocks
    pub fn translate_camera_image(
        &self,
        image_bytes: &[u8],
        source_lang: Option<LanguageCode>,
        target_lang: LanguageCode,
    ) -> Result<CameraTranslationResult, ServiceError> {
        Ok(self
            .camera
            .translate_image_bytes(image_bytes, source_lang, target_lang)?)
    }

    /// Render annotated BMP image with translated overlays
    pub fn render_camera_overlay_bmp(
        &self,
        image_bytes: &[u8],
        result: &CameraTranslationResult,
    ) -> Result<Vec<u8>, ServiceError> {
        Ok(self.camera.render_overlay_bmp(image_bytes, result)?)
    }

    // -----------------------------------------------------------------------
    // Document & Subtitle Translation
    // -----------------------------------------------------------------------

    /// Translate subtitle content (.srt or .vtt)
    pub fn translate_subtitle_content(
        &self,
        content: &str,
        format: SubtitleFormat,
        source_lang: Option<LanguageCode>,
        target_lang: LanguageCode,
        domain_glossary: Option<&str>,
    ) -> Result<String, ServiceError> {
        Ok(self.subtitles.translate_subtitle_file(
            content,
            format,
            source_lang,
            target_lang,
            domain_glossary,
        )?)
    }

    /// Translate subtitle content (.srt or .vtt) bilingually
    pub fn translate_subtitle_content_bilingual(
        &self,
        content: &str,
        format: SubtitleFormat,
        source_lang: Option<LanguageCode>,
        target_lang: LanguageCode,
        domain_glossary: Option<&str>,
        primary_top: bool,
    ) -> Result<String, ServiceError> {
        Ok(self.subtitles.translate_subtitle_file_bilingual(
            content,
            format,
            source_lang,
            target_lang,
            domain_glossary,
            primary_top,
        )?)
    }

    /// Translate document text (.md, .txt) preserving structural elements
    pub fn translate_document_content(
        &self,
        doc_text: &str,
        format: &str,
        source_lang: Option<LanguageCode>,
        target_lang: LanguageCode,
        domain_glossary: Option<&str>,
    ) -> Result<String, ServiceError> {
        Ok(self.subtitles.translate_document(
            doc_text,
            format,
            source_lang,
            target_lang,
            domain_glossary,
        )?)
    }

    /// Batch translate multiple documents (.txt, .md, .srt, .vtt)
    pub fn batch_translate_documents(
        &self,
        documents: &[BatchDocumentItem],
        source_lang: Option<LanguageCode>,
        target_lang: LanguageCode,
        domain_glossary: Option<&str>,
    ) -> Vec<BatchTranslationResult> {
        self.subtitles.batch_translate_documents(
            documents,
            source_lang,
            target_lang,
            domain_glossary,
        )
    }

    /// Generate timestamped subtitle file (.srt or .vtt) directly from 16kHz audio PCM
    pub fn generate_subtitles_from_audio(
        &self,
        pcm: &[f32],
        audio_lang: LanguageCode,
        target_lang: Option<LanguageCode>,
        format: SubtitleFormat,
    ) -> Result<String, ServiceError> {
        Ok(self
            .subtitles
            .generate_subtitles_from_audio(pcm, audio_lang, target_lang, format)?)
    }

    // -----------------------------------------------------------------------
    // Phrasebook & History
    // -----------------------------------------------------------------------

    pub fn add_phrase(&self, item: &PhrasebookItem) -> Result<(), ServiceError> {
        Ok(self.phrasebook.add_phrase(item)?)
    }

    pub fn get_phrase(&self, id: &str) -> Result<Option<PhrasebookItem>, ServiceError> {
        Ok(self.phrasebook.get_phrase(id)?)
    }

    pub fn list_phrases(
        &self,
        category: Option<PhraseCategory>,
        favorite_only: bool,
    ) -> Result<Vec<PhrasebookItem>, ServiceError> {
        Ok(self.phrasebook.list_phrases(category, favorite_only)?)
    }

    pub fn toggle_favorite_phrase(&self, id: &str) -> Result<bool, ServiceError> {
        Ok(self.phrasebook.toggle_favorite(id)?)
    }

    pub fn delete_phrase(&self, id: &str) -> Result<bool, ServiceError> {
        Ok(self.phrasebook.delete_phrase(id)?)
    }

    pub fn get_translation_history(
        &self,
        limit: usize,
    ) -> Result<Vec<TranslationHistoryRecord>, ServiceError> {
        Ok(self.phrasebook.get_history(limit)?)
    }

    pub fn clear_translation_history(&self) -> Result<(), ServiceError> {
        Ok(self.phrasebook.clear_history()?)
    }

    pub fn add_glossary_term(&self, domain: &str, term: &GlossaryTerm) -> Result<(), ServiceError> {
        self.phrasebook.add_glossary_term(domain, term)?;
        if let Ok(terms) = self.phrasebook.get_glossary_terms(domain) {
            self.nmt.register_glossary(domain, terms);
        }
        Ok(())
    }

    pub fn get_glossary_terms(&self, domain: &str) -> Result<Vec<GlossaryTerm>, ServiceError> {
        Ok(self.phrasebook.get_glossary_terms(domain)?)
    }

    pub fn delete_glossary_term(
        &self,
        domain: &str,
        source_term: &str,
    ) -> Result<bool, ServiceError> {
        let deleted = self.phrasebook.delete_glossary_term(domain, source_term)?;
        if let Ok(terms) = self.phrasebook.get_glossary_terms(domain) {
            self.nmt.register_glossary(domain, terms);
        }
        Ok(deleted)
    }

    pub fn list_glossary_domains(&self) -> Result<Vec<String>, ServiceError> {
        Ok(self.phrasebook.list_glossary_domains()?)
    }

    // -----------------------------------------------------------------------
    // Conversation Mode (Split-Screen Dialogue)
    // -----------------------------------------------------------------------

    /// Initialize a new split-screen conversation session
    pub fn create_conversation_session(
        &self,
        party_a_lang: LanguageCode,
        party_b_lang: LanguageCode,
    ) -> ConversationSession {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let id = format!("conv-{}", now);
        let session = ConversationSession {
            id: id.clone(),
            title: format!(
                "{} - {} Conversation",
                party_a_lang.display_name(),
                party_b_lang.display_name()
            ),
            party_a_lang,
            party_b_lang,
            created_at_secs: now,
            updated_at_secs: now,
            turns: Vec::new(),
        };

        if let Ok(mut map) = self.conversations.write() {
            map.insert(id, session.clone());
        }

        session
    }

    /// Add a dialogue turn to an active conversation session and translate to other party's language
    pub fn add_conversation_turn(
        &self,
        session_id: &str,
        speaker: &str,
        text: &str,
    ) -> Result<ConversationTurn, ServiceError> {
        let (source_lang, target_lang) = {
            let map = self
                .conversations
                .read()
                .map_err(|_| ServiceError::ConversationNotFound(session_id.to_string()))?;
            let session = map
                .get(session_id)
                .ok_or_else(|| ServiceError::ConversationNotFound(session_id.to_string()))?;

            if speaker.eq_ignore_ascii_case("speaker_a") || speaker.eq_ignore_ascii_case("party_a")
            {
                (session.party_a_lang, session.party_b_lang)
            } else {
                (session.party_b_lang, session.party_a_lang)
            }
        };

        let trans_req = TranslationRequest {
            source_lang: Some(source_lang),
            target_lang,
            text: text.to_string(),
            preserve_formatting: true,
            domain_glossary: None,
        };

        let trans_res = self.translate_text(&trans_req)?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let turn = ConversationTurn {
            id: format!("turn-{}", now),
            speaker: speaker.to_string(),
            source_lang,
            target_lang,
            original_text: text.to_string(),
            translated_text: trans_res.translated_text,
            timestamp_secs: now,
            audio_duration_ms: None,
        };

        if let Ok(mut map) = self.conversations.write() {
            if let Some(session) = map.get_mut(session_id) {
                session.turns.push(turn.clone());
                session.updated_at_secs = now;
            }
        }

        Ok(turn)
    }

    /// Retrieve active conversation session by ID
    pub fn get_conversation_session(&self, session_id: &str) -> Option<ConversationSession> {
        let map = self.conversations.read().ok()?;
        map.get(session_id).cloned()
    }

    /// List all conversation sessions
    pub fn list_conversation_sessions(&self) -> Vec<ConversationSession> {
        if let Ok(map) = self.conversations.read() {
            map.values().cloned().collect()
        } else {
            Vec::new()
        }
    }

    // -----------------------------------------------------------------------
    // Meeting Mode (System Loopback Audio & Ephemeral Zero-Retention)
    // -----------------------------------------------------------------------

    /// Start a new meeting session with system loopback audio translation
    pub fn start_meeting_session(&self, config: MeetingSessionConfig) -> MeetingSessionState {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let state = MeetingSessionState {
            config,
            created_at_secs: now,
            updated_at_secs: now,
            entries: Vec::new(),
            is_active: true,
        };

        if let Ok(mut map) = self.meeting_sessions.write() {
            map.insert(state.config.session_id.clone(), state.clone());
        }

        state
    }

    /// Feed audio chunk (16kHz mono PCM) from loopback capture into meeting session
    pub fn feed_meeting_audio_chunk(
        &self,
        session_id: &str,
        speaker: &str,
        pcm: &[f32],
        timestamp_ms: u64,
    ) -> Result<Option<MeetingTranscriptEntry>, ServiceError> {
        let (source_lang, target_lang, zero_retention, domain_glossary) = {
            let map = self
                .meeting_sessions
                .read()
                .map_err(|_| ServiceError::ConversationNotFound(session_id.to_string()))?;
            let s = map
                .get(session_id)
                .ok_or_else(|| ServiceError::ConversationNotFound(session_id.to_string()))?;
            if !s.is_active {
                return Ok(None);
            }
            (
                s.config.source_lang.unwrap_or(LanguageCode::En),
                s.config.target_lang,
                s.config.zero_retention_mode,
                s.config.domain_glossary.clone(),
            )
        };

        // Recognize speech from PCM
        let segments = self.speech_batch.recognize(pcm, source_lang)?;
        if segments.is_empty() {
            return Ok(None);
        }

        let combined_text = segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();

        if combined_text.is_empty() {
            return Ok(None);
        }

        let avg_confidence =
            segments.iter().map(|s| s.confidence).sum::<f32>() / (segments.len() as f32);

        // Translate to target language. If zero_retention_mode is true, never log to disk history.
        let trans_req = TranslationRequest {
            source_lang: Some(source_lang),
            target_lang,
            text: combined_text.clone(),
            preserve_formatting: true,
            domain_glossary,
        };
        let trans_res = self.translate_text_with_options(&trans_req, !zero_retention)?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut map = self
            .meeting_sessions
            .write()
            .map_err(|_| ServiceError::ConversationNotFound(session_id.to_string()))?;
        let session = map
            .get_mut(session_id)
            .ok_or_else(|| ServiceError::ConversationNotFound(session_id.to_string()))?;

        let entry_id = session.entries.len() + 1;
        let entry = MeetingTranscriptEntry {
            id: entry_id,
            timestamp_ms,
            speaker: speaker.to_string(),
            source_text: combined_text,
            translated_text: trans_res.translated_text,
            confidence: avg_confidence,
        };

        session.entries.push(entry.clone());
        session.updated_at_secs = now;

        Ok(Some(entry))
    }

    /// Retrieve meeting transcript state
    pub fn get_meeting_transcript(&self, session_id: &str) -> Option<MeetingSessionState> {
        let map = self.meeting_sessions.read().ok()?;
        map.get(session_id).cloned()
    }

    /// End meeting session, optionally exporting to subtitles, and wipe from ephemeral memory if zero-retention
    pub fn end_meeting_session(
        &self,
        session_id: &str,
        export_subtitles: bool,
        format: Option<SubtitleFormat>,
        bilingual: bool,
    ) -> Result<Option<String>, ServiceError> {
        let mut map = self
            .meeting_sessions
            .write()
            .map_err(|_| ServiceError::ConversationNotFound(session_id.to_string()))?;
        let session = map
            .get_mut(session_id)
            .ok_or_else(|| ServiceError::ConversationNotFound(session_id.to_string()))?;

        session.is_active = false;

        let subtitle_export = if export_subtitles && !session.entries.is_empty() {
            let fmt = format.unwrap_or(SubtitleFormat::Srt);
            let mut cues = Vec::with_capacity(session.entries.len());
            for entry in &session.entries {
                let start_ms = entry.timestamp_ms;
                let end_ms = start_ms + 3000;
                cues.push(SubtitleCue {
                    index: entry.id,
                    start_ms,
                    end_ms,
                    original_text: format!("{}: {}", entry.speaker, entry.source_text),
                    translated_text: format!("{}: {}", entry.speaker, entry.translated_text),
                });
            }
            if bilingual {
                Some(
                    self.subtitles
                        .serialize_subtitles_bilingual(&cues, fmt, true),
                )
            } else {
                Some(self.subtitles.serialize_subtitles(&cues, fmt))
            }
        } else {
            None
        };

        if session.config.zero_retention_mode {
            map.remove(session_id);
        }

        Ok(subtitle_export)
    }

    // -----------------------------------------------------------------------
    // Offline AI Model Hub Bridge
    // -----------------------------------------------------------------------

    /// List available offline neural translation packs
    pub fn list_available_model_packs(&self) -> Vec<LanguagePackInfo> {
        vec![
            LanguagePackInfo {
                id: "pack-en-es-v1".to_string(),
                pair_name: "English - Spanish (Neural 4-bit)".to_string(),
                source_code: LanguageCode::En,
                target_code: LanguageCode::Es,
                size_bytes: 48 * 1024 * 1024,
                is_installed: self.nmt.has_neural_pack(LanguageCode::En, LanguageCode::Es),
                version: "1.0.0".to_string(),
                sha256_hash: "a1b2c3d4e5f6".to_string(),
            },
            LanguagePackInfo {
                id: "pack-en-fr-v1".to_string(),
                pair_name: "English - French (Neural 4-bit)".to_string(),
                source_code: LanguageCode::En,
                target_code: LanguageCode::Fr,
                size_bytes: 46 * 1024 * 1024,
                is_installed: self.nmt.has_neural_pack(LanguageCode::En, LanguageCode::Fr),
                version: "1.0.0".to_string(),
                sha256_hash: "f6e5d4c3b2a1".to_string(),
            },
            LanguagePackInfo {
                id: "pack-en-de-v1".to_string(),
                pair_name: "English - German (Neural 4-bit)".to_string(),
                source_code: LanguageCode::En,
                target_code: LanguageCode::De,
                size_bytes: 50 * 1024 * 1024,
                is_installed: self.nmt.has_neural_pack(LanguageCode::En, LanguageCode::De),
                version: "1.0.0".to_string(),
                sha256_hash: "b3c4d5e6f7a8".to_string(),
            },
            LanguagePackInfo {
                id: "pack-en-ja-v1".to_string(),
                pair_name: "English - Japanese (Neural 4-bit)".to_string(),
                source_code: LanguageCode::En,
                target_code: LanguageCode::Ja,
                size_bytes: 54 * 1024 * 1024,
                is_installed: self.nmt.has_neural_pack(LanguageCode::En, LanguageCode::Ja),
                version: "1.0.0".to_string(),
                sha256_hash: "c7d8e9f0a1b2".to_string(),
            },
            LanguagePackInfo {
                id: "pack-en-zh-v1".to_string(),
                pair_name: "English - Chinese (Neural 4-bit)".to_string(),
                source_code: LanguageCode::En,
                target_code: LanguageCode::Zh,
                size_bytes: 52 * 1024 * 1024,
                is_installed: self.nmt.has_neural_pack(LanguageCode::En, LanguageCode::Zh),
                version: "1.0.0".to_string(),
                sha256_hash: "d8e9f0a1b2c3".to_string(),
            },
        ]
    }

    /// Mount neural pack entries directly into NMT engine
    pub fn mount_neural_pack_entries(
        &self,
        source_lang: LanguageCode,
        target_lang: LanguageCode,
        entries: HashMap<String, String>,
    ) -> usize {
        self.nmt
            .load_neural_pack_entries(source_lang, target_lang, entries)
    }

    /// Check if neural model pack is mounted for this language pair
    pub fn has_neural_pack(&self, source_lang: LanguageCode, target_lang: LanguageCode) -> bool {
        self.nmt.has_neural_pack(source_lang, target_lang)
    }
}

impl Default for TranslatorService {
    fn default() -> Self {
        Self::open_in_memory().expect("Failed to initialize default TranslatorService")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_lifecycle_and_translation() {
        let service = TranslatorService::open_in_memory().unwrap();

        // 1. Automatic LID
        let lid = service.detect_language("Bonjour tout le monde").unwrap();
        assert_eq!(lid.language, LanguageCode::Fr);

        // 2. Text Translation
        let req = TranslationRequest {
            source_lang: Some(LanguageCode::En),
            target_lang: LanguageCode::Es,
            text: "Hello world".to_string(),
            preserve_formatting: true,
            domain_glossary: None,
        };
        let res = service.translate_text(&req).unwrap();
        assert_eq!(res.translated_text, "Hola mundo");

        // 3. History check
        let history = service.get_translation_history(5).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].original_text, "Hello world");

        // 4. TTS Synthesis
        let tts_req = TtsSynthesisRequest {
            text: "Hello".to_string(),
            language: LanguageCode::En,
            speaking_rate: 1.0,
            pitch: 1.0,
            sample_rate: 22050,
        };
        let tts_res = service.synthesize_speech(&tts_req).unwrap();
        assert_eq!(&tts_res.wav_bytes[0..4], b"RIFF");

        // 5. Conversation session
        let conv = service.create_conversation_session(LanguageCode::En, LanguageCode::Es);
        let turn = service
            .add_conversation_turn(&conv.id, "speaker_a", "Hello")
            .unwrap();
        assert_eq!(turn.translated_text.to_lowercase(), "hola");

        let fetched = service.get_conversation_session(&conv.id).unwrap();
        assert_eq!(fetched.turns.len(), 1);
    }

    #[test]
    fn test_meeting_mode_zero_retention_and_bilingual_export() {
        let service = TranslatorService::open_in_memory().unwrap();

        let session_id = "meet-101".to_string();
        let config = MeetingSessionConfig {
            session_id: session_id.clone(),
            title: Some("All-Hands Global Sync".to_string()),
            source_lang: Some(LanguageCode::En),
            target_lang: LanguageCode::Es,
            zero_retention_mode: true,
            domain_glossary: None,
        };

        let state = service.start_meeting_session(config);
        assert_eq!(state.config.session_id, session_id);
        assert!(state.is_active);

        // Feed speech chunk
        let pcm = vec![0.35f32; 16000]; // 1 sec synthetic audio
        let entry_opt = service
            .feed_meeting_audio_chunk(&session_id, "Alice (Presenter)", &pcm, 1000)
            .unwrap();
        assert!(entry_opt.is_some());
        let entry = entry_opt.unwrap();
        assert_eq!(entry.speaker, "Alice (Presenter)");
        assert!(!entry.source_text.is_empty());
        assert!(!entry.translated_text.is_empty());

        // Check transcript
        let current_state = service.get_meeting_transcript(&session_id).unwrap();
        assert_eq!(current_state.entries.len(), 1);

        // Because zero_retention_mode is true, translation history in SQLite must NOT contain this entry
        let history = service.get_translation_history(10).unwrap();
        assert_eq!(history.len(), 0);

        // End meeting and export bilingual subtitles
        let exported_srt = service
            .end_meeting_session(&session_id, true, Some(SubtitleFormat::Srt), true)
            .unwrap();
        assert!(exported_srt.is_some());
        let srt_text = exported_srt.unwrap();
        assert!(srt_text.contains("Alice (Presenter):"));
        assert!(srt_text.contains("-->"));

        // Because zero_retention_mode is true, session must be wiped from memory upon termination
        assert!(service.get_meeting_transcript(&session_id).is_none());
    }

    #[test]
    fn test_service_model_pack_management() {
        let service = TranslatorService::open_in_memory().unwrap();

        let packs = service.list_available_model_packs();
        assert!(packs.len() >= 5);

        assert!(!service.has_neural_pack(LanguageCode::En, LanguageCode::Es));

        let mut custom_pack = HashMap::new();
        custom_pack.insert("microservices".to_string(), "microservicios".to_string());
        let loaded =
            service.mount_neural_pack_entries(LanguageCode::En, LanguageCode::Es, custom_pack);
        assert_eq!(loaded, 1);
        assert!(service.has_neural_pack(LanguageCode::En, LanguageCode::Es));
    }
}
