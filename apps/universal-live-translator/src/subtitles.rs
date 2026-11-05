#![forbid(unsafe_code)]
//! Document & Subtitle Structural Translator & Audio-to-SRT Generator
//!
//! Provides parsing, structural translation, and serialization for:
//! - SubRip (`.srt`) and WebVTT (`.vtt`) time-stamped subtitles
//! - Markdown (`.md`) and plain text (`.txt`) documents preserving code blocks,
//!   headers, lists, blockquotes, and link structures
//! - Audio-to-Subtitle transcription generating timestamped SRT/VTT from raw speech
//!
//! 100% offline, zero network egress.

use crate::models::{
    BatchDocumentItem, BatchTranslationResult, LanguageCode, SubtitleCue, SubtitleFormat,
    TranslationRequest,
};
use crate::nmt::{OfflineTranslationEngine, TranslationError};
use crate::speech::{BatchSpeechRecognizer, SpeechError};
use std::sync::Arc;
use thiserror::Error;

/// Subtitle and document translation error definitions
#[derive(Debug, Error)]
pub enum SubtitleError {
    #[error("Failed to parse subtitle content: {0}")]
    Parse(String),
    #[error("Translation failed: {0}")]
    Translation(#[from] TranslationError),
    #[error("Speech recognition failed: {0}")]
    Speech(#[from] SpeechError),
    #[error("Unsupported subtitle format: {0}")]
    InvalidFormat(String),
}

/// Subtitle and document translation engine
pub struct SubtitleTranslator {
    nmt: Arc<OfflineTranslationEngine>,
    speech: Arc<BatchSpeechRecognizer>,
}

impl SubtitleTranslator {
    /// Initialize subtitle translator with NMT and ASR engines
    pub fn new(nmt: Arc<OfflineTranslationEngine>, speech: Arc<BatchSpeechRecognizer>) -> Self {
        Self { nmt, speech }
    }

    /// Parse raw subtitle string into structured `SubtitleCue` list
    pub fn parse_subtitles(
        &self,
        content: &str,
        format: SubtitleFormat,
    ) -> Result<Vec<SubtitleCue>, SubtitleError> {
        match format {
            SubtitleFormat::Srt => parse_srt_subtitles(content),
            SubtitleFormat::Vtt => parse_vtt_subtitles(content),
        }
    }

    /// Serialize structured `SubtitleCue` list into target format string
    pub fn serialize_subtitles(&self, cues: &[SubtitleCue], format: SubtitleFormat) -> String {
        match format {
            SubtitleFormat::Srt => serialize_srt_subtitles(cues),
            SubtitleFormat::Vtt => serialize_vtt_subtitles(cues),
        }
    }

    /// Translate list of subtitle cues into target language preserving millisecond timing
    pub fn translate_cues(
        &self,
        cues: &[SubtitleCue],
        source_lang: Option<LanguageCode>,
        target_lang: LanguageCode,
        domain_glossary: Option<&str>,
    ) -> Result<Vec<SubtitleCue>, SubtitleError> {
        let mut translated = Vec::with_capacity(cues.len());

        for cue in cues {
            let req = TranslationRequest {
                source_lang,
                target_lang,
                text: cue.original_text.clone(),
                preserve_formatting: true,
                domain_glossary: domain_glossary.map(|s| s.to_string()),
            };
            let res = self.nmt.translate(&req)?;

            translated.push(SubtitleCue {
                index: cue.index,
                start_ms: cue.start_ms,
                end_ms: cue.end_ms,
                original_text: cue.original_text.clone(),
                translated_text: res.translated_text,
            });
        }

        Ok(translated)
    }

    /// Complete subtitle file translation workflow: parse, translate cues, serialize
    pub fn translate_subtitle_file(
        &self,
        content: &str,
        format: SubtitleFormat,
        source_lang: Option<LanguageCode>,
        target_lang: LanguageCode,
        domain_glossary: Option<&str>,
    ) -> Result<String, SubtitleError> {
        let cues = self.parse_subtitles(content, format)?;
        let translated_cues = self.translate_cues(&cues, source_lang, target_lang, domain_glossary)?;
        Ok(self.serialize_subtitles(&translated_cues, format))
    }

    /// Translate document text (.md, .txt) preserving structural elements (headings, code blocks, lists)
    pub fn translate_document(
        &self,
        doc_text: &str,
        format: &str,
        source_lang: Option<LanguageCode>,
        target_lang: LanguageCode,
        domain_glossary: Option<&str>,
    ) -> Result<String, SubtitleError> {
        let is_markdown =
            format.eq_ignore_ascii_case("md") || format.eq_ignore_ascii_case("markdown");

        let glossary_owned = domain_glossary.map(|s| s.to_string());

        if !is_markdown {
            // Standard plain text translation
            let req = TranslationRequest {
                source_lang,
                target_lang,
                text: doc_text.to_string(),
                preserve_formatting: true,
                domain_glossary: glossary_owned,
            };
            let res = self.nmt.translate(&req)?;
            return Ok(res.translated_text);
        }

        // Markdown structural translation
        let lines: Vec<&str> = doc_text.lines().collect();
        let mut output_lines = Vec::with_capacity(lines.len());
        let mut in_code_block = false;

        for line in lines {
            let trimmed = line.trim();

            // Code fence toggle
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                in_code_block = !in_code_block;
                output_lines.push(line.to_string());
                continue;
            }

            // Do not translate contents inside code blocks
            if in_code_block || trimmed.is_empty() {
                output_lines.push(line.to_string());
                continue;
            }

            // Preserve Markdown Headings (# Header)
            if trimmed.starts_with('#') {
                let hash_count = trimmed.chars().take_while(|&c| c == '#').count();
                let heading_prefix = &trimmed[..hash_count];
                let heading_text = trimmed[hash_count..].trim();

                let req = TranslationRequest {
                    source_lang,
                    target_lang,
                    text: heading_text.to_string(),
                    preserve_formatting: true,
                    domain_glossary: glossary_owned.clone(),
                };
                let res = self.nmt.translate(&req)?;
                output_lines.push(format!("{} {}", heading_prefix, res.translated_text));
                continue;
            }

            // Preserve List Items (- item, * item, 1. item)
            if let Some(item_text) = trimmed.strip_prefix("- ") {
                let req = TranslationRequest {
                    source_lang,
                    target_lang,
                    text: item_text.to_string(),
                    preserve_formatting: true,
                    domain_glossary: glossary_owned.clone(),
                };
                let res = self.nmt.translate(&req)?;
                output_lines.push(format!("- {}", res.translated_text));
                continue;
            }
            if let Some(item_text) = trimmed.strip_prefix("* ") {
                let req = TranslationRequest {
                    source_lang,
                    target_lang,
                    text: item_text.to_string(),
                    preserve_formatting: true,
                    domain_glossary: glossary_owned.clone(),
                };
                let res = self.nmt.translate(&req)?;
                output_lines.push(format!("* {}", res.translated_text));
                continue;
            }

            // Preserve Blockquotes (> quote)
            if let Some(quote_text) = trimmed.strip_prefix("> ") {
                let req = TranslationRequest {
                    source_lang,
                    target_lang,
                    text: quote_text.to_string(),
                    preserve_formatting: true,
                    domain_glossary: glossary_owned.clone(),
                };
                let res = self.nmt.translate(&req)?;
                output_lines.push(format!("> {}", res.translated_text));
                continue;
            }

            // Standard Markdown paragraph line
            let req = TranslationRequest {
                source_lang,
                target_lang,
                text: line.to_string(),
                preserve_formatting: true,
                domain_glossary: glossary_owned.clone(),
            };
            let res = self.nmt.translate(&req)?;
            output_lines.push(res.translated_text);
        }

        Ok(output_lines.join("\n"))
    }

    /// Batch translate multiple documents (.txt, .md, .srt, .vtt)
    pub fn batch_translate_documents(
        &self,
        documents: &[BatchDocumentItem],
        source_lang: Option<LanguageCode>,
        target_lang: LanguageCode,
        domain_glossary: Option<&str>,
    ) -> Vec<BatchTranslationResult> {
        documents
            .iter()
            .map(|item| {
                let start = std::time::Instant::now();
                let original_chars = item.content.chars().count();
                let lower_name = item.filename.to_lowercase();

                let detected_format = item
                    .format
                    .as_deref()
                    .unwrap_or_else(|| {
                        if lower_name.ends_with(".srt") {
                            "srt"
                        } else if lower_name.ends_with(".vtt") {
                            "vtt"
                        } else if lower_name.ends_with(".md") || lower_name.ends_with(".markdown") {
                            "md"
                        } else {
                            "txt"
                        }
                    });

                let extension = match detected_format {
                    "srt" => ".srt",
                    "vtt" => ".vtt",
                    "md" | "markdown" => ".md",
                    _ => ".txt",
                };

                let base_name = if let Some(stripped) = item.filename.strip_suffix(extension) {
                    stripped
                } else if let Some(idx) = item.filename.rfind('.') {
                    &item.filename[..idx]
                } else {
                    &item.filename
                };

                let target_tag = target_lang.as_str();
                let translated_filename = format!("{}.{}{}", base_name, target_tag, extension);

                let translation_result: Result<String, SubtitleError> = match detected_format {
                    "srt" => self.translate_subtitle_file(
                        &item.content,
                        SubtitleFormat::Srt,
                        source_lang,
                        target_lang,
                        domain_glossary,
                    ),
                    "vtt" => self.translate_subtitle_file(
                        &item.content,
                        SubtitleFormat::Vtt,
                        source_lang,
                        target_lang,
                        domain_glossary,
                    ),
                    other => self.translate_document(
                        &item.content,
                        other,
                        source_lang,
                        target_lang,
                        domain_glossary,
                    ),
                };

                let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

                match translation_result {
                    Ok(translated_content) => {
                        let translated_chars = translated_content.chars().count();
                        BatchTranslationResult {
                            filename: item.filename.clone(),
                            translated_filename,
                            translated_content,
                            original_chars,
                            translated_chars,
                            elapsed_ms,
                            success: true,
                            error: None,
                        }
                    }
                    Err(err) => BatchTranslationResult {
                        filename: item.filename.clone(),
                        translated_filename,
                        translated_content: String::new(),
                        original_chars,
                        translated_chars: 0,
                        elapsed_ms,
                        success: false,
                        error: Some(err.to_string()),
                    },
                }
            })
            .collect()
    }

    /// Automatically generate timestamped subtitle file (.srt or .vtt) from raw 16kHz audio PCM
    pub fn generate_subtitles_from_audio(
        &self,
        pcm: &[f32],
        audio_lang: LanguageCode,
        target_lang: Option<LanguageCode>,
        format: SubtitleFormat,
    ) -> Result<String, SubtitleError> {
        // 1. Run ASR on audio stream
        let segments = self.speech.recognize(pcm, audio_lang)?;

        // 2. Build subtitle cues
        let mut cues = Vec::new();
        for (idx, seg) in segments.iter().enumerate() {
            let translated_text = if let Some(tgt) = target_lang {
                let req = TranslationRequest {
                    source_lang: Some(audio_lang),
                    target_lang: tgt,
                    text: seg.text.clone(),
                    preserve_formatting: true,
                    domain_glossary: None,
                };
                self.nmt.translate(&req)?.translated_text
            } else {
                seg.text.clone()
            };

            cues.push(SubtitleCue {
                index: idx + 1,
                start_ms: seg.start_ms as u64,
                end_ms: seg.end_ms as u64,
                original_text: seg.text.clone(),
                translated_text,
            });
        }

        // 3. Serialize to format
        Ok(self.serialize_subtitles(&cues, format))
    }
}

// ---------------------------------------------------------------------------
// SRT (SubRip) Parser & Serializer
// ---------------------------------------------------------------------------

fn parse_srt_subtitles(content: &str) -> Result<Vec<SubtitleCue>, SubtitleError> {
    let mut cues = Vec::new();
    let normalized = content.replace("\r\n", "\n");
    let blocks: Vec<&str> = normalized.split("\n\n").collect();

    for block in blocks {
        let lines: Vec<&str> = block.lines().collect();
        if lines.len() < 2 {
            continue;
        }

        let mut line_idx = 0;
        // Optional index number
        let index: usize = if let Ok(num) = lines[line_idx].trim().parse() {
            line_idx += 1;
            num
        } else {
            cues.len() + 1
        };

        if line_idx >= lines.len() {
            continue;
        }

        // Time range line (00:00:01,000 --> 00:00:04,500)
        let time_line = lines[line_idx];
        line_idx += 1;

        if !time_line.contains("-->") {
            continue;
        }

        let parts: Vec<&str> = time_line.split("-->").collect();
        if parts.len() != 2 {
            continue;
        }

        let start_ms = parse_timestamp_ms(parts[0].trim(), ',')
            .ok_or_else(|| SubtitleError::Parse(format!("Invalid start time: {}", parts[0])))?;
        let end_ms = parse_timestamp_ms(parts[1].trim(), ',')
            .ok_or_else(|| SubtitleError::Parse(format!("Invalid end time: {}", parts[1])))?;

        let text = lines[line_idx..].join("\n").trim().to_string();

        cues.push(SubtitleCue {
            index,
            start_ms,
            end_ms,
            original_text: text.clone(),
            translated_text: text,
        });
    }

    Ok(cues)
}

fn serialize_srt_subtitles(cues: &[SubtitleCue]) -> String {
    let mut out = String::new();
    for (i, cue) in cues.iter().enumerate() {
        let idx = i + 1;
        let start_str = format_timestamp_ms(cue.start_ms, ',');
        let end_str = format_timestamp_ms(cue.end_ms, ',');
        let text = if !cue.translated_text.is_empty() {
            &cue.translated_text
        } else {
            &cue.original_text
        };

        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            idx, start_str, end_str, text
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// WebVTT Parser & Serializer
// ---------------------------------------------------------------------------

fn parse_vtt_subtitles(content: &str) -> Result<Vec<SubtitleCue>, SubtitleError> {
    let mut cues = Vec::new();
    let normalized = content.replace("\r\n", "\n");
    let blocks: Vec<&str> = normalized.split("\n\n").collect();

    for block in blocks {
        let trimmed = block.trim();
        if trimmed.starts_with("WEBVTT") || trimmed.is_empty() {
            continue;
        }

        let lines: Vec<&str> = block.lines().collect();
        let mut line_idx = 0;

        let index: usize = if let Ok(num) = lines[line_idx].trim().parse() {
            line_idx += 1;
            num
        } else {
            cues.len() + 1
        };

        if line_idx >= lines.len() {
            continue;
        }

        let time_line = lines[line_idx];
        line_idx += 1;

        if !time_line.contains("-->") {
            continue;
        }

        let parts: Vec<&str> = time_line.split("-->").collect();
        if parts.len() != 2 {
            continue;
        }

        let start_ms = parse_timestamp_ms(parts[0].trim(), '.')
            .ok_or_else(|| SubtitleError::Parse(format!("Invalid start time: {}", parts[0])))?;
        let end_ms = parse_timestamp_ms(parts[1].trim(), '.')
            .ok_or_else(|| SubtitleError::Parse(format!("Invalid end time: {}", parts[1])))?;

        let text = lines[line_idx..].join("\n").trim().to_string();

        cues.push(SubtitleCue {
            index,
            start_ms,
            end_ms,
            original_text: text.clone(),
            translated_text: text,
        });
    }

    Ok(cues)
}

fn serialize_vtt_subtitles(cues: &[SubtitleCue]) -> String {
    let mut out = String::from("WEBVTT\n\n");
    for (i, cue) in cues.iter().enumerate() {
        let idx = i + 1;
        let start_str = format_timestamp_ms(cue.start_ms, '.');
        let end_str = format_timestamp_ms(cue.end_ms, '.');
        let text = if !cue.translated_text.is_empty() {
            &cue.translated_text
        } else {
            &cue.original_text
        };

        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            idx, start_str, end_str, text
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// Timestamp Conversion Helpers
// ---------------------------------------------------------------------------

fn parse_timestamp_ms(s: &str, sep: char) -> Option<u64> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return None;
    }

    let hours: u64 = parts[0].trim().parse().ok()?;
    let minutes: u64 = parts[1].trim().parse().ok()?;

    let sec_parts: Vec<&str> = parts[2].split(sep).collect();
    if sec_parts.len() != 2 {
        return None;
    }

    let seconds: u64 = sec_parts[0].trim().parse().ok()?;
    let millis: u64 = sec_parts[1].trim().parse().ok()?;

    Some((hours * 3600 + minutes * 60 + seconds) * 1000 + millis)
}

fn format_timestamp_ms(ms: u64, sep: char) -> String {
    let total_secs = ms / 1000;
    let millis = ms % 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    format!(
        "{:02}:{:02}:{:02}{}{:03}",
        hours, minutes, seconds, sep, millis
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srt_parsing_and_serialization() {
        let srt_content = "1\n00:01:20,000 --> 00:01:23,500\nHello World!\n\n2\n00:01:24,000 --> 00:01:26,000\nThank you\n\n";
        let nmt = Arc::new(OfflineTranslationEngine::new());
        let speech = Arc::new(BatchSpeechRecognizer::new());
        let translator = SubtitleTranslator::new(nmt, speech);

        let cues = translator
            .parse_subtitles(srt_content, SubtitleFormat::Srt)
            .unwrap();
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].start_ms, 80000);
        assert_eq!(cues[0].end_ms, 83500);
        assert_eq!(cues[0].original_text, "Hello World!");

        let serialized = translator.serialize_subtitles(&cues, SubtitleFormat::Srt);
        assert!(serialized.contains("00:01:20,000 --> 00:01:23,500"));
        assert!(serialized.contains("Hello World!"));
    }

    #[test]
    fn test_vtt_parsing_and_serialization() {
        let vtt_content = "WEBVTT\n\n1\n00:00:10.500 --> 00:00:13.000\nWelcome\n\n";
        let nmt = Arc::new(OfflineTranslationEngine::new());
        let speech = Arc::new(BatchSpeechRecognizer::new());
        let translator = SubtitleTranslator::new(nmt, speech);

        let cues = translator
            .parse_subtitles(vtt_content, SubtitleFormat::Vtt)
            .unwrap();
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].start_ms, 10500);
        assert_eq!(cues[0].original_text, "Welcome");

        let serialized = translator.serialize_subtitles(&cues, SubtitleFormat::Vtt);
        assert!(serialized.starts_with("WEBVTT"));
        assert!(serialized.contains("00:00:10.500 --> 00:00:13.000"));
    }

