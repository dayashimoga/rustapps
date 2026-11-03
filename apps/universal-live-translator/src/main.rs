#![deny(unsafe_code)]
//! Universal Live Translator Desktop GUI & CLI Entrypoint

use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use serde_json::{json, Value};
use ult_core::{
    commands::dispatch_ipc_command, LanguageCode, TranslationRequest, TranslatorService,
};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "universal-live-translator",
    app_title: "Universal Live Translator — 100% Offline NMT, ASR, TTS & OCR",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1420,
    window_height: 900,
};

struct TranslatorDesktopHandler;

impl DesktopIpcHandler for TranslatorDesktopHandler {
    fn handle_command(&self, command: &str, args: Value) -> Result<Value, String> {
        dispatch_ipc_command(command, args)
    }

    fn self_test_checks(&self) -> Vec<SelfTestCheck> {
        vec![
            SelfTestCheck {
                command: "detect_language_cmd",
                payload: json!({ "text": "Bonjour tout le monde" }),
            },
            SelfTestCheck {
                command: "translate_text_cmd",
                payload: json!({
                    "request": {
                        "text": "Hello world",
                        "sourceLang": "en",
                        "targetLang": "es",
                        "preserveFormatting": true
                    }
                }),
            },
            SelfTestCheck {
                command: "synthesize_speech_cmd",
                payload: json!({
                    "request": {
                        "text": "Hola mundo",
                        "language": "es",
                        "speakingRate": 1.0
                    }
                }),
            },
            SelfTestCheck {
                command: "list_phrases_cmd",
                payload: json!({ "categoryStr": null, "favoriteOnly": false }),
            },
            SelfTestCheck {
                command: "list_glossary_domains_cmd",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "start_conversation_session_cmd",
                payload: json!({ "partyALangStr": "en", "partyBLangStr": "es" }),
            },
            SelfTestCheck {
                command: "translate_subtitles_cmd",
                payload: json!({
                    "content": "1\n00:00:01,000 --> 00:00:03,000\nHello world\n",
                    "formatStr": "srt",
                    "sourceLangStr": "en",
                    "targetLangStr": "es"
                }),
            },
            SelfTestCheck {
                command: "translate_document_cmd",
                payload: json!({
                    "content": "# Document Title\nHello world",
                    "format": "md",
                    "sourceLangStr": "en",
                    "targetLangStr": "es"
                }),
            },
        ]
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("universal-live-translator v{}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Universal Live Translator v{}", env!("CARGO_PKG_VERSION"));
        println!("Usage: universal-live-translator [OPTIONS]");
        println!("  (no args)           Launch standalone desktop Translator Studio GUI");
        println!("  --self-test         Verify embedded UI assets and live IPC commands");
        println!("  --headless-server   Start loopback GUI/IPC server without browser window");
        println!("  --cli               Execute headless CLI translation diagnostics");
        println!("  --version, -v       Show version");
        return;
    }

    if args.iter().any(|a| a == "--cli") {
        run_cli_diagnostics();
        return;
    }

    if let Err(e) = run_desktop_app(UI_BUNDLE, TranslatorDesktopHandler) {
        eprintln!("Universal Live Translator desktop runtime error: {}", e);
        std::process::exit(1);
    }
}

fn run_cli_diagnostics() {
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

    if let Ok(resp) = service.translate_text(&req) {
        println!(
            "Spanish Translation: \"{}\" (in {:.2} ms)",
            resp.translated_text, resp.latency_ms
        );
    }
}
