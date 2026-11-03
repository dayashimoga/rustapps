//! Universal Live Translator - C-ABI Foreign Function Interface (FFI)
//!
//! Conforms to ADR-0015 for cross-platform desktop and mobile integration.
//! Exposes `ult_*` C-ABI endpoints for Flutter mobile (Android/iOS) and Tauri desktop IPC.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;
use tokio::runtime::Runtime;

use crate::models::{
    LanguageCode, PhraseCategory, PhrasebookItem, SubtitleFormat, TranslationRequest,
    TtsSynthesisRequest,
};
use crate::service::TranslatorService;

/// Opaque wrapper for `TranslatorService` across C-ABI boundary
pub struct UltHandle {
    service: TranslatorService,
    _rt: Arc<Runtime>,
}

/// Helper converting Rust serialize type to raw C string pointer
fn to_c_string<T: serde::Serialize>(val: &T) -> *mut c_char {
    let json = serde_json::to_string(val).unwrap_or_else(|_| "{}".to_string());
    CString::new(json)
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// Helper converting error string to raw C string JSON pointer
fn to_error_json(err_msg: &str) -> *mut c_char {
    let err_json = format!("{{\"error\": \"{}\"}}", err_msg.replace('"', "\\\""));
    CString::new(err_json)
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// Create a new `UltHandle` instance
///
/// # Safety
/// `data_dir` must be a valid null-terminated C string or null pointer.
#[no_mangle]
pub unsafe extern "C" fn ult_create_handle(data_dir: *const c_char) -> *mut UltHandle {
    let rt = match Runtime::new() {
        Ok(r) => Arc::new(r),
        Err(_) => return std::ptr::null_mut(),
    };

    let service = if data_dir.is_null() {
        match TranslatorService::open_in_memory() {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        }
    } else {
        // SAFETY: data_dir is verified non-null
        let dir_str = match CStr::from_ptr(data_dir).to_str() {
            Ok(s) if !s.trim().is_empty() => s,
            _ => {
                return match TranslatorService::open_in_memory() {
                    Ok(s) => Box::into_raw(Box::new(UltHandle {
                        service: s,
                        _rt: rt,
                    })),
                    Err(_) => std::ptr::null_mut(),
                };
            }
        };

        match TranslatorService::open(dir_str) {
            Ok(s) => s,
            Err(_) => match TranslatorService::open_in_memory() {
                Ok(s) => s,
                Err(_) => return std::ptr::null_mut(),
            },
        }
    };

    Box::into_raw(Box::new(UltHandle { service, _rt: rt }))
}

/// Free an `UltHandle`
///
/// # Safety
/// `handle` must be a valid non-null pointer returned by `ult_create_handle` and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn ult_free_handle(handle: *mut UltHandle) {
    if !handle.is_null() {
        // SAFETY: Pointer is non-null and was allocated by Box::into_raw
        let _ = Box::from_raw(handle);
    }
}

/// Free a string returned by any `ult_*` function
///
/// # Safety
/// `s` must have been returned by an `ult_*` function and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn ult_free_string(s: *mut c_char) {
    if !s.is_null() {
        // SAFETY: Pointer was returned by CString::into_raw
        let _ = CString::from_raw(s);
    }
}

/// Automatically identify language from input text snippet
///
/// # Safety
/// `handle` and `text` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn ult_detect_language(
    handle: *mut UltHandle,
    text: *const c_char,
) -> *mut c_char {
    if handle.is_null() || text.is_null() {
        return to_error_json("Null pointer passed to ult_detect_language");
    }

    // SAFETY: text is non-null
    let text_str = match CStr::from_ptr(text).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in text"),
    };

    let h = &*handle;
    match h.service.detect_language(text_str) {
        Some(res) => to_c_string(&res),
        None => to_error_json("Could not determine language"),
    }
}

