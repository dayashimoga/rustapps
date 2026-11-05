#![deny(unsafe_code)]
//! Project Templates, Export Presets & Timeline Interchange (EDL & FCPXML)
//!
//! Provides standardized studio configurations and interchange with external NLEs:
//! - Pre-configured templates: 4K UHD, 9:16 Vertical Shorts, Cinema Scope, Multitrack Podcast
//! - Export presets: YouTube 4K, 1080p60 Web, Reel/Shorts, Podcast Broadcast WAV
//! - CMX 3600 Edit Decision List (EDL) text exporter
//! - Final Cut Pro XML (FCPXML) structural exporter

use crate::models::{FrameRate, ProjectSettings, TimelineProject, TrackType};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// 1. Export Presets
// ---------------------------------------------------------------------------

/// Render output format presets
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub width: u32,
    pub height: u32,
    pub frame_rate: FrameRate,
    pub video_bitrate_kbps: u32,
    pub audio_sample_rate: u32,
    pub audio_channels: u32,
    pub audio_bitrate_kbps: u32,
    pub container: String, // "mp4", "mov", "wav", "webm"
}

impl ExportPreset {
    pub fn all_standard_presets() -> Vec<Self> {
        vec![
            Self {
                id: "preset_4k60_master".to_string(),
                name: "4K UHD Master (60 FPS)".to_string(),
                description: "ProRes / High-bitrate 3840x2160 60 FPS master delivery".to_string(),
                width: 3840,
                height: 2160,
                frame_rate: FrameRate::Fps60,
                video_bitrate_kbps: 80000,
                audio_sample_rate: 48000,
                audio_channels: 2,
                audio_bitrate_kbps: 384,
                container: "mp4".to_string(),
            },
            Self {
                id: "preset_1080p60_web".to_string(),
                name: "Full HD Web Delivery (1080p60)".to_string(),
                description: "Optimized for YouTube, Vimeo, and web streaming".to_string(),
                width: 1920,
                height: 1080,
                frame_rate: FrameRate::Fps60,
                video_bitrate_kbps: 16000,
                audio_sample_rate: 48000,
                audio_channels: 2,
                audio_bitrate_kbps: 320,
                container: "mp4".to_string(),
            },
            Self {
                id: "preset_vertical_shorts".to_string(),
                name: "9:16 Vertical Short / Reel (1080x1920)".to_string(),
                description: "Optimized for TikTok, YouTube Shorts, and Instagram Reels"
                    .to_string(),
                width: 1080,
                height: 1920,
                frame_rate: FrameRate::Fps60,
                video_bitrate_kbps: 18000,
                audio_sample_rate: 48000,
                audio_channels: 2,
                audio_bitrate_kbps: 320,
                container: "mp4".to_string(),
            },
            Self {
                id: "preset_cinematic_scope".to_string(),
                name: "2.39:1 Cinema Scope (3840x1608 24p)".to_string(),
                description:
                    "Anamorphic widescreen film deliverable with 24 FPS theatrical cadence"
                        .to_string(),
                width: 3840,
                height: 1608,
                frame_rate: FrameRate::Fps24,
                video_bitrate_kbps: 60000,
                audio_sample_rate: 48000,
                audio_channels: 2,
                audio_bitrate_kbps: 384,
                container: "mp4".to_string(),
            },
            Self {
                id: "preset_uncompressed_avi".to_string(),
                name: "Lossless Master Broadcast Video (AVI)".to_string(),
                description: "Pristine uncompressed broadcast master with linear PCM 48kHz stereo"
                    .to_string(),
                width: 1920,
                height: 1080,
                frame_rate: FrameRate::Fps60,
                video_bitrate_kbps: 200000,
                audio_sample_rate: 48000,
                audio_channels: 2,
                audio_bitrate_kbps: 1536,
                container: "avi".to_string(),
            },
            Self {
                id: "preset_broadcast_wav".to_string(),
                name: "Broadcast Master WAV (48kHz 24-bit)".to_string(),
                description: "EBU R128 compliant uncompressed linear PCM audio master".to_string(),
                width: 0,
                height: 0,
                frame_rate: FrameRate::Fps24,
                video_bitrate_kbps: 0,
                audio_sample_rate: 48000,
                audio_channels: 2,
                audio_bitrate_kbps: 2304,
                container: "wav".to_string(),
            },
        ]
    }
}

