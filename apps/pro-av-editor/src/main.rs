#![deny(unsafe_code)]
//! Professional Audio & Video Editor Desktop GUI & CLI Entrypoint

use pave_core::{
    commands::dispatch_ipc_command, AnimatedProperty, BlendMode, Clip, ClipType,
    ColorGradingSettings, CompressorSettings, EditorService, ParametricEqSettings, Transform2D,
};
use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use serde_json::{json, Value};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "pro-av-editor",
    app_title: "Pro AV Studio — 100% Offline Non-Destructive NLE",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1520,
    window_height: 940,
};

struct ProAvDesktopHandler;

impl DesktopIpcHandler for ProAvDesktopHandler {
    fn handle_command(&self, command: &str, args: Value) -> Result<Value, String> {
        dispatch_ipc_command(command, args)
    }

    fn self_test_checks(&self) -> Vec<SelfTestCheck> {
        vec![
            SelfTestCheck {
                command: "get_project_state_cmd",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "render_preview_frame_cmd",
                payload: json!({ "timestampMs": 1000, "width": 160, "height": 90 }),
            },
            SelfTestCheck {
                command: "get_video_scopes_cmd",
                payload: json!({ "timestampMs": 1000, "width": 160, "height": 90 }),
            },
            SelfTestCheck {
                command: "get_audio_loudness_cmd",
                payload: json!({ "startMs": 0, "endMs": 1500 }),
            },
            SelfTestCheck {
                command: "ai_detect_beats_cmd",
                payload: json!({ "threshold": 0.25 }),
            },
            SelfTestCheck {
                command: "ai_detect_scene_cuts_cmd",
                payload: json!({ "threshold": 25.0 }),
            },
            SelfTestCheck {
                command: "ai_generate_auto_captions_cmd",
                payload: json!({ "sampleRate": 48000 }),
            },
            SelfTestCheck {
                command: "export_edl_cmd",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "export_fcpxml_cmd",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "get_export_presets_cmd",
                payload: json!({}),
            },
        ]
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("pro-av-editor v{}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "Professional Audio & Video Editor v{}",
            env!("CARGO_PKG_VERSION")
        );
        println!("Usage: pro-av-editor [OPTIONS]");
        println!("  (no args)           Launch standalone desktop Studio GUI");
        println!("  --self-test         Verify embedded UI assets and live IPC commands");
        println!("  --headless-server   Start loopback GUI/IPC server without browser window");
        println!("  --cli               Execute headless CLI timeline diagnostics");
        println!("  --version, -v       Show version");
        return;
    }

    if args.iter().any(|a| a == "--cli") {
        run_cli_diagnostics();
        return;
    }

    if let Err(e) = run_desktop_app(UI_BUNDLE, ProAvDesktopHandler) {
        eprintln!("Pro AV Editor desktop runtime error: {}", e);
        std::process::exit(1);
    }
}

fn run_cli_diagnostics() {
    println!("========================================================");
    println!("  Professional Audio & Video Editor - 100% Offline NLE ");
    println!("========================================================");

    let service = match EditorService::from_template("tpl_standard_video", "Flagship Studio Demo") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize EditorService: {e}");
            return;
        }
    };

    if let Ok(proj) = service.get_project() {
        println!(
            "Project Initialized: \"{}\" (Tracks: {}, Canvas: {}x{} @ {:.2} FPS)",
            proj.name,
            proj.tracks.len(),
            proj.settings.width,
            proj.settings.height,
            proj.settings.frame_rate.as_f64()
        );
    }

    let clip1 = Clip {
        id: "clip-demo-1".to_string(),
        name: "Main Camera A-Roll".to_string(),
        clip_type: ClipType::Video,
        track_id: "v1".to_string(),
        source_id: Some("media_raw_001".to_string()),
        source_in_ms: 0,
        source_out_ms: 6000,
        timeline_in_ms: 0,
        timeline_out_ms: 6000,
        speed_multiplier: 1.0,
        volume: AnimatedProperty::constant(1.0),
        pan: 0.0,
        opacity: AnimatedProperty::constant(1.0),
        blend_mode: BlendMode::Normal,
        transform: Transform2D::default(),
        color_grading: ColorGradingSettings::default(),
        eq_settings: ParametricEqSettings::default(),
        compressor: CompressorSettings::default(),
    };

    if let Err(e) = service.add_clip(clip1) {
        eprintln!("Error adding clip: {e}");
    } else {
        println!("Successfully added clip on track V1 (Duration: 6.00s)");
    }

    if let Ok(frame) = service.render_frame(2500, 320, 180) {
        println!(
            "Rendered preview frame at 2.50s ({}x{}, {} bytes)",
            frame.width,
            frame.height,
            frame.data.len()
        );
    }

    if let Ok(scopes) = service.get_video_scopes(2500, 320, 180) {
        println!(
            "Video Scopes computed: Waveform {} cols x {} rows, Vectorscope {}x{}",
            scopes.waveform_cols,
            scopes.waveform_rows,
            scopes.vectorscope_dim,
            scopes.vectorscope_dim
        );
    }

    if let Ok(edl) = service.export_edl() {
        println!(
            "\nGenerated CMX 3600 Edit Decision List (EDL):\n{}",
            edl.trim()
        );
    }
}
