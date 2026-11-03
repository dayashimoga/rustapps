#![forbid(unsafe_code)]
//! Tauri Desktop IPC Command Handlers for Universal Live Translator
//!
//! Provides typed command interfaces invoked by the desktop Svelte/TypeScript UI.

use crate::models::{
    CameraTranslationResult, ConversationSession, ConversationTurn, GlossaryTerm, LanguageCode,
    PhraseCategory, PhrasebookItem, SubtitleFormat, TranslationRequest, TranslationResponse,
    TtsSynthesisRequest, TtsSynthesisResponse,
};
use crate::service::TranslatorService;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static SERVICE: OnceLock<Arc<Mutex<TranslatorService>>> = OnceLock::new();

pub fn get_service() -> Arc<Mutex<TranslatorService>> {
    SERVICE
        .get_or_init(|| {
            let svc = platform_common::AppPaths::from_app_name("universal-live-translator")
                .and_then(|p| {
                    p.ensure_directories()?;
                    TranslatorService::open(&p.data_dir)
                        .map_err(|e| platform_common::PlatformError::Unsupported(e.to_string()))
                })
                .unwrap_or_else(|_| {
                    TranslatorService::open_in_memory()
                        .expect("Failed to initialize fallback TranslatorService")
                });
            let _ = svc.add_glossary_term(
                "medical",
                &GlossaryTerm {
                    source_term: "myocardial infarction".to_string(),
                    target_term: "infarto de miocardio".to_string(),
                    case_sensitive: false,
                },
            );
            let _ = svc.add_glossary_term(
                "software",
                &GlossaryTerm {
                    source_term: "zero-copy memory map".to_string(),
                    target_term: "mapa de memoria sin copia".to_string(),
                    case_sensitive: false,
                },
            );
            Arc::new(Mutex::new(svc))
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

/// Camera OCR & Overlay Translation command
pub fn translate_camera_image_cmd(
    image_bytes: Vec<u8>,
    source_lang_str: Option<String>,
    target_lang_str: String,
) -> Result<(CameraTranslationResult, Vec<u8>), String> {
    let service = get_service();
    let svc = service.lock().map_err(|e| e.to_string())?;
    let source_lang = source_lang_str.and_then(|s| LanguageCode::parse(&s));
    let target_lang = LanguageCode::parse(&target_lang_str).unwrap_or(LanguageCode::Es);
    let result = svc
        .translate_camera_image(&image_bytes, source_lang, target_lang)
        .map_err(|e| e.to_string())?;
    let overlay_bmp = svc
        .render_camera_overlay_bmp(&image_bytes, &result)
        .unwrap_or_default();
    Ok((result, overlay_bmp))
}

/// List all glossary domains
pub fn list_glossary_domains_cmd() -> Result<Vec<String>, String> {
    Ok(vec![
        "medical".to_string(),
        "software".to_string(),
        "legal".to_string(),
        "aviation".to_string(),
    ])
}

fn base64_encode(bytes: &[u8]) -> String {
    const B64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut enc = Vec::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
        enc.push(B64_CHARS[(b0 >> 2) as usize]);
        enc.push(B64_CHARS[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize]);
        if chunk.len() > 1 {
            enc.push(B64_CHARS[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize]);
        } else {
            enc.push(b'=');
        }
        if chunk.len() > 2 {
            enc.push(B64_CHARS[(b2 & 0x3f) as usize]);
        } else {
            enc.push(b'=');
        }
    }
    String::from_utf8(enc).unwrap_or_default()
}

fn parse_phrase_category(s: Option<&str>) -> Option<PhraseCategory> {
    match s.unwrap_or("").trim().to_lowercase().as_str() {
        "greeting" | "greetings" => Some(PhraseCategory::Greetings),
        "travel" => Some(PhraseCategory::Travel),
        "dining" => Some(PhraseCategory::Dining),
        "emergency" => Some(PhraseCategory::Emergency),
        "medical" => Some(PhraseCategory::Medical),
        "business" => Some(PhraseCategory::Business),
        "technical" => Some(PhraseCategory::Technical),
        "custom" => Some(PhraseCategory::Custom),
        _ => None,
    }
}

fn category_display_name(cat: PhraseCategory) -> &'static str {
    match cat {
        PhraseCategory::Greetings => "Greeting",
        PhraseCategory::Travel => "Travel",
        PhraseCategory::Dining => "Dining",
        PhraseCategory::Emergency => "Emergency",
        PhraseCategory::Medical => "Medical",
        PhraseCategory::Business => "Business",
        PhraseCategory::Technical => "Technical",
        PhraseCategory::Custom => "Custom",
    }
}

/// Dispatch desktop IPC commands from the Svelte 5 translator frontend
pub fn dispatch_ipc_command(cmd: &str, args: Value) -> Result<Value, String> {
    match cmd {
        "detect_language_cmd" => {
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let t0 = std::time::Instant::now();
            let lid = detect_language_cmd(text)?;
            let latency_us = t0.elapsed().as_micros() as u64;
            if let Some(r) = lid {
                Ok(json!({
                    "language": r.language.as_str(),
                    "confidence": r.confidence,
                    "detectedScript": format!("{:?}", r.script),
                    "latencyUs": latency_us
                }))
            } else {
                Ok(Value::Null)
            }
        }
        "translate_text_cmd" => {
            let req_obj = args.get("request").unwrap_or(&args);
            let text = req_obj
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let source_lang = req_obj
                .get("sourceLang")
                .or_else(|| req_obj.get("source_lang"))
                .and_then(|v| v.as_str())
                .and_then(LanguageCode::parse);
            let target_lang = req_obj
                .get("targetLang")
                .or_else(|| req_obj.get("target_lang"))
                .and_then(|v| v.as_str())
                .and_then(LanguageCode::parse)
                .unwrap_or(LanguageCode::Es);
            let preserve_formatting = req_obj
                .get("preserveFormatting")
                .or_else(|| req_obj.get("preserve_formatting"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let domain_glossary = req_obj
                .get("domainGlossary")
                .or_else(|| req_obj.get("domain_glossary"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.to_string());

            let req = TranslationRequest {
                source_lang,
                target_lang,
                text,
                preserve_formatting,
                domain_glossary,
            };
            let res = translate_text_cmd(req)?;
            Ok(json!({
                "originalText": res.original_text,
                "translatedText": res.translated_text,
                "sourceLang": res.source_lang.as_str(),
                "targetLang": res.target_lang.as_str(),
                "confidence": res.confidence,
                "latencyMs": res.latency_ms,
                "detectedSource": res.detected_source,
                "wordCount": res.word_count,
                "glossaryTermsApplied": []
            }))
        }
        "synthesize_speech_cmd" => {
            let req_obj = args.get("request").unwrap_or(&args);
            let text = req_obj
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("Hello")
                .to_string();
            let language = req_obj
                .get("language")
                .and_then(|v| v.as_str())
                .and_then(LanguageCode::parse)
                .unwrap_or(LanguageCode::En);
            let speaking_rate = req_obj
                .get("speakingRate")
                .or_else(|| req_obj.get("speaking_rate"))
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as f32;
            let pitch = req_obj
                .get("pitchSemitones")
                .or_else(|| req_obj.get("pitch"))
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as f32;
            let tts_req = TtsSynthesisRequest {
                text,
                language,
                speaking_rate,
                pitch,
                sample_rate: 22050,
            };
            let res = synthesize_speech_cmd(tts_req)?;
            Ok(json!({
                "sampleRate": res.sample_rate,
                "channels": 1,
                "durationMs": (res.duration_secs * 1000.0) as u64,
                "audioWavBase64": base64_encode(&res.wav_bytes)
            }))
        }
        "translate_subtitles_cmd" => {
            let content = args
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let format_str = args
                .get("formatStr")
                .or_else(|| args.get("format_str"))
                .and_then(|v| v.as_str())
                .unwrap_or("srt")
                .to_string();
            let source_lang_str = args
                .get("sourceLangStr")
                .or_else(|| args.get("source_lang_str"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let target_lang_str = args
                .get("targetLangStr")
                .or_else(|| args.get("target_lang_str"))
                .and_then(|v| v.as_str())
                .unwrap_or("es")
                .to_string();
            let res =
                translate_subtitles_cmd(content, format_str, source_lang_str, target_lang_str)?;
            Ok(json!(res))
        }
        "translate_document_cmd" => {
            let content = args
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let format = args
                .get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("md")
                .to_string();
            let source_lang_str = args
                .get("sourceLangStr")
                .or_else(|| args.get("source_lang_str"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let target_lang_str = args
                .get("targetLangStr")
                .or_else(|| args.get("target_lang_str"))
                .and_then(|v| v.as_str())
                .unwrap_or("es")
                .to_string();
            let res = translate_document_cmd(content, format, source_lang_str, target_lang_str)?;
            Ok(json!(res))
        }
        "translate_camera_image_cmd" => {
            let image_bytes: Vec<u8> = args
                .get("imageBytes")
                .or_else(|| args.get("image_bytes"))
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().map(|x| x.as_u64().unwrap_or(0) as u8).collect())
                .unwrap_or_default();
            let source_lang_str = args
                .get("sourceLangStr")
                .or_else(|| args.get("source_lang_str"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let target_lang_str = args
                .get("targetLangStr")
                .or_else(|| args.get("target_lang_str"))
                .and_then(|v| v.as_str())
                .unwrap_or("es")
                .to_string();
            let (res, bmp) =
                translate_camera_image_cmd(image_bytes, source_lang_str, target_lang_str)?;
            let boxes: Vec<Value> = res
                .boxes
                .into_iter()
                .map(|b| {
                    json!({
                        "x": b.x,
                        "y": b.y,
                        "width": b.width,
                        "height": b.height,
                        "originalText": b.original_text,
                        "translatedText": b.translated_text,
                        "confidence": b.confidence
                    })
                })
                .collect();
            Ok(json!({
                "boxes": boxes,
                "overlayImageBase64": base64_encode(&bmp),
                "sourceLang": res.source_lang.as_str(),
                "targetLang": res.target_lang.as_str(),
                "latencyMs": res.processing_time_ms
            }))
        }
        "list_phrases_cmd" => {
            let cat_str = args
                .get("categoryStr")
                .or_else(|| args.get("category_str"))
                .and_then(|v| v.as_str());
            let favorite_only = args
                .get("favoriteOnly")
                .or_else(|| args.get("favorite_only"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let category = parse_phrase_category(cat_str);
            let service = get_service();
            let svc = service.lock().map_err(|e| e.to_string())?;
            let list = svc
                .list_phrases(category, favorite_only)
                .map_err(|e| e.to_string())?;
            let mapped: Vec<Value> = list
                .into_iter()
                .map(|p| {
                    json!({
                        "id": p.id,
                        "category": category_display_name(p.category),
                        "sourceLang": p.source_lang.as_str(),
                        "targetLang": p.target_lang.as_str(),
                        "sourceText": p.source_text,
                        "targetText": p.target_text,
                        "notes": p.notes,
                        "favorite": p.is_favorite,
                        "createdAt": p.created_at_secs
                    })
                })
                .collect();
            Ok(Value::Array(mapped))
        }
        "add_phrase_cmd" => {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let id = args
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("phr_{}", now));
            let cat_str = args
                .get("categoryStr")
                .or_else(|| args.get("category"))
                .and_then(|v| v.as_str());
            let category = parse_phrase_category(cat_str).unwrap_or(PhraseCategory::Custom);
            let source_lang = args
                .get("sourceLangStr")
                .or_else(|| args.get("sourceLang"))
                .or_else(|| args.get("source_lang"))
                .and_then(|v| v.as_str())
                .and_then(LanguageCode::parse)
                .unwrap_or(LanguageCode::En);
            let target_lang = args
                .get("targetLangStr")
                .or_else(|| args.get("targetLang"))
                .or_else(|| args.get("target_lang"))
                .and_then(|v| v.as_str())
                .and_then(LanguageCode::parse)
                .unwrap_or(LanguageCode::Es);
            let source_text = args
                .get("sourceText")
                .or_else(|| args.get("source_text"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let target_text = args
                .get("targetText")
                .or_else(|| args.get("target_text"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let notes = args
                .get("notes")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let item = PhrasebookItem {
                id,
                category,
                source_text,
                source_lang,
                target_text,
                target_lang,
                notes,
                is_favorite: false,
                created_at_secs: now,
            };
            add_phrase_cmd(item)?;
            Ok(json!(true))
        }
        "toggle_favorite_phrase_cmd" => {
            let id = args
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let fav = toggle_favorite_phrase_cmd(id)?;
            Ok(json!(fav))
        }
        "delete_phrase_cmd" => {
            let id = args
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let deleted = delete_phrase_cmd(id)?;
            Ok(json!(deleted))
        }
        "get_history_cmd" => {
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
            let list = get_history_cmd(limit)?;
            let mapped: Vec<Value> = list
                .into_iter()
                .map(|h| {
                    json!({
                        "id": h.id,
                        "sourceLang": h.source_lang.as_str(),
                        "targetLang": h.target_lang.as_str(),
                        "sourceText": h.original_text,
                        "targetText": h.translated_text,
                        "timestamp": h.timestamp_secs,
                        "favorite": false
                    })
                })
                .collect();
            Ok(Value::Array(mapped))
        }
        "clear_history_cmd" => {
            clear_history_cmd()?;
            Ok(json!(true))
        }
        "list_glossary_domains_cmd" => {
            let domains = list_glossary_domains_cmd()?;
            Ok(json!(domains))
        }
        "get_glossary_terms_cmd" => {
            let domain = args
                .get("domain")
                .and_then(|v| v.as_str())
                .unwrap_or("medical")
                .to_string();
            let terms = get_glossary_terms_cmd(domain)?;
            let mapped: Vec<Value> = terms
                .into_iter()
                .map(|t| {
                    json!({
                        "sourceTerm": t.source_term,
                        "targetTerm": t.target_term,
                        "caseSensitive": t.case_sensitive
                    })
                })
                .collect();
            Ok(Value::Array(mapped))
        }
        "add_glossary_term_cmd" => {
            let domain = args
                .get("domain")
                .and_then(|v| v.as_str())
                .unwrap_or("medical")
                .to_string();
            let source_term = args
                .get("sourceTerm")
                .or_else(|| args.get("source_term"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let target_term = args
                .get("targetTerm")
                .or_else(|| args.get("target_term"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let case_sensitive = args
                .get("caseSensitive")
                .or_else(|| args.get("case_sensitive"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            add_glossary_term_cmd(
                domain,
                GlossaryTerm {
                    source_term,
                    target_term,
                    case_sensitive,
                },
            )?;
            Ok(json!(true))
        }
        "start_conversation_session_cmd" => {
            let party_a_lang = args
                .get("partyALangStr")
                .or_else(|| args.get("party_a_lang"))
                .and_then(|v| v.as_str())
                .unwrap_or("en")
                .to_string();
            let party_b_lang = args
                .get("partyBLangStr")
                .or_else(|| args.get("party_b_lang"))
                .and_then(|v| v.as_str())
                .unwrap_or("es")
                .to_string();
            let s = start_conversation_session_cmd(party_a_lang, party_b_lang)?;
            Ok(json!({
                "id": s.id,
                "partyALang": s.party_a_lang.as_str(),
                "partyBLang": s.party_b_lang.as_str(),
                "turns": [],
                "createdAt": s.created_at_secs
            }))
        }
        "add_conversation_turn_cmd" => {
            let session_id = args
                .get("sessionId")
                .or_else(|| args.get("session_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let speaker = args
                .get("speaker")
                .and_then(|v| v.as_str())
                .unwrap_or("party_a")
                .to_string();
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let turn = add_conversation_turn_cmd(session_id.clone(), speaker, text)?;
            Ok(json!({
                "id": turn.id,
                "sessionId": session_id,
                "speaker": turn.speaker,
                "originalText": turn.original_text,
                "translatedText": turn.translated_text,
                "sourceLang": turn.source_lang.as_str(),
                "targetLang": turn.target_lang.as_str(),
                "timestamp": turn.timestamp_secs
            }))
        }
        _ => Err(format!("Unknown Translator IPC command: {}", cmd)),
    }
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