// ---------------------------------------------------------------------------
// 2. Project Templates
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectTemplate {
    pub id: String,
    pub name: String,
    pub category: String,
    pub settings: ProjectSettings,
    pub initial_tracks: Vec<(String, String, TrackType, i32)>,
}

impl ProjectTemplate {
    pub fn all_templates() -> Vec<Self> {
        vec![
            Self {
                id: "tpl_standard_video".to_string(),
                name: "Standard Video Editing (1080p60)".to_string(),
                category: "Video".to_string(),
                settings: ProjectSettings {
                    width: 1920,
                    height: 1080,
                    frame_rate: FrameRate::Fps60,
                    sample_rate: 48000,
                    audio_channels: 2,
                },
                initial_tracks: vec![
                    ("v1".into(), "Video Main".into(), TrackType::Video, 10),
                    (
                        "v2".into(),
                        "B-Roll / Overlays".into(),
                        TrackType::Video,
                        11,
                    ),
                    (
                        "a1".into(),
                        "Dialogue / Voiceover".into(),
                        TrackType::Audio,
                        100,
                    ),
                    ("a2".into(), "Sound Effects".into(), TrackType::Audio, 101),
                    ("a3".into(), "Music Bed".into(), TrackType::Audio, 102),
                    ("s1".into(), "Subtitles".into(), TrackType::Subtitle, 200),
                ],
            },
            Self {
                id: "tpl_vertical_shorts".to_string(),
                name: "Vertical Social Media Short (9:16)".to_string(),
                category: "Social".to_string(),
                settings: ProjectSettings {
                    width: 1080,
                    height: 1920,
                    frame_rate: FrameRate::Fps60,
                    sample_rate: 48000,
                    audio_channels: 2,
                },
                initial_tracks: vec![
                    ("v1".into(), "Main Video".into(), TrackType::Video, 10),
                    (
                        "v2".into(),
                        "Stickers & Emojis".into(),
                        TrackType::Video,
                        11,
                    ),
                    ("a1".into(), "Speech Voice".into(), TrackType::Audio, 100),
                    ("a2".into(), "Trending Audio".into(), TrackType::Audio, 101),
                    ("s1".into(), "Captions".into(), TrackType::Subtitle, 200),
                ],
            },
            Self {
                id: "tpl_cinematic_239".to_string(),
                name: "Cinematic Anamorphic (2.39:1 Scope 24p)".to_string(),
                category: "Film".to_string(),
                settings: ProjectSettings {
                    width: 3840,
                    height: 1608,
                    frame_rate: FrameRate::Fps24,
                    sample_rate: 48000,
                    audio_channels: 2,
                },
                initial_tracks: vec![
                    ("v1".into(), "Cinematic A-Cam".into(), TrackType::Video, 10),
                    ("v2".into(), "B-Cam Cutaways".into(), TrackType::Video, 11),
                    (
                        "adj1".into(),
                        "Master Grade LUT".into(),
                        TrackType::Adjustment,
                        50,
                    ),
                    ("a1".into(), "Boom Mic".into(), TrackType::Audio, 100),
                    (
                        "a2".into(),
                        "Foley / Ambience".into(),
                        TrackType::Audio,
                        101,
                    ),
                    ("a3".into(), "Score".into(), TrackType::Audio, 102),
                ],
            },
            Self {
                id: "tpl_podcast_master".to_string(),
                name: "Multi-Host Podcast Studio".to_string(),
                category: "Audio".to_string(),
                settings: ProjectSettings {
                    width: 1920,
                    height: 1080,
                    frame_rate: FrameRate::Fps30,
                    sample_rate: 48000,
                    audio_channels: 2,
                },
                initial_tracks: vec![
                    ("v1".into(), "Host Camera Grid".into(), TrackType::Video, 10),
                    ("a1".into(), "Host 1 Mic".into(), TrackType::Audio, 100),
                    ("a2".into(), "Host 2 Mic".into(), TrackType::Audio, 101),
                    ("a3".into(), "Guest Remote".into(), TrackType::Audio, 102),
                    (
                        "a4".into(),
                        "Intro / Outro Jingles".into(),
                        TrackType::Audio,
                        103,
                    ),
                ],
            },
        ]
    }
}

