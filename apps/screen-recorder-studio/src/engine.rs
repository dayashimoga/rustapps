#![forbid(unsafe_code)]
//! Offline AI Screen Recording & Tutorial Synthesis Engine
//!
//! Handles cursor tracking, automated zoom keyframe generation, silence/dead-air
//! detection, subtitle cue synchronization, PII redaction detection, keystroke HUD rendering,
//! and self-contained tutorial compilation.

use crate::models::{
    AudioSilenceInterval, ClickType, CursorEvent, KeystrokeEvent, RedactionRegion, SubtitleCue,
    TextRegion, TutorialStep, ZoomKeyframe,
};
use regex::Regex;
use std::sync::OnceLock;

static RE_API_KEY: OnceLock<Regex> = OnceLock::new();
static RE_JWT: OnceLock<Regex> = OnceLock::new();
static RE_EMAIL: OnceLock<Regex> = OnceLock::new();
static RE_CREDIT_CARD: OnceLock<Regex> = OnceLock::new();

fn get_api_key_regex() -> &'static Regex {
    RE_API_KEY.get_or_init(|| {
        Regex::new(r"(?i)(?:sk-[a-zA-Z0-9_\-]{16,}|AKIA[0-9A-Z]{16}|ghp_[a-zA-Z0-9]{36}|[a-zA-Z0-9_\-]{20,}_api_key)")
            .expect("Valid API key regex")
    })
}

fn get_jwt_regex() -> &'static Regex {
    RE_JWT.get_or_init(|| {
        Regex::new(r"eyJ[a-zA-Z0-9_\-]{10,}\.eyJ[a-zA-Z0-9_\-]{10,}\.[a-zA-Z0-9_\-]+")
            .expect("Valid JWT regex")
    })
}

fn get_email_regex() -> &'static Regex {
    RE_EMAIL.get_or_init(|| {
        Regex::new(r"[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+").expect("Valid email regex")
    })
}

fn get_credit_card_regex() -> &'static Regex {
    RE_CREDIT_CARD.get_or_init(|| {
        Regex::new(r"\b(?:\d{4}[-\s]?){3}\d{4}\b").expect("Valid credit card regex")
    })
}

pub struct TutorialEngine;

impl TutorialEngine {
    /// Generates dynamic camera zoom keyframes centered on user click coordinates
    pub fn generate_auto_zooms(
        events: &[CursorEvent],
        session_duration_ms: u64,
        zoom_scale: f32,
    ) -> Vec<ZoomKeyframe> {
        let mut keyframes = Vec::new();
        let scale = zoom_scale.clamp(1.1, 2.5);

        // Filter events that represent mouse clicks
        let click_events: Vec<&CursorEvent> = events
            .iter()
            .filter(|e| e.click != ClickType::None)
            .collect();

        let mut last_end_ms = 0u64;

        for (idx, click) in click_events.iter().enumerate() {
            let start_ms = click.timestamp_ms.saturating_sub(400);

            // Avoid overlapping zooms with previous keyframe
            if start_ms < last_end_ms {
                continue;
            }

            let end_ms = (click.timestamp_ms + 1600).min(session_duration_ms);
            if end_ms <= start_ms {
                continue;
            }

            let action_name = match click.click {
                ClickType::Left => "Primary Action",
                ClickType::Right => "Context Menu",
                ClickType::Double => "Double Click Selection",
                ClickType::None => "Focus",
            };

            keyframes.push(ZoomKeyframe {
                start_ms,
                end_ms,
                focus_x: click.x.clamp(0.1, 0.9),
                focus_y: click.y.clamp(0.1, 0.9),
                scale,
                easing: "ease-in-out".to_string(),
                label: format!("Step #{}: {}", idx + 1, action_name),
            });

            last_end_ms = end_ms + 200; // 200ms padding before next zoom
        }

        keyframes
    }

