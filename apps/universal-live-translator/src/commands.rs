#![forbid(unsafe_code)]
//! Tauri Desktop IPC Command Handlers for Universal Live Translator
//!
//! Provides typed command interfaces invoked by the desktop Svelte/TypeScript UI.

use crate::models::{
    ConversationSession, ConversationTurn, GlossaryTerm, LanguageCode, PhraseCategory,
    PhrasebookItem, SubtitleFormat, TranslationRequest, TranslationResponse, TtsSynthesisRequest,
    TtsSynthesisResponse,
};
use crate::service::TranslatorService;
use std::sync::{Arc, Mutex, OnceLock};

static SERVICE: OnceLock<Arc<Mutex<TranslatorService>>> = OnceLock::new();

pub fn get_service() -> Arc<Mutex<TranslatorService>> {
    SERVICE
        .get_or_init(|| {
            Arc::new(Mutex::new(
                TranslatorService::open_in_memory()
                    .expect("Failed to initialize default TranslatorService"),
            ))
        })
        .clone()
}

/// Automatic Language Identification command
pub fn detect_language_cmd(text: String) -> Result<Option<crate::lid::LidResult>, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    Ok(svc.detect_language(&text))
}

/// Text translation command
pub fn translate_text_cmd(request: TranslationRequest) -> Result<TranslationResponse, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.translate_text(&request).map_err(|e| e.to_string())
}

/// Speech synthesis command returning WAV bytes
pub fn synthesize_speech_cmd(request: TtsSynthesisRequest) -> Result<TtsSynthesisResponse, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.synthesize_speech(&request).map_err(|e| e.to_string())
}

/// Subtitle translation command (.srt or .vtt)
pub fn translate_subtitles_cmd(
    content: String,
    format_str: String,
    source_lang_str: Option<String>,
    target_lang_str: String,
) -> Result<String, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;

    let format = match format_str.to_lowercase().as_str() {
        "vtt" => SubtitleFormat::Vtt,
        _ => SubtitleFormat::Srt,
    };

    let source_lang = source_lang_str.and_then(|s| LanguageCode::parse(&s));
    let target_lang = LanguageCode::parse(&target_lang_str).unwrap_or(LanguageCode::Es);

    svc.translate_subtitle_content(&content, format, source_lang, target_lang)
        .map_err(|e| e.to_string())
}

/// Document translation command (.md or .txt)
pub fn translate_document_cmd(
    content: String,
    format: String,
    source_lang_str: Option<String>,
    target_lang_str: String,
) -> Result<String, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;

    let source_lang = source_lang_str.and_then(|s| LanguageCode::parse(&s));
    let target_lang = LanguageCode::parse(&target_lang_str).unwrap_or(LanguageCode::Es);

    svc.translate_document_content(&content, &format, source_lang, target_lang)
        .map_err(|e| e.to_string())
}

/// List phrasebook items with optional filters
pub fn list_phrases_cmd(
    category_str: Option<String>,
    favorite_only: bool,
) -> Result<Vec<PhrasebookItem>, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;

    let category = category_str
        .and_then(|c| serde_json::from_str::<PhraseCategory>(&format!("\"{}\"", c)).ok());

    svc.list_phrases(category, favorite_only)
        .map_err(|e| e.to_string())
}

/// Add or update a phrasebook item
pub fn add_phrase_cmd(item: PhrasebookItem) -> Result<(), String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.add_phrase(&item).map_err(|e| e.to_string())
}

/// Toggle favorite status of a phrasebook item
pub fn toggle_favorite_phrase_cmd(id: String) -> Result<bool, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.toggle_favorite_phrase(&id).map_err(|e| e.to_string())
}

/// Delete a phrasebook item
pub fn delete_phrase_cmd(id: String) -> Result<bool, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.delete_phrase(&id).map_err(|e| e.to_string())
}

/// Retrieve translation audit history
pub fn get_history_cmd(
    limit: usize,
) -> Result<Vec<crate::phrasebook::TranslationHistoryRecord>, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.get_translation_history(limit)
        .map_err(|e| e.to_string())
}

/// Clear translation history
pub fn clear_history_cmd() -> Result<(), String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.clear_translation_history().map_err(|e| e.to_string())
}

/// Add domain glossary term
pub fn add_glossary_term_cmd(domain: String, term: GlossaryTerm) -> Result<(), String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.add_glossary_term(&domain, &term)
        .map_err(|e| e.to_string())
}

/// Retrieve domain glossary terms
pub fn get_glossary_terms_cmd(domain: String) -> Result<Vec<GlossaryTerm>, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.get_glossary_terms(&domain).map_err(|e| e.to_string())
}

/// Start a new split-screen conversation session
pub fn start_conversation_session_cmd(
    party_a_lang: String,
    party_b_lang: String,
) -> Result<ConversationSession, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;

    let party_a = LanguageCode::parse(&party_a_lang).unwrap_or(LanguageCode::En);
    let party_b = LanguageCode::parse(&party_b_lang).unwrap_or(LanguageCode::Es);

    Ok(svc.create_conversation_session(party_a, party_b))
}

/// Add a turn to an active conversation session
pub fn add_conversation_turn_cmd(
    session_id: String,
    speaker: String,
    text: String,
) -> Result<ConversationTurn, String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    svc.add_conversation_turn(&session_id, &speaker, &text)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commands_workflow() {
        let req = TranslationRequest {
            source_lang: Some(LanguageCode::En),
            target_lang: LanguageCode::Es,
            text: "Hello".to_string(),
            preserve_formatting: true,
            domain_glossary: None,
        };

        let res = translate_text_cmd(req).unwrap();
        assert_eq!(res.translated_text.to_lowercase(), "hola");

        let phrase = PhrasebookItem {
            id: "cmd-p1".to_string(),
            category: PhraseCategory::Travel,
            source_text: "Where is the hotel?".to_string(),
            source_lang: LanguageCode::En,
            target_text: "¿Dónde está el hotel?".to_string(),
            target_lang: LanguageCode::Es,
            notes: None,
            is_favorite: false,
            created_at_secs: 1729300000,
        };

        add_phrase_cmd(phrase).unwrap();
        let list = list_phrases_cmd(None, false).unwrap();
        assert!(!list.is_empty());
        assert_eq!(list[0].id, "cmd-p1");

        let is_fav = toggle_favorite_phrase_cmd("cmd-p1".to_string()).unwrap();
        assert!(is_fav);

        let conv = start_conversation_session_cmd("en".to_string(), "es".to_string()).unwrap();
        let turn =
            add_conversation_turn_cmd(conv.id, "party_a".to_string(), "Hello".to_string()).unwrap();
        assert_eq!(turn.translated_text.to_lowercase(), "hola");
    }
}
