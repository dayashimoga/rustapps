---
name: offline-ai-media-pipeline
description: >-
  Use this skill when implementing, optimizing, or testing offline AI models (ASR, NMT
  translation, TTS, OCR, semantic embeddings, AI auto-editing) or real-time audio/video
  DSP and multi-track timeline rendering.
---

# Offline AI & Media Pipeline Engineering Skill

Use this skill when building or modifying `shared/ai-core`, `shared/model-hub`, `shared/media-core`, `apps/universal-live-translator`, `apps/pro-av-editor`, or `apps/ai-file-explorer`.

## 1. Offline AI Model Architecture (`shared/ai-core` & `shared/model-hub`)

1. **Model Packaging & Verification:**
   - Language, ASR, TTS, OCR, and embedding models are defined via `ModelManifest` (ID, version, quantization format `INT8`/`FP16`/`GGUF`/`ONNX`, byte size, `sha256`, `ed25519_signature`, license tier).
   - Models are loaded lazily via memory mapping (`MmapReader`) to keep heap allocations bounded within the $<300\text{ MB}$ base RSS budget.
2. **Inference Pipelines:**
   - **ASR (Speech-to-Text):** Streaming Mel-spectrogram feature extraction (80-bin log-Mel at 16 kHz) + Voice Activity Detection (Silero/WebRTC energy+spectral VAD) + quantized Whisper/Conformer encoder-decoder with timestamp token alignment.
   - **NMT (Translation):** SentencePiece / BPE tokenizer + quantized Transformer Beam/Greedy decoder supporting 100+ languages with automatic language identification (LID via character n-gram & fastText/quantized classifier).
   - **TTS (Text-to-Speech):** Phonemizer + VITS/Piper neural synthesis producing 22.05kHz/24kHz PCM audio streams with adjustable speaking rate and pitch.
   - **OCR & Vision:** True image decoding (`image` crate / pure-Rust PNG/JPEG/WebP decoder) + text detection bounding boxes + CRNN/Transformer line recognition.
   - **Dense Embeddings:** WordPiece/BPE tokenization + quantized MiniLM/BGE projection with SIMD INT8 cosine similarity and HNSW/flat index persistence.

## 2. Professional Audio & Video Engine (`shared/media-core` & `apps/pro-av-editor`)

1. **Non-Destructive Multi-Track Timeline DAG:**
   - Timeline state is an immutable/transactional Directed Acyclic Graph (DAG) of `Track` (Video, Audio, Subtitle/Text, Effect/Adjustment), `Clip` (source media URI, trim `in_ns`/`out_ns`, timeline `start_ns`, speed curve, keyframes), and `Transition` (CrossDissolve, DipToBlack, Wipe, Slide, LumaFade).
   - Every edit operation goes through a `CommandHistory` stack supporting unlimited Undo/Redo and deterministic project serialization (`.avedproj`).
2. **Real-Time Video & Color Pipeline:**
   - Supports 3D LUT (`.cube` parser and trilinear tetrahedral interpolation), Lift-Gamma-Gain color wheels, HSL secondary qualifiers, exposure/contrast/saturation/temperature/tint, vignette, chroma key (green screen), and histogram/waveform/vectorscope scopes.
3. **Real-Time Audio DSP Graph:**
   - Lock-free ring buffers (`rtrb` / atomic SPSC) with zero heap allocations on the audio callback thread.
   - Parametric EQ (biquad cascaded filters: LowShelf, Peaking, HighShelf, HighPass, LowPass), Compressor/Limiter (envelope follower + gain reduction), Noise Gate, Spectral Denoise, Loudness Normalization (EBU R128 LUFS meter), and Crossfade curves.
4. **AI Auto-Editing Features:**
   - **Silence Detection & Auto-Ripple Cut:** RMS/dBFS threshold + minimum silence duration scanner that automatically splits and ripple-deletes dead air.
   - **Scene Cut Detection:** Frame-to-frame histogram/dHash delta analyzer that splits continuous footage into discrete scenes.
   - **Auto-Captions & Beat Sync:** Generates frame-accurate `.srt`/`.vtt` timeline subtitle tracks from audio tracks and detects musical transients/BPM for beat-aligned cuts.

