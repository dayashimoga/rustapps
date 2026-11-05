#![forbid(unsafe_code)]
//! Offline AI Screen Recording & Tutorial Synthesis Engine
//!
//! Handles cursor tracking, automated zoom keyframe generation, silence/dead-air
//! detection, subtitle cue synchronization, and self-contained tutorial compilation.

use crate::models::{
    AudioSilenceInterval, ClickType, CursorEvent, SubtitleCue, TutorialStep, ZoomKeyframe,
};

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
