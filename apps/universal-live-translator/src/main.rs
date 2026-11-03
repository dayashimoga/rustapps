#![deny(unsafe_code)]
//! Universal Live Translator Desktop / CLI Entrypoint

use ult_core::{LanguageCode, TranslationRequest, TranslatorService};

fn main() {
    println!("========================================================");
    println!("  Universal Live Translator - 100% Offline Multi-Engine ");
    println!("========================================================");

    let service = match TranslatorService::open_in_memory() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize TranslatorService: {e}");
            return;
        }
    };

    let sample_text = "Hello world, welcome to real-time offline translation.";
    if let Some(res) = service.detect_language(sample_text) {
        println!(
            "Input Text: \"{}\"\nDetected: {} ({}) with confidence {:.2}%",
            sample_text,
            res.language.display_name(),
            res.language.as_str(),
            res.confidence * 100.0
        );
    }

    let req = TranslationRequest {
        text: sample_text.to_string(),
        source_lang: Some(LanguageCode::En),
        target_lang: LanguageCode::Es,
        preserve_formatting: false,
        domain_glossary: None,
    };

    match service.translate_text(&req) {
        Ok(resp) => {
            println!(
                "Spanish Translation: \"{}\" (in {:.2} ms)",
                resp.translated_text, resp.latency_ms
            );
        }
        Err(e) => eprintln!("Translation error: {e}"),
    }

    let de_req = TranslationRequest {
        text: "Guten Tag, wie geht es Ihnen heute?".to_string(),
        source_lang: None,
        target_lang: LanguageCode::En,
        preserve_formatting: false,
        domain_glossary: None,
    };

    match service.translate_text(&de_req) {
        Ok(resp) => {
            println!(
                "German -> English: \"{}\" (detected {:?})",
                resp.translated_text, resp.source_lang
            );
        }
        Err(e) => eprintln!("Translation error: {e}"),
    }

    println!("Translator core initialized and operational.");
}
