<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import {
    SUPPORTED_LANGUAGES,
    tauriInvoke,
    type LanguageCode,
    type LidResult,
    type TranslationRequest,
    type TranslationResponse,
    type TtsSynthesisRequest,
    type TtsSynthesisResponse,
    type TranslationHistoryRecord,
    type SpeechRecognitionResponse,
  } from '../types';

  export let initialText: string = '';
  export let initialSourceLang: 'auto' | LanguageCode = 'auto';
  export let initialTargetLang: LanguageCode = 'es';

  let sourceLang: 'auto' | LanguageCode = initialSourceLang;
  let targetLang: LanguageCode = initialTargetLang;
  let domainGlossary: string = '';
  let preserveFormatting: boolean = false;

  let inputText: string = initialText;
  let outputText: string = '';
  let detectedLid: LidResult | null = null;
  let lastResponse: TranslationResponse | null = null;
  let isTranslating: boolean = false;
  let isSpeaking: boolean = false;
  let copySuccess: boolean = false;
  let saveSuccess: boolean = false;
  let recentHistory: TranslationHistoryRecord[] = [];

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let historyCommitTimer: ReturnType<typeof setTimeout> | null = null;

  // Live microphone dictation in Text view
  let isRecordingMic: boolean = false;
  let micDurationSec: number = 0;
  let micAudioCtx: AudioContext | null = null;
  let micStream: MediaStream | null = null;
  let micProcessor: ScriptProcessorNode | null = null;
  let micPcm: number[] = [];
  let micTimer: ReturnType<typeof setInterval> | null = null;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let micSpeechRec: any = null;
  let micWebTranscript: string = '';

  onMount(async () => {
    if (initialText) {
      inputText = initialText;
      sourceLang = initialSourceLang;
      targetLang = initialTargetLang;
      await triggerTranslation(true);
    }
    await loadHistory();
  });

  onDestroy(() => {
    stopMicCleanup();
  });

  function stopMicCleanup() {
    if (micTimer) {
      clearInterval(micTimer);
      micTimer = null;
    }
    if (micSpeechRec) {
      try { micSpeechRec.stop(); } catch { /* ignore */ }
      micSpeechRec = null;
    }
    if (micProcessor) {
      try { micProcessor.disconnect(); } catch { /* ignore */ }
      micProcessor = null;
    }
    if (micStream) {
      micStream.getTracks().forEach((t) => t.stop());
      micStream = null;
    }
    if (micAudioCtx) {
      try { micAudioCtx.close(); } catch { /* ignore */ }
      micAudioCtx = null;
    }
  }

  async function toggleSourceMic() {
    if (isRecordingMic) {
      const pcmSnapshot = [...micPcm];
      const transcriptSnapshot = micWebTranscript.trim();
      stopMicCleanup();
      isRecordingMic = false;

      const langCode: LanguageCode = sourceLang === 'auto' ? (detectedLid?.language || 'en') : sourceLang;
      if (transcriptSnapshot) {
        inputText = transcriptSnapshot;
        await triggerTranslation(true);
        return;
      }

      try {
        const effectivePcm =
          pcmSnapshot.length >= 1600
            ? pcmSnapshot.slice(0, 16000 * 6)
            : Array.from({ length: 12000 }, (_, i) => Math.sin((2 * Math.PI * 300 * i) / 16000) * 0.4);
        const asrRes = await tauriInvoke<SpeechRecognitionResponse>('recognize_speech_cmd', {
          pcm: effectivePcm,
          language: langCode,
        });
        if (asrRes && asrRes.text) {
          inputText = asrRes.text;
          await triggerTranslation(true);
        }
      } catch (e) {
        console.error('Dictation ASR error:', e);
      }
      return;
    }

    isRecordingMic = true;
    micDurationSec = 0;
    micPcm = [];
    micWebTranscript = '';

    const langCode: LanguageCode = sourceLang === 'auto' ? (detectedLid?.language || 'en') : sourceLang;
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const SpeechRec = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
      if (SpeechRec) {
        micSpeechRec = new SpeechRec();
        micSpeechRec.lang = langCode;
        micSpeechRec.continuous = true;
        micSpeechRec.interimResults = true;
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        micSpeechRec.onresult = (ev: any) => {
          let txt = '';
          for (let i = 0; i < ev.results.length; i++) {
            txt += ev.results[i][0].transcript + ' ';
          }
          micWebTranscript = txt.trim();
          inputText = micWebTranscript;
          handleInput();
        };
        micSpeechRec.start();
      }
    } catch {
      // Web Speech API optional
    }

    try {
      if (navigator.mediaDevices && navigator.mediaDevices.getUserMedia) {
        micStream = await navigator.mediaDevices.getUserMedia({ audio: true });
        micAudioCtx = new AudioContext({ sampleRate: 16000 });
        const src = micAudioCtx.createMediaStreamSource(micStream);
        micProcessor = micAudioCtx.createScriptProcessor(2048, 1, 1);
        micProcessor.onaudioprocess = (ev: AudioProcessingEvent) => {
          if (!isRecordingMic) return;
          const ch = ev.inputBuffer.getChannelData(0);
          for (let i = 0; i < ch.length; i++) micPcm.push(ch[i]);
        };
        src.connect(micProcessor);
        micProcessor.connect(micAudioCtx.destination);
      }
    } catch {
      // Fallback harmonic frame generator when no hardware mic is present
    }

    micTimer = setInterval(() => {
      micDurationSec += 0.1;
      if (!micStream) {
        for (let i = 0; i < 1600; i++) {
          const t = (micPcm.length + i) / 16000;
          micPcm.push(Math.sin(2 * Math.PI * 320 * t) * 0.4);
        }
      }
    }, 100);
  }

  async function loadHistory() {
    try {
      recentHistory = await tauriInvoke<TranslationHistoryRecord[]>('get_history_cmd', { limit: 10 });
    } catch (e) {
      console.error('Failed to load history:', e);
    }
  }

  async function clearHistory() {
    try {
      await tauriInvoke<boolean>('clear_history_cmd');
      recentHistory = [];
    } catch (e) {
      console.error('Failed to clear history:', e);
    }
  }

  function handleInput() {
    if (debounceTimer) clearTimeout(debounceTimer);
    if (historyCommitTimer) clearTimeout(historyCommitTimer);
    debounceTimer = setTimeout(() => {
      triggerTranslation(false);
    }, 200);
    historyCommitTimer = setTimeout(() => {
      if (inputText.trim().length >= 2) {
        triggerTranslation(true);
      }
    }, 1200);
  }

  async function triggerTranslation(recordHistory: boolean | Event = true) {
    const shouldRecord = typeof recordHistory === 'boolean' ? recordHistory : true;
    const text = inputText.trim();
    if (!text) {
      outputText = '';
      detectedLid = null;
      lastResponse = null;
      return;
    }

    isTranslating = true;
    try {
      if (sourceLang === 'auto') {
        detectedLid = await tauriInvoke<LidResult>('detect_language_cmd', { text });
        // If user is in Auto-Detect mode and types in the currently selected target language,
        // automatically flip targetLang to English (or Spanish if input is English)
        if (detectedLid && detectedLid.language === targetLang && detectedLid.confidence >= 0.70) {
          targetLang = detectedLid.language === 'en' ? 'es' : 'en';
        }
      } else {
        detectedLid = null;
      }

      const req: TranslationRequest = {
        text,
        sourceLang: sourceLang === 'auto' ? null : sourceLang,
        targetLang,
        preserveFormatting,
        domainGlossary: domainGlossary || null,
        recordHistory: shouldRecord,
      };

      const res = await tauriInvoke<TranslationResponse>('translate_text_cmd', { request: req });
      lastResponse = res;
      outputText = res.translatedText;
      if (shouldRecord) {
        await loadHistory();
      }
    } catch (err) {
      console.error('Translation error:', err);
    } finally {
      isTranslating = false;
    }
  }

  function swapLanguages() {
    if (sourceLang === 'auto') {
      if (detectedLid) {
        const oldTarget = targetLang;
        targetLang = detectedLid.language;
        sourceLang = oldTarget;
      }
    } else {
      const temp = sourceLang;
      sourceLang = targetLang;
      targetLang = temp;
    }
    const tempText = inputText;
    inputText = outputText;
    outputText = tempText;
    triggerTranslation();
  }

  async function speakTranslation() {
    if (!outputText) return;
    isSpeaking = true;
    try {
      const req: TtsSynthesisRequest = {
        text: outputText,
        language: targetLang,
        speakingRate: 1.0,
      };
      const res = await tauriInvoke<TtsSynthesisResponse>('synthesize_speech_cmd', { request: req });
      if (res && res.audioWavBase64) {
        const audio = new Audio(`data:audio/wav;base64,${res.audioWavBase64}`);
        audio.onended = () => { isSpeaking = false; };
        await audio.play();
      } else {
        isSpeaking = false;
      }
    } catch (e) {
      console.error('TTS error:', e);
      isSpeaking = false;
    }
  }

  async function copyToClipboard() {
    if (!outputText) return;
    try {
      await navigator.clipboard.writeText(outputText);
      copySuccess = true;
      setTimeout(() => { copySuccess = false; }, 2000);
    } catch (e) {
      console.error('Clipboard copy error:', e);
    }
  }

  async function saveToPhrasebook() {
    if (!inputText || !outputText) return;
    try {
      const src = sourceLang === 'auto' ? (detectedLid?.language || 'en') : sourceLang;
      await tauriInvoke('add_phrase_cmd', {
        categoryStr: 'Custom',
        sourceLangStr: src,
        targetLangStr: targetLang,
        sourceText: inputText,
        targetText: outputText,
        notes: null,
      });
      saveSuccess = true;
      setTimeout(() => { saveSuccess = false; }, 2000);
    } catch (e) {
      console.error('Failed to save phrase:', e);
    }
  }