    #[test]
    fn test_subtitle_translation() {
        let srt_content = "1\n00:00:01,000 --> 00:00:04,000\nHello world\n\n";
        let nmt = Arc::new(OfflineTranslationEngine::new());
        let speech = Arc::new(BatchSpeechRecognizer::new());
        let translator = SubtitleTranslator::new(nmt, speech);

        let translated_srt = translator
            .translate_subtitle_file(
                srt_content,
                SubtitleFormat::Srt,
                Some(LanguageCode::En),
                LanguageCode::Es,
                None,
            )
            .unwrap();

        assert!(translated_srt.contains("Hola mundo"));
        assert!(translated_srt.contains("00:00:01,000 --> 00:00:04,000"));
    }

    #[test]
    fn test_markdown_document_structural_translation() {
        let md_doc =
            "# Welcome\n\nThis is a test.\n\n```rust\nlet x = 42;\n```\n\n- Hello\n- Thank you\n";
        let nmt = Arc::new(OfflineTranslationEngine::new());
        let speech = Arc::new(BatchSpeechRecognizer::new());
        let translator = SubtitleTranslator::new(nmt, speech);

        let translated = translator
            .translate_document(md_doc, "md", Some(LanguageCode::En), LanguageCode::Es, None)
            .unwrap();

        // Heading was translated
        assert!(translated.contains("# Bienvenido"));
        // Code fence block was left untouched
        assert!(translated.contains("let x = 42;"));
        // List items were translated preserving dashes
        assert!(translated.contains("- Hola"));
        assert!(translated.contains("- Gracias"));
    }