    /// Detects silent intervals in audio signal below dB threshold eligible for dead-air trimming
    pub fn detect_silence_intervals(
        rms_levels_per_100ms: &[f32],
        silence_threshold_db: f32,
        min_silence_duration_ms: u64,
    ) -> Vec<AudioSilenceInterval> {
        let mut intervals = Vec::new();
        let mut in_silence = false;
        let mut silence_start_ms = 0u64;
        let mut sum_rms = 0.0f32;
        let mut sample_count = 0usize;

        for (i, &db) in rms_levels_per_100ms.iter().enumerate() {
            let current_ms = (i as u64) * 100;
            let is_silent = db <= silence_threshold_db;

            if is_silent {
                if !in_silence {
                    in_silence = true;
                    silence_start_ms = current_ms;
                    sum_rms = db;
                    sample_count = 1;
                } else {
                    sum_rms += db;
                    sample_count += 1;
                }
            } else if in_silence {
                let duration = current_ms.saturating_sub(silence_start_ms);
                if duration >= min_silence_duration_ms {
                    let avg_db = if sample_count > 0 {
                        sum_rms / (sample_count as f32)
                    } else {
                        silence_threshold_db
                    };
                    intervals.push(AudioSilenceInterval {
                        start_ms: silence_start_ms,
                        end_ms: current_ms,
                        duration_ms: duration,
                        rms_db: avg_db,
                    });
                }
                in_silence = false;
                sum_rms = 0.0;
                sample_count = 0;
            }
        }

        // Flush terminal silence if active
        if in_silence {
            let final_ms = (rms_levels_per_100ms.len() as u64) * 100;
            let duration = final_ms.saturating_sub(silence_start_ms);
            if duration >= min_silence_duration_ms {
                let avg_db = if sample_count > 0 {
                    sum_rms / (sample_count as f32)
                } else {
                    silence_threshold_db
                };
                intervals.push(AudioSilenceInterval {
                    start_ms: silence_start_ms,
                    end_ms: final_ms,
                    duration_ms: duration,
                    rms_db: avg_db,
                });
            }
        }

        intervals
    }

    /// Converts recorded mouse clicks into structured interactive tutorial steps
    pub fn build_tutorial_steps(events: &[CursorEvent]) -> Vec<TutorialStep> {
        let mut steps = Vec::new();
        let mut step_num = 1u32;

        for event in events {
            if event.click == ClickType::None {
                continue;
            }

            let (action_type, title, instruction) = match event.click {
                ClickType::Left => (
                    "Click".to_string(),
                    format!("Step {}: Select Target Element", step_num),
                    format!(
                        "Click the target control located at position ({:.0}%, {:.0}%) on the screen.",
                        event.x * 100.0,
                        event.y * 100.0
                    ),
                ),
                ClickType::Right => (
                    "RightClick".to_string(),
                    format!("Step {}: Open Context Menu", step_num),
                    format!(
                        "Right click on coordinate ({:.0}%, {:.0}%) to reveal operational actions.",
                        event.x * 100.0,
                        event.y * 100.0
                    ),
                ),
                ClickType::Double => (
                    "DoubleClick".to_string(),
                    format!("Step {}: Activate / Open Item", step_num),
                    format!(
                        "Double click the item at ({:.0}%, {:.0}%) to execute or open.",
                        event.x * 100.0,
                        event.y * 100.0
                    ),
                ),
                ClickType::None => continue,
            };

            steps.push(TutorialStep {
                step_number: step_num,
                timestamp_ms: event.timestamp_ms,
                title,
                instruction,
                action_type,
                target_x: event.x,
                target_y: event.y,
            });

            step_num += 1;
        }

        steps
    }

    /// Detects sensitive PII patterns (API keys, JWT tokens, emails, credit cards)
    /// across text bounding boxes and returns suggested redaction regions.
    pub fn detect_pii_redactions(regions: &[TextRegion]) -> Vec<RedactionRegion> {
        let mut redactions = Vec::new();
        let re_api = get_api_key_regex();
        let re_jwt = get_jwt_regex();
        let re_email = get_email_regex();
        let re_card = get_credit_card_regex();

        for region in regions {
            let mut detected_type = None;

            if re_api.is_match(&region.text) {
                detected_type = Some("api_key");
            } else if re_jwt.is_match(&region.text) {
                detected_type = Some("jwt");
            } else if re_card.is_match(&region.text) {
                detected_type = Some("credit_card");
            } else if re_email.is_match(&region.text) {
                detected_type = Some("email");
            }

            if let Some(ptype) = detected_type {
                redactions.push(RedactionRegion {
                    id: format!("redact-{}", uuid::Uuid::new_v4().simple()),
                    label: format!("Auto Redacted ({})", ptype),
                    x: region.x.clamp(0.0, 1.0),
                    y: region.y.clamp(0.0, 1.0),
                    width: region.width.clamp(0.01, 1.0),
                    height: region.height.clamp(0.01, 1.0),
                    pattern_type: ptype.to_string(),
                });
            }
        }

        redactions
    }