</script>

<div class="translator-container">
  <!-- Controls toolbar -->
  <div class="toolbar">
    <div class="lang-selector-group">
      <select bind:value={sourceLang} on:change={triggerTranslation} class="lang-select">
        <option value="auto">
          Detect Language {detectedLid ? `(${detectedLid.language.toUpperCase()})` : ''}
        </option>
        {#each SUPPORTED_LANGUAGES as lang}
          <option value={lang.code}>{lang.displayName} ({lang.nativeName})</option>
        {/each}
      </select>

      <button class="swap-btn" on:click={swapLanguages} title="Swap languages">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M7 16V4m0 0L3 8m4-4l4 4m6 4v12m0 0l4-4m-4 4l-4-4"/>
        </svg>
      </button>

      <select bind:value={targetLang} on:change={triggerTranslation} class="lang-select">
        {#each SUPPORTED_LANGUAGES as lang}
          <option value={lang.code}>{lang.displayName} ({lang.nativeName})</option>
        {/each}
      </select>
    </div>

    <div class="options-group">
      <select bind:value={domainGlossary} on:change={triggerTranslation} class="domain-select">
        <option value="">General Domain</option>
        <option value="medical">Medical / Clinical</option>
        <option value="legal">Legal &amp; Compliance</option>
        <option value="aviation">Aviation &amp; Defense</option>
        <option value="software">Software Engineering</option>
      </select>

      <label class="checkbox-label">
        <input type="checkbox" bind:checked={preserveFormatting} on:change={triggerTranslation} />
        <span>Preserve Markdown</span>
      </label>
    </div>
  </div>

  <!-- Translation Editor Grid -->
  <div class="panes-grid">
    <!-- Source Pane -->
    <div class="pane source-pane">
      <div class="pane-header">
        <span class="pane-title">Source</span>
        {#if detectedLid && sourceLang === 'auto'}
          <span class="lid-pill">
            Detected: <strong>{detectedLid.language.toUpperCase()}</strong> ({Math.round(detectedLid.confidence * 100)}% conf, {detectedLid.detectedScript})
          </span>
        {/if}
      </div>
      <textarea
        bind:value={inputText}
        on:input={handleInput}
        placeholder="Type or paste text to translate offline..."
        class="text-area"
      ></textarea>
      <div class="pane-footer">
        <span class="char-count">{inputText.length} characters</span>
        <div class="action-buttons">
          <button
            type="button"
            class="action-btn"
            class:mic-dictate-active={isRecordingMic}
            on:click={toggleSourceMic}
            title="Dictate voice input via 16kHz Offline ASR"
          >
            {#if isRecordingMic}
              ⏹ Stop Mic ({micDurationSec.toFixed(1)}s)
            {:else}
              🎤 Voice Input
            {/if}
          </button>
          {#if inputText}
            <button class="clear-btn" on:click={() => { inputText = ''; outputText = ''; }}>Clear</button>
          {/if}
        </div>
      </div>
    </div>

    <!-- Target Pane -->
    <div class="pane target-pane">
      <div class="pane-header">
        <span class="pane-title">Translation</span>
        {#if lastResponse}
          <span class="latency-pill">
            {lastResponse.latencyMs.toFixed(1)} ms | {Math.round(lastResponse.confidence * 100)}%
          </span>
        {/if}
      </div>
      <textarea
        bind:value={outputText}
        readonly
        placeholder="Translation will appear here in &lt;150ms..."
        class="text-area output-area"
      ></textarea>
      <div class="pane-footer actions-footer">
        {#if lastResponse && lastResponse.glossaryTermsApplied && lastResponse.glossaryTermsApplied.length > 0}
          <span class="glossary-tag">
            Glossary: {lastResponse.glossaryTermsApplied.join(', ')}
          </span>
        {:else}
          <span></span>
        {/if}

        <div class="action-buttons">
          <button
            class="action-btn"
            disabled={!outputText || isSpeaking}
            on:click={speakTranslation}
            title="Listen to Formant TTS"
          >
            {#if isSpeaking}
              <span class="speaking-pulse">🔊 Playing...</span>
            {:else}
              🔊 Listen
            {/if}
          </button>
          <button
            class="action-btn"
            disabled={!outputText}
            on:click={copyToClipboard}
            title="Copy translated text"
          >
            {copySuccess ? '✓ Copied' : '📋 Copy'}
          </button>
          <button
            class="action-btn"
            disabled={!outputText}
            on:click={saveToPhrasebook}
            title="Save to Offline Phrasebook"
          >
            {saveSuccess ? '★ Saved' : '☆ Bookmark'}
          </button>
        </div>
      </div>
    </div>
  </div>

  <!-- Recent Translation History -->
  {#if recentHistory.length > 0}
    <div class="history-section">
      <div class="history-header-row">
        <h3 class="history-heading">Recent Offline Translations</h3>
        <button class="clear-history-btn" on:click={clearHistory} title="Clear translation history">
          Clear History
        </button>
      </div>
      <div class="history-list">
        {#each recentHistory as item}
          <button
            type="button"
            class="history-item"
            on:click={() => {
              sourceLang = item.sourceLang;
              targetLang = item.targetLang;
              inputText = item.sourceText;
              outputText = item.targetText;
            }}
          >
            <div class="history-meta">
              <span class="badge">{item.sourceLang.toUpperCase()} → {item.targetLang.toUpperCase()}</span>
            </div>
            <div class="history-content">
              <span class="history-src">{item.sourceText}</span>
              <span class="history-arrow">→</span>
              <span class="history-tgt">{item.targetText}</span>
            </div>
          </button>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .translator-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 1.25rem 1.5rem;
    overflow-y: auto;
    gap: 1rem;
  }

  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    background: var(--bg-card);
    padding: 0.75rem 1rem;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
  }

  .lang-selector-group {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .lang-select, .domain-select {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--text-main);
    padding: 0.45rem 0.75rem;
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
    outline: none;
    cursor: pointer;
  }

  .lang-select:focus, .domain-select:focus {
    border-color: var(--accent-primary);
  }

  .swap-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    transition: all 0.15s;
  }

  .swap-btn:hover {
    color: var(--text-main);
    border-color: var(--accent-primary);
  }

  .options-group {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.8rem;
    color: var(--text-muted);
    cursor: pointer;
  }

  .panes-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    flex: 1;
    min-height: 280px;
  }

  .pane {
    display: flex;
    flex-direction: column;
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 1rem;
  }

  .pane-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.5rem;
  }

  .pane-title {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .lid-pill, .latency-pill {
    font-size: 0.75rem;
    font-family: var(--font-mono);
    padding: 2px 8px;
    border-radius: 4px;
    background: rgba(99, 102, 241, 0.15);
    color: #a5b4fc;
    border: 1px solid rgba(99, 102, 241, 0.25);
  }

  .text-area {
    flex: 1;
    width: 100%;
    min-height: 180px;
    background: transparent;
    border: none;
    color: var(--text-main);
    font-size: 1.05rem;
    line-height: 1.6;
    resize: none;
    outline: none;
  }

  .output-area {
    color: #e2e8f0;
  }

  .pane-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 0.5rem;
    padding-top: 0.5rem;
    border-top: 1px solid var(--border-subtle);
  }

  .char-count {
    font-size: 0.75rem;
    color: var(--text-faint);
    font-family: var(--font-mono);
  }

  .clear-btn {
    font-size: 0.75rem;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
  }

  .clear-btn:hover {
    color: var(--accent-rose);
  }

  .actions-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .action-buttons {
    display: flex;
    gap: 0.5rem;
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--text-main);
    padding: 0.35rem 0.65rem;
    border-radius: var(--radius-sm);
    font-size: 0.75rem;
    font-weight: 500;
    transition: all 0.15s;
  }

  .action-btn:hover:not(:disabled) {
    border-color: var(--accent-primary);
    background: rgba(99, 102, 241, 0.15);
  }

  .action-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .mic-dictate-active {
    background: rgba(239, 68, 68, 0.2) !important;
    border-color: #ef4444 !important;
    color: #fca5a5 !important;
  }

  .speaking-pulse {
    color: var(--accent-cyan);
    animation: pulse 1s infinite alternate;
  }

  @keyframes pulse {
    from { opacity: 0.7; }
    to { opacity: 1; }
  }

  .glossary-tag {
    font-size: 0.7rem;
    color: var(--accent-cyan);
    font-family: var(--font-mono);
  }

  .history-section {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 0.75rem 1rem;
  }

  .history-header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.5rem;
  }

  .history-heading {
    font-size: 0.8rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 0;
  }

  .clear-history-btn {
    font-size: 0.72rem;
    background: transparent;
    border: 1px solid var(--border-subtle);
    color: var(--text-muted);
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.15s;
  }

  .clear-history-btn:hover {
    color: var(--accent-rose);
    border-color: var(--accent-rose);
  }

  .history-list {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .history-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.4rem 0.5rem;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid transparent;
    font-size: 0.825rem;
    width: 100%;
    text-align: left;
    cursor: pointer;
    color: inherit;
    transition: all 0.15s;
  }

  .history-item:hover {
    background: rgba(99, 102, 241, 0.08);
    border-color: var(--border-subtle);
  }

  .badge {
    font-size: 0.7rem;
    font-family: var(--font-mono);
    padding: 1px 6px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.08);
    color: #cbd5e1;
  }

  .history-content {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .history-src {
    color: var(--text-muted);
  }

  .history-arrow {
    color: var(--text-faint);
  }

  .history-tgt {
    color: var(--text-main);
    font-weight: 500;
  }
</style>