// ---------------------------------------------------------------------------
// 3. CMX 3600 Edit Decision List (EDL) Exporter
// ---------------------------------------------------------------------------

pub struct EdlExporter;

impl EdlExporter {
    /// Export timeline project to standard CMX 3600 EDL format
    pub fn export_cmx3600(project: &TimelineProject) -> String {
        let mut out = String::new();
        out.push_str(&format!("TITLE: {}\n", project.name));
        out.push_str("FCM: NON-DROP FRAME\n\n");

        let fps = project.settings.frame_rate;
        let mut event_num = 1;

        for track in &project.tracks {
            let track_type_str = match track.track_type {
                TrackType::Video => "V",
                TrackType::Audio => "A",
                TrackType::Subtitle => "SUB",
                TrackType::Adjustment => "ADJ",
            };

            for clip in &track.clips {
                let src_in_tc =
                    crate::models::Timecode::from_millis(clip.source_in_ms, fps).to_smpte();
                let src_out_tc =
                    crate::models::Timecode::from_millis(clip.source_out_ms, fps).to_smpte();
                let rec_in_tc =
                    crate::models::Timecode::from_millis(clip.timeline_in_ms, fps).to_smpte();
                let rec_out_tc =
                    crate::models::Timecode::from_millis(clip.timeline_out_ms, fps).to_smpte();

                let reel_name = clip
                    .source_id
                    .as_deref()
                    .unwrap_or("AX")
                    .chars()
                    .take(8)
                    .collect::<String>();

                // Format: EVENT_NUM REEL_NAME TRACK_TYPE EDIT_TYPE SRC_IN SRC_OUT REC_IN REC_OUT
                out.push_str(&format!(
                    "{:03}  {:<8} {:<4} C        {} {} {} {}\n",
                    event_num,
                    reel_name,
                    track_type_str,
                    src_in_tc,
                    src_out_tc,
                    rec_in_tc,
                    rec_out_tc
                ));
                out.push_str(&format!("* FROM CLIP NAME: {}\n\n", clip.name));

                event_num += 1;
            }
        }

        out
    }
}

// ---------------------------------------------------------------------------
// 4. Final Cut Pro XML (FCPXML) Exporter
// ---------------------------------------------------------------------------

pub struct FcpxmlExporter;