    /// Renders an SVG overlay element representing keystroke shortcut pill HUD
    pub fn render_keystroke_hud_svg(
        active_keys: &[&KeystrokeEvent],
        width: u32,
        height: u32,
    ) -> String {
        if active_keys.is_empty() {
            return format!(
                r#"<svg width="{}" height="{}" viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg"></svg>"#,
                width, height, width, height
            );
        }

        let mut pills = String::new();
        let pill_height = 36;
        let base_y = height.saturating_sub(60);

        for (i, key) in active_keys.iter().enumerate() {
            let combo = &key.key_combination;
            let parts: Vec<&str> = combo.split('+').collect();
            let mut key_tags = String::new();
            let mut offset_x = 20.0f32;

            for part in parts {
                let text = part.trim();
                let key_w = (text.len() as f32 * 10.0 + 20.0).max(36.0);
                key_tags.push_str(&format!(
                    r##"<rect x="{}" y="6" width="{}" height="24" rx="4" fill="#1e293b" stroke="#475569" stroke-width="1.5"/>
                    <text x="{}" y="22" font-family="monospace, sans-serif" font-size="12" font-weight="700" fill="#f8fafc" text-anchor="middle">{}</text>"##,
                    offset_x,
                    key_w,
                    offset_x + key_w / 2.0,
                    text
                ));
                offset_x += key_w + 8.0;
            }

            let total_pill_w = offset_x + 12.0;
            let start_x = ((width as f32 - total_pill_w) / 2.0).max(10.0);
            let pill_y = base_y.saturating_sub((i as u32) * (pill_height + 8));

            pills.push_str(&format!(
                r##"<g transform="translate({}, {})">
                    <rect x="0" y="0" width="{}" height="{}" rx="8" fill="rgba(15, 23, 42, 0.9)" stroke="#38bdf8" stroke-width="1.5" filter="drop-shadow(0 4px 6px rgba(0,0,0,0.6))"/>
                    {}
                </g>"##,
                start_x, pill_y, total_pill_w, pill_height, key_tags
            ));
        }

        format!(
            r#"<svg width="{}" height="{}" viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg">{}</svg>"#,
            width, height, width, height, pills
        )
    }

    /// Formats subtitle cues into standard SubRip (.srt) timing text
    pub fn format_srt_subtitles(cues: &[SubtitleCue]) -> String {
        let mut out = String::new();

        for (i, cue) in cues.iter().enumerate() {
            let start = Self::format_srt_timestamp(cue.start_ms);
            let end = Self::format_srt_timestamp(cue.end_ms);
            let speaker_prefix = cue
                .speaker
                .as_ref()
                .map(|s| format!("[{}] ", s))
                .unwrap_or_default();

            out.push_str(&format!(
                "{}\n{} --> {}\n{}{}\n\n",
                i + 1,
                start,
                end,
                speaker_prefix,
                cue.text
            ));
        }

        out
    }

    fn format_srt_timestamp(ms: u64) -> String {
        let hours = ms / 3_600_000;
        let rem = ms % 3_600_000;
        let mins = rem / 60_000;
        let rem = rem % 60_000;
        let secs = rem / 1_000;
        let millis = rem % 1_000;

        format!("{:02}:{:02}:{:02},{:03}", hours, mins, secs, millis)
    }