    #[test]
    fn test_batch_document_and_glossary_translation() {
        use crate::models::GlossaryTerm;

        let nmt = Arc::new(OfflineTranslationEngine::new());
        nmt.register_glossary(
            "medical",
            vec![GlossaryTerm {
                source_term: "myocardial infarction".to_string(),
                target_term: "infarto agudo de miocardio".to_string(),
                case_sensitive: false,
            }],
        );
        let speech = Arc::new(BatchSpeechRecognizer::new());
        let translator = SubtitleTranslator::new(nmt, speech);

        let items = vec![
            BatchDocumentItem {
                filename: "clinical_report.md".to_string(),
                content: "# Diagnosis\n\nPatient presented with myocardial infarction.".to_string(),
                format: Some("md".to_string()),
            },
            BatchDocumentItem {
                filename: "procedure.srt".to_string(),
                content: "1\n00:00:01,000 --> 00:00:03,000\nHello\n\n2\n00:00:04,000 --> 00:00:07,000\nThank you\n\n".to_string(),
                format: Some("srt".to_string()),
            },
        ];

        let results = translator.batch_translate_documents(
            &items,
            Some(LanguageCode::En),
            LanguageCode::Es,
            Some("medical"),
        );

        assert_eq!(results.len(), 2);
        assert!(results[0].success);
        assert_eq!(results[0].translated_filename, "clinical_report.es.md");
        assert!(results[0].translated_content.contains("infarto agudo de miocardio"));

        assert!(results[1].success);
        assert_eq!(results[1].translated_filename, "procedure.es.srt");
        assert!(results[1].translated_content.contains("Hola"));
        assert!(results[1].translated_content.contains("Gracias"));
    }
}