impl FcpxmlExporter {
    /// Export timeline project to standard Final Cut Pro XML (FCPXML v1.9)
    pub fn export_fcpxml(project: &TimelineProject) -> String {
        let mut out = String::new();
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        out.push_str("<!DOCTYPE fcpxml>\n");
        out.push_str("<fcpxml version=\"1.9\">\n");
        out.push_str("  <resources>\n");

        let fps_f64 = project.settings.frame_rate.as_f64();
        let frame_duration = format!("{}/{}s", 1000, (fps_f64 * 1000.0).round() as u64);

        out.push_str(&format!(
            "    <format id=\"r1\" name=\"FFVideoFormat{}p{}\" frameDuration=\"{}\" width=\"{}\" height=\"{}\"/>\n",
            project.settings.height,
            project.settings.frame_rate.as_f64().round() as u32,
            frame_duration,
            project.settings.width,
            project.settings.height
        ));
        out.push_str("  </resources>\n");

        out.push_str("  <library>\n");
        out.push_str(&format!("    <event name=\"{}\">\n", project.name));
        out.push_str(&format!("      <project name=\"{}\">\n", project.name));
        out.push_str(&format!(
            "        <sequence format=\"r1\" duration=\"{}/1000s\">\n",
            project.duration_ms()
        ));
        out.push_str("          <spine>\n");

        for track in &project.tracks {
            for clip in &track.clips {
                let duration_ms = clip.duration_ms();
                out.push_str(&format!(
                    "            <clip name=\"{}\" offset=\"{}/1000s\" duration=\"{}/1000s\" start=\"{}/1000s\">\n",
                    clip.name.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;"),
                    clip.timeline_in_ms,
                    duration_ms,
                    clip.source_in_ms
                ));
                out.push_str("            </clip>\n");
            }
        }

        out.push_str("          </spine>\n");
        out.push_str("        </sequence>\n");
        out.push_str("      </project>\n");
        out.push_str("    </event>\n");
        out.push_str("  </library>\n");
        out.push_str("</fcpxml>\n");

        out
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        AnimatedProperty, BlendMode, ChromaKeySettings, Clip, ClipType, ColorGradingSettings,
        CompressorSettings, ParametricEqSettings, Track, Transform2D,
    };

    #[test]
    fn test_export_presets_and_project_templates() {
        let presets = ExportPreset::all_standard_presets();
        assert_eq!(presets.len(), 6);
        assert!(presets.iter().any(|p| p.id == "preset_4k60_master"));
        assert!(presets.iter().any(|p| p.container == "wav"));

        let templates = ProjectTemplate::all_templates();
        assert_eq!(templates.len(), 4);
        let social = templates
            .iter()
            .find(|t| t.id == "tpl_vertical_shorts")
            .unwrap();
        assert_eq!(social.settings.width, 1080);
        assert_eq!(social.settings.height, 1920);
    }

    #[test]
    fn test_edl_and_fcpxml_export_format() {
        let mut project =
            TimelineProject::new("EDL Project", "Studio Reel", ProjectSettings::default());
        let mut v1 = Track::new("v1", "Video Track", TrackType::Video, 0);

        let clip = Clip {
            id: "c1".to_string(),
            name: "Interview Shot".to_string(),
            clip_type: ClipType::Video,
            track_id: "v1".to_string(),
            source_id: Some("REEL_A".to_string()),
            source_in_ms: 0,
            source_out_ms: 3000,
            timeline_in_ms: 0,
            timeline_out_ms: 3000,
            speed_multiplier: 1.0,
            volume: AnimatedProperty::constant(1.0),
            pan: 0.0,
            opacity: AnimatedProperty::constant(1.0),
            blend_mode: BlendMode::Normal,
            transform: Transform2D::default(),
            keyframed_transform: None,
            chroma_key: ChromaKeySettings::default(),
            color_grading: ColorGradingSettings::default(),
            eq_settings: ParametricEqSettings::default(),
            compressor: CompressorSettings::default(),
        };

        v1.clips.push(clip);
        project.tracks.push(v1);

        // Test CMX 3600 EDL
        let edl = EdlExporter::export_cmx3600(&project);
        assert!(edl.contains("TITLE: Studio Reel"));
        assert!(edl.contains("REEL_A"));
        assert!(edl.contains("FROM CLIP NAME: Interview Shot"));

        // Test FCPXML
        let fcpxml = FcpxmlExporter::export_fcpxml(&project);
        assert!(fcpxml.contains("<!DOCTYPE fcpxml>"));
        assert!(fcpxml.contains("<fcpxml version=\"1.9\">"));
        assert!(fcpxml.contains("name=\"Interview Shot\""));
    }
}