    /// Compiles a 100% offline, self-contained HTML5 interactive tutorial walkthrough
    pub fn render_interactive_html(
        session_title: &str,
        steps: &[TutorialStep],
        subtitles: &[SubtitleCue],
    ) -> String {
        let mut steps_html = String::new();
        for step in steps {
            steps_html.push_str(&format!(
                r#"<div class="step-card" data-time="{}">
                    <div class="step-badge">STEP {}</div>
                    <div class="step-title">{}</div>
                    <div class="step-desc">{}</div>
                    <div class="step-coords">Coordinates: {:.1}%, {:.1}% | Action: {}</div>
                </div>"#,
                step.timestamp_ms,
                step.step_number,
                step.title,
                step.instruction,
                step.target_x * 100.0,
                step.target_y * 100.0,
                step.action_type
            ));
        }

        let mut subs_html = String::new();
        for sub in subtitles {
            subs_html.push_str(&format!(
                r#"<div class="sub-item" data-start="{}" data-end="{}">
                    <span class="sub-time">{}</span>
                    <span class="sub-text">{}</span>
                </div>"#,
                sub.start_ms,
                sub.end_ms,
                Self::format_srt_timestamp(sub.start_ms),
                sub.text
            ));
        }

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Interactive Tutorial - {}</title>
<style>
  body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #0b0f19; color: #f1f5f9; margin: 0; padding: 24px; }}
  h1 {{ color: #38bdf8; margin-bottom: 8px; }}
  .layout {{ display: grid; grid-template-columns: 1fr 340px; gap: 24px; max-width: 1200px; margin: 0 auto; }}
  .tutorial-steps {{ display: flex; flex-direction: column; gap: 14px; }}
  .step-card {{ background: #131c2e; border: 1px solid #1e293b; border-radius: 8px; padding: 16px; }}
  .step-badge {{ display: inline-block; background: #0284c7; color: #fff; font-size: 11px; font-weight: 700; padding: 2px 8px; border-radius: 4px; margin-bottom: 6px; }}
  .step-title {{ font-size: 16px; font-weight: 600; margin-bottom: 4px; }}
  .step-desc {{ font-size: 14px; color: #94a3b8; margin-bottom: 8px; }}
  .step-coords {{ font-size: 12px; font-family: monospace; color: #64748b; }}
  .transcript-panel {{ background: #131c2e; border: 1px solid #1e293b; border-radius: 8px; padding: 16px; max-height: 80vh; overflow-y: auto; }}
  .sub-item {{ padding: 8px 0; border-bottom: 1px dashed #1e293b; font-size: 13px; }}
  .sub-time {{ color: #38bdf8; font-family: monospace; display: block; font-size: 11px; }}
</style>
</head>
<body>
  <h1>{}</h1>
  <div class="layout">
    <div class="tutorial-steps">{}</div>
    <div class="transcript-panel">
      <h3>Synchronized Subtitles</h3>
      {}
    </div>
  </div>
</body>
</html>"#,
            session_title, session_title, steps_html, subs_html
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_zoom_generation() {
        let events = vec![
            CursorEvent {
                timestamp_ms: 1000,
                x: 0.25,
                y: 0.35,
                click: ClickType::Left,
            },
            CursorEvent {
                timestamp_ms: 1200,
                x: 0.26,
                y: 0.36,
                click: ClickType::None,
            },
            CursorEvent {
                timestamp_ms: 5000,
                x: 0.75,
                y: 0.85,
                click: ClickType::Right,
            },
        ];

        let zooms = TutorialEngine::generate_auto_zooms(&events, 10_000, 1.5);
        assert_eq!(zooms.len(), 2);
        assert_eq!(zooms[0].start_ms, 600); // 1000 - 400
        assert_eq!(zooms[0].end_ms, 2600); // 1000 + 1600
        assert_eq!(zooms[0].scale, 1.5);
        assert_eq!(zooms[1].start_ms, 4600);
    }

    #[test]
    fn test_silence_detection() {
        // 20 intervals of 100ms = 2.0s
        // indices 5 to 15 have silence (-55.0 dB) = 11 intervals = 1100ms
        let mut rms = vec![-20.0f32; 25];
        for val in rms.iter_mut().take(18).skip(5) {
            *val = -52.0;
        }

        let intervals = TutorialEngine::detect_silence_intervals(&rms, -45.0, 800);
        assert_eq!(intervals.len(), 1);
        assert_eq!(intervals[0].start_ms, 500);
        assert_eq!(intervals[0].end_ms, 1800);
        assert_eq!(intervals[0].duration_ms, 1300);
    }

    #[test]
    fn test_tutorial_steps_builder() {
        let events = vec![
            CursorEvent {
                timestamp_ms: 1500,
                x: 0.4,
                y: 0.5,
                click: ClickType::Left,
            },
            CursorEvent {
                timestamp_ms: 3500,
                x: 0.8,
                y: 0.2,
                click: ClickType::Double,
            },
        ];

        let steps = TutorialEngine::build_tutorial_steps(&events);
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].step_number, 1);
        assert_eq!(steps[0].action_type, "Click");
        assert_eq!(steps[1].step_number, 2);
        assert_eq!(steps[1].action_type, "DoubleClick");
    }

    #[test]
    fn test_pii_redaction_detection() {
        let regions = vec![
            TextRegion {
                text: "Here is your API key: sk-ant-api03-abcdef1234567890xyz and continue"
                    .to_string(),
                x: 0.1,
                y: 0.2,
                width: 0.3,
                height: 0.05,
            },
            TextRegion {
                text: "Contact user at dev@enterprise.internal for support".to_string(),
                x: 0.5,
                y: 0.6,
                width: 0.25,
                height: 0.04,
            },
            TextRegion {
                text: "Card number 4111 2222 3333 4444 on file".to_string(),
                x: 0.2,
                y: 0.8,
                width: 0.4,
                height: 0.05,
            },
            TextRegion {
                text: "Normal non-sensitive instructional text".to_string(),
                x: 0.1,
                y: 0.1,
                width: 0.5,
                height: 0.05,
            },
        ];

        let redactions = TutorialEngine::detect_pii_redactions(&regions);
        assert_eq!(redactions.len(), 3);
        assert_eq!(redactions[0].pattern_type, "api_key");
        assert_eq!(redactions[1].pattern_type, "email");
        assert_eq!(redactions[2].pattern_type, "credit_card");
    }

    #[test]
    fn test_keystroke_hud_svg_rendering() {
        let ev1 = KeystrokeEvent {
            timestamp_ms: 1000,
            key_combination: "Ctrl+Shift+P".to_string(),
            duration_ms: 1200,
        };
        let ev2 = KeystrokeEvent {
            timestamp_ms: 2500,
            key_combination: "Enter".to_string(),
            duration_ms: 500,
        };

        let svg_empty = TutorialEngine::render_keystroke_hud_svg(&[], 1280, 720);
        assert!(svg_empty.contains("<svg width=\"1280\""));

        let svg_active = TutorialEngine::render_keystroke_hud_svg(&[&ev1, &ev2], 1280, 720);
        assert!(svg_active.contains("Ctrl"));
        assert!(svg_active.contains("Shift"));
        assert!(svg_active.contains("P"));
        assert!(svg_active.contains("Enter"));
    }

    #[test]
    fn test_srt_subtitle_formatter() {
        let cues = vec![
            SubtitleCue {
                start_ms: 1000,
                end_ms: 3250,
                text: "Welcome to the tutorial.".to_string(),
                speaker: Some("Instructor".to_string()),
            },
            SubtitleCue {
                start_ms: 4000,
                end_ms: 6000,
                text: "Click the primary action button.".to_string(),
                speaker: None,
            },
        ];

        let srt = TutorialEngine::format_srt_subtitles(&cues);
        assert!(srt.contains("00:00:01,000 --> 00:00:03,250"));
        assert!(srt.contains("[Instructor] Welcome to the tutorial."));
        assert!(srt.contains("00:00:04,000 --> 00:00:06,000"));
    }

    #[test]
    fn test_interactive_html_render() {
        let steps = vec![TutorialStep {
            step_number: 1,
            timestamp_ms: 1000,
            title: "Open Dashboard".to_string(),
            instruction: "Click top-left icon".to_string(),
            action_type: "Click".to_string(),
            target_x: 0.1,
            target_y: 0.1,
        }];
        let subtitles = vec![SubtitleCue {
            start_ms: 1000,
            end_ms: 2000,
            text: "Opening dashboard".to_string(),
            speaker: None,
        }];

        let html = TutorialEngine::render_interactive_html("Demo Walkthrough", &steps, &subtitles);
        assert!(html.contains("Demo Walkthrough"));
        assert!(html.contains("Open Dashboard"));
        assert!(html.contains("Opening dashboard"));
    }
}