/// Translate text and return TranslationResponse JSON
///
/// # Safety
/// `handle` and `request_json` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn ult_translate_text(
    handle: *mut UltHandle,
    request_json: *const c_char,
) -> *mut c_char {
    if handle.is_null() || request_json.is_null() {
        return to_error_json("Null pointer passed to ult_translate_text");
    }

    // SAFETY: request_json is non-null
    let req_str = match CStr::from_ptr(request_json).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in request JSON"),
    };

    let req: TranslationRequest = match serde_json::from_str(req_str) {
        Ok(r) => r,
        Err(e) => return to_error_json(&format!("Failed to parse TranslationRequest: {}", e)),
    };

    let h = &*handle;
    match h.service.translate_text(&req) {
        Ok(res) => to_c_string(&res),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Synthesize speech to playable 22.05kHz WAV bytes returning TtsSynthesisResponse JSON
///
/// # Safety
/// `handle` and `request_json` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn ult_synthesize_speech(
    handle: *mut UltHandle,
    request_json: *const c_char,
) -> *mut c_char {
    if handle.is_null() || request_json.is_null() {
        return to_error_json("Null pointer passed to ult_synthesize_speech");
    }

    // SAFETY: request_json is non-null
    let req_str = match CStr::from_ptr(request_json).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in request JSON"),
    };

    let req: TtsSynthesisRequest = match serde_json::from_str(req_str) {
        Ok(r) => r,
        Err(e) => return to_error_json(&format!("Failed to parse TtsSynthesisRequest: {}", e)),
    };

    let h = &*handle;
    match h.service.synthesize_speech(&req) {
        Ok(res) => to_c_string(&res),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Translate camera or file image bytes via OCR returning CameraTranslationResult JSON
///
/// # Safety
/// `handle`, `image_bytes`, and `target_lang` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn ult_translate_camera_image(
    handle: *mut UltHandle,
    image_bytes: *const u8,
    length: usize,
    source_lang: *const c_char,
    target_lang: *const c_char,
) -> *mut c_char {
    if handle.is_null() || image_bytes.is_null() || length == 0 || target_lang.is_null() {
        return to_error_json("Null or invalid parameters passed to ult_translate_camera_image");
    }

    // SAFETY: image_bytes points to length valid bytes
    let slice = std::slice::from_raw_parts(image_bytes, length);

    let src = if !source_lang.is_null() {
        // SAFETY: source_lang is non-null
        CStr::from_ptr(source_lang)
            .to_str()
            .ok()
            .and_then(LanguageCode::parse)
    } else {
        None
    };

    // SAFETY: target_lang is non-null
    let tgt_str = match CStr::from_ptr(target_lang).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in target_lang"),
    };
    let tgt = LanguageCode::parse(tgt_str).unwrap_or(LanguageCode::Es);

    let h = &*handle;
    match h.service.translate_camera_image(slice, src, tgt) {
        Ok(res) => to_c_string(&res),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Translate subtitles (.srt or .vtt) returning translated subtitle content
///
/// # Safety
/// `handle`, `content`, and `target_lang` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn ult_translate_subtitles(
    handle: *mut UltHandle,
    content: *const c_char,
    format: *const c_char,
    source_lang: *const c_char,
    target_lang: *const c_char,
) -> *mut c_char {
    if handle.is_null() || content.is_null() || target_lang.is_null() {
        return to_error_json("Null pointer passed to ult_translate_subtitles");
    }

    // SAFETY: pointers are verified non-null
    let content_str = match CStr::from_ptr(content).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in content"),
    };

    let fmt = if !format.is_null() {
        match CStr::from_ptr(format).to_str() {
            Ok(s) if s.eq_ignore_ascii_case("vtt") => SubtitleFormat::Vtt,
            _ => SubtitleFormat::Srt,
        }
    } else {
        SubtitleFormat::Srt
    };

    let src = if !source_lang.is_null() {
        CStr::from_ptr(source_lang)
            .to_str()
            .ok()
            .and_then(LanguageCode::parse)
    } else {
        None
    };

    let tgt_str = match CStr::from_ptr(target_lang).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in target_lang"),
    };
    let tgt = LanguageCode::parse(tgt_str).unwrap_or(LanguageCode::Es);

    let h = &*handle;
    match h
        .service
        .translate_subtitle_content(content_str, fmt, src, tgt)
    {
        Ok(out) => {
            let res = serde_json::json!({ "translated_content": out });
            to_c_string(&res)
        }
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// List phrasebook items returning Vec<PhrasebookItem> JSON
///
/// # Safety
/// `handle` must be a valid non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn ult_list_phrases(
    handle: *mut UltHandle,
    category: *const c_char,
    favorite_only: i32,
) -> *mut c_char {
    if handle.is_null() {
        return to_error_json("Null pointer passed to ult_list_phrases");
    }

    let cat = if !category.is_null() {
        // SAFETY: category is non-null
        if let Ok(s) = CStr::from_ptr(category).to_str() {
            serde_json::from_str::<PhraseCategory>(&format!("\"{}\"", s)).ok()
        } else {
            None
        }
    } else {
        None
    };

    let h = &*handle;
    match h.service.list_phrases(cat, favorite_only != 0) {
        Ok(items) => to_c_string(&items),
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Add phrasebook item
///
/// # Safety
/// `handle` and `item_json` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn ult_add_phrase(
    handle: *mut UltHandle,
    item_json: *const c_char,
) -> *mut c_char {
    if handle.is_null() || item_json.is_null() {
        return to_error_json("Null pointer passed to ult_add_phrase");
    }

    // SAFETY: item_json is non-null
    let str_val = match CStr::from_ptr(item_json).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in item_json"),
    };

    let item: PhrasebookItem = match serde_json::from_str(str_val) {
        Ok(it) => it,
        Err(e) => return to_error_json(&format!("Failed to parse PhrasebookItem: {}", e)),
    };

    let h = &*handle;
    match h.service.add_phrase(&item) {
        Ok(_) => {
            let res = serde_json::json!({ "success": true, "id": item.id });
            to_c_string(&res)
        }
        Err(e) => to_error_json(&e.to_string()),
    }
}

