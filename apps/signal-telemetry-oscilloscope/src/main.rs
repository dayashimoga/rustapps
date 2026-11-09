#![deny(unsafe_code)]
//! Offline Scientific Signal & Telemetry Oscilloscope Desktop GUI & CLI Entrypoint

use platform_common::{run_desktop_app, DesktopIpcHandler, EmbeddedUiBundle, SelfTestCheck};
use serde_json::{json, Value};
use signal_telemetry_oscilloscope::commands::{dispatch_ipc_command, get_waveform_binary};

const UI_BUNDLE: EmbeddedUiBundle = EmbeddedUiBundle {
    app_id: "signal-telemetry-oscilloscope",
    app_title:
        "Scientific Signal Oscilloscope — 100% Offline Digital Signal Processing, FFT & Protocol Decoding",
    version: env!("CARGO_PKG_VERSION"),
    index_html: include_str!("../ui/dist/index.html"),
    app_js: include_bytes!("../ui/dist/assets/app.js"),
    app_css: include_bytes!("../ui/dist/assets/app.css"),
    window_width: 1540,
    window_height: 940,
};

struct OscilloscopeDesktopHandler;

impl DesktopIpcHandler for OscilloscopeDesktopHandler {
    fn handle_command(&self, command: &str, args: Value) -> Result<Value, String> {
        dispatch_ipc_command(command, args)
    }

    fn handle_binary_command(
        &self,
        command: &str,
        args: Value,
        payload: &[u8],
    ) -> Result<Vec<u8>, String> {
        match command {
            "get_channel_waveform_bin" => {
                let session_id = args["session_id"].as_str().unwrap_or("latest");
                let channel = args["channel"].as_str().unwrap_or("ch1");
                get_waveform_binary(session_id, channel)
            }
            _ => Ok(payload.to_vec()),
        }
    }

    fn self_test_checks(&self) -> Vec<SelfTestCheck> {
        vec![
            SelfTestCheck {
                command: "get_oscilloscope_stats",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "list_captures",
                payload: json!({}),
            },
            SelfTestCheck {
                command: "get_capture",
                payload: json!({ "session_id": "scope-demo" }),
            },
            SelfTestCheck {
                command: "acquire_capture",
                payload: json!({
                    "title": "Self-Test Verification Sweep",
                    "sample_rate_hz": 40_000.0,
                    "num_samples": 512,
                    "ch1_freq": 1000.0,
                    "ch1_amp": 3.3
                }),
            },
            SelfTestCheck {
                command: "compute_channel_math",
                payload: json!({
                    "session_id": "scope-demo",
                    "operation": "Subtract"
                }),
            },
            SelfTestCheck {
                command: "update_measurement_cursors",
                payload: json!({
                    "session_id": "scope-demo",
                    "cursors": {
                        "x1_time_ms": 0.2,
                        "x2_time_ms": 1.2,
                        "delta_t_ms": 1.0,
                        "frequency_hz": 1000.0,
                        "y1_volts": -1.5,
                        "y2_volts": 1.5,
                        "delta_v_volts": 3.0
                    }
                }),
            },
            SelfTestCheck {
                command: "import_signal_file",
                payload: json!({
                    "title": "Self-Test CSV Import",
                    "format": "csv",
                    "content": "0.0,1.0,0.5\n0.001,1.5,0.7\n0.002,2.0,0.9\n"
                }),
            },
            SelfTestCheck {
                command: "get_license_status",
                payload: json!({}),
            },
        ]
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "Offline Scientific Signal & Telemetry Oscilloscope v{}",
            env!("CARGO_PKG_VERSION")
        );
        println!("Usage: signal-telemetry-oscilloscope [OPTIONS]");
        println!("Options:");
        println!("  --self-test         Verify embedded UI assets and live IPC commands");
        println!("  --headless-server   Start loopback GUI/IPC server without browser window");
        println!("  --version, -v       Show version");
        return;
    }

    if let Err(e) = run_desktop_app(UI_BUNDLE, OscilloscopeDesktopHandler) {
        eprintln!(
            "Scientific Signal Oscilloscope desktop runtime error: {}",
            e
        );
        std::process::exit(1);
    }
}
