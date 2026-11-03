#![forbid(unsafe_code)]
//! Shared AI & Media Processing Core Subsystem
//!
//! Provides offline subword BPE tokenization, concept-aware dense semantic embeddings,
//! genuine PNG & BMP raster decoding, 8x8 glyph template OCR, and 16kHz audio VAD & Mel DSP.

pub mod audio_dsp;
pub mod embeddings;
pub mod image_decoder;
pub mod ocr_engine;
pub mod tokenizer;

pub use audio_dsp::{MelSpectrogramExtractor, SpeechSegment, VadEngine};
pub use embeddings::{ConceptEmbeddingEngine, DenseEmbedding, EMBEDDING_DIM};
pub use image_decoder::{encode_gray_to_bmp, DecodedImage, ImageDecodeError, ImageDecoder};
pub use ocr_engine::{render_text_to_gray_image, GlyphTable, OcrResult, TemplateOcrEngine};
pub use tokenizer::BpeTokenizer;