/// Toggle favorite status of a phrasebook item (returns 1 for favorite, 0 for not, -1 for error)
///
/// # Safety
/// `handle` and `id` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn ult_toggle_favorite_phrase(
    handle: *mut UltHandle,
    id: *const c_char,
) -> i32 {
    if handle.is_null() || id.is_null() {
        return -1;
    }

    // SAFETY: id is non-null
    let id_str = match CStr::from_ptr(id).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let h = &*handle;
    match h.service.toggle_favorite_phrase(id_str) {
        Ok(fav) => {
            if fav {
                1
            } else {
                0
            }
        }
        Err(_) => -1,
    }
}

/// Delete a phrasebook item (returns 1 if deleted, 0 if not found, -1 for error)
///
/// # Safety
/// `handle` and `id` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn ult_delete_phrase(handle: *mut UltHandle, id: *const c_char) -> i32 {
    if handle.is_null() || id.is_null() {
        return -1;
    }

    // SAFETY: id is non-null
    let id_str = match CStr::from_ptr(id).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let h = &*handle;
    match h.service.delete_phrase(id_str) {
        Ok(deleted) => {
            if deleted {
                1
            } else {
                0
            }
        }
        Err(_) => -1,
    }
}

/// Start a split-screen conversation session returning ConversationSession JSON
///
/// # Safety
/// `handle`, `party_a`, and `party_b` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn ult_start_conversation(
    handle: *mut UltHandle,
    party_a: *const c_char,
    party_b: *const c_char,
) -> *mut c_char {
    if handle.is_null() || party_a.is_null() || party_b.is_null() {
        return to_error_json("Null pointer passed to ult_start_conversation");
    }

    // SAFETY: pointers are verified non-null
    let a_str = match CStr::from_ptr(party_a).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in party_a"),
    };
    let b_str = match CStr::from_ptr(party_b).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in party_b"),
    };

    let a_lang = LanguageCode::parse(a_str).unwrap_or(LanguageCode::En);
    let b_lang = LanguageCode::parse(b_str).unwrap_or(LanguageCode::Es);

    let h = &*handle;
    let session = h.service.create_conversation_session(a_lang, b_lang);
    to_c_string(&session)
}

/// Add turn to active conversation session returning ConversationTurn JSON
///
/// # Safety
/// `handle`, `session_id`, `speaker`, and `text` must be valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn ult_add_conversation_turn(
    handle: *mut UltHandle,
    session_id: *const c_char,
    speaker: *const c_char,
    text: *const c_char,
) -> *mut c_char {
    if handle.is_null() || session_id.is_null() || speaker.is_null() || text.is_null() {
        return to_error_json("Null pointer passed to ult_add_conversation_turn");
    }

    // SAFETY: pointers are non-null
    let sess_str = match CStr::from_ptr(session_id).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in session_id"),
    };
    let spk_str = match CStr::from_ptr(speaker).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in speaker"),
    };
    let text_str = match CStr::from_ptr(text).to_str() {
        Ok(s) => s,
        Err(_) => return to_error_json("Invalid UTF-8 in text"),
    };

    let h = &*handle;
    match h.service.add_conversation_turn(sess_str, spk_str, text_str) {
        Ok(turn) => to_c_string(&turn),
        Err(e) => to_error_json(&e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_lifecycle_and_core_operations() {
        unsafe {
            // 1. Create handle
            let handle = ult_create_handle(std::ptr::null());
            assert!(!handle.is_null());

            // 2. LID detection via FFI
            let c_text = CString::new("Bonjour tout le monde").unwrap();
            let lid_ptr = ult_detect_language(handle, c_text.as_ptr());
            assert!(!lid_ptr.is_null());
            let lid_str = CStr::from_ptr(lid_ptr).to_str().unwrap();
            assert!(lid_str.contains("\"language\":\"fr\""));
            ult_free_string(lid_ptr);

            // 3. Text translation via FFI
            let req_json = CString::new(
                r#"{"source_lang": "en", "target_lang": "es", "text": "Hello world", "preserve_formatting": true, "domain_glossary": null}"#
            ).unwrap();
            let trans_ptr = ult_translate_text(handle, req_json.as_ptr());
            assert!(!trans_ptr.is_null());
            let trans_str = CStr::from_ptr(trans_ptr).to_str().unwrap();
            assert!(trans_str.contains("Hola mundo"));
            ult_free_string(trans_ptr);

            // 4. Phrasebook addition via FFI
            let item_json = CString::new(
                r#"{"id": "p-ffi-1", "category": "travel", "source_text": "Airport", "source_lang": "en", "target_text": "Aeropuerto", "target_lang": "es", "notes": null, "is_favorite": false, "created_at_secs": 1729300000}"#
            ).unwrap();
            let add_ptr = ult_add_phrase(handle, item_json.as_ptr());
            assert!(!add_ptr.is_null());
            ult_free_string(add_ptr);

            // 5. Toggle favorite
            let c_id = CString::new("p-ffi-1").unwrap();
            let fav_res = ult_toggle_favorite_phrase(handle, c_id.as_ptr());
            assert_eq!(fav_res, 1);

            // 6. Start conversation
            let party_a = CString::new("en").unwrap();
            let party_b = CString::new("es").unwrap();
            let conv_ptr = ult_start_conversation(handle, party_a.as_ptr(), party_b.as_ptr());
            assert!(!conv_ptr.is_null());
            let conv_str = CStr::from_ptr(conv_ptr).to_str().unwrap();
            assert!(conv_str.contains("Conversation"));
            ult_free_string(conv_ptr);

            // 7. Free handle
            ult_free_handle(handle);
        }
    }
}
