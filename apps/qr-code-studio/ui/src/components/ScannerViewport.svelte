<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invokeIpc } from '../lib/ipc';

  export let onScanned: () => void = () => {};
  export let onInspectThreats: (payload: string) => void = () => {};

  let videoElement: HTMLVideoElement;
  let canvasElement: HTMLCanvasElement;

  let isStreaming = false;
  let cameraDevices: MediaDeviceInfo[] = [];
  let selectedDeviceId = '';
  let stream: MediaStream | null = null;
  let scanInterval: any = null;

  let isProcessing = false;
  let decryptPassphrase = '';
  let errorMessage = '';

  let scanResult: {
    id: string;
    payload_type: string;
    raw_payload: string;
    structured_fields: Record<string, string>;
    security_report: {
      risk_score: number;
      risk_level: 'Safe' | 'Low' | 'Medium' | 'High' | 'Critical';
      threat_categories: string[];
      findings: string[];
      sanitized_url: string | null;
      is_air_gap_safe: boolean;
    };
    signature_status: 'None' | 'Verified' | 'SignatureMismatch' | 'MalformedSignature';
    is_encrypted: boolean;
    source: string;
  } | null = null;

  onMount(async () => {
    try {
      if (navigator.mediaDevices && navigator.mediaDevices.enumerateDevices) {
        const devices = await navigator.mediaDevices.enumerateDevices();
        cameraDevices = devices.filter(d => d.kind === 'videoinput');
        if (cameraDevices.length > 0) {
          selectedDeviceId = cameraDevices[0].deviceId;
        }
      }
    } catch (e) {
      console.warn('Media devices enumeration unavailable:', e);
    }
  });

  onDestroy(() => {
    stopCamera();
  });

  async function startCamera() {
    errorMessage = '';
    try {
      const constraints: MediaStreamConstraints = {
        video: selectedDeviceId ? { deviceId: { exact: selectedDeviceId } } : true,
        audio: false,
      };
      stream = await navigator.mediaDevices.getUserMedia(constraints);
      if (videoElement) {
        videoElement.srcObject = stream;
        videoElement.play();
        isStreaming = true;
        scanInterval = setInterval(captureAndScanFrame, 600);
      }
    } catch (e: any) {
      errorMessage = 'Camera access error: ' + (e?.message || String(e));
      isStreaming = false;
    }
  }

  function stopCamera() {
    if (scanInterval) {
      clearInterval(scanInterval);
      scanInterval = null;
    }
    if (stream) {
      stream.getTracks().forEach(track => track.stop());
      stream = null;
    }
    isStreaming = false;
  }

  async function captureAndScanFrame() {
    if (!videoElement || !canvasElement || !isStreaming || isProcessing) return;

    const ctx = canvasElement.getContext('2d');
    if (!ctx) return;

    canvasElement.width = videoElement.videoWidth || 640;
    canvasElement.height = videoElement.videoHeight || 480;
    ctx.drawImage(videoElement, 0, 0, canvasElement.width, canvasElement.height);

    const b64 = canvasElement.toDataURL('image/png');
    await scanBase64Image(b64);
  }

  async function handleFileUpload(e: Event) {
    const target = e.target as HTMLInputElement;
    if (!target.files || target.files.length === 0) return;
    const file = target.files[0];

    const reader = new FileReader();
    reader.onload = async () => {
      const b64 = reader.result as string;
      await scanBase64Image(b64);
    };
    reader.readAsDataURL(file);
  }

  async function scanBase64Image(b64: string) {
    if (isProcessing) return;
    isProcessing = true;
    errorMessage = '';

    try {
      const result = await invokeIpc<any>('scan_qr_image', {
        png_base64: b64,
        passphrase: decryptPassphrase.trim() ? decryptPassphrase.trim() : null,
      });

      scanResult = result;
      onScanned();
    } catch (e: any) {
      // Don't show error if streaming frames simply don't contain a QR yet
      if (!isStreaming) {
        errorMessage = e?.message || String(e);
      }
    } finally {
      isProcessing = false;
    }
  }

  async function unlockEncrypted() {
    if (!scanResult || !decryptPassphrase.trim()) return;
    // Re-scan with entered passphrase
    try {
      const result = await invokeIpc<any>('scan_qr_image', {
        file_path: null,
        passphrase: decryptPassphrase.trim(),
      });
      scanResult = result;
    } catch (e: any) {
      errorMessage = 'Failed to decrypt: ' + (e?.message || String(e));
    }
  }

  function copyText() {
    if (!scanResult) return;
    navigator.clipboard.writeText(scanResult.raw_payload);
  }

  function getRiskBadgeColor(level: string): string {
    switch (level) {
      case 'Safe': return 'var(--risk-safe)';
      case 'Low': return 'var(--risk-low)';
      case 'Medium': return 'var(--risk-medium)';
      case 'High': return 'var(--risk-high)';
      case 'Critical': return 'var(--risk-critical)';
      default: return 'var(--text-muted)';
    }
  }
</script>

<div class="scanner-container">
  <!-- Viewport Area -->
  <div class="viewport-panel">
    <div class="viewport-header">
      <div class="header-titles">
        <h2>Live Scanner & Image Decoder</h2>
        <p class="subtitle">Offline Pure-Rust Barcode Recognition & Anti-Quishing Engine</p>
      </div>
      <div class="camera-controls">
        {#if cameraDevices.length > 0}
          <select bind:value={selectedDeviceId} disabled={isStreaming} class="select-camera">
            {#each cameraDevices as device}
              <option value={device.deviceId}>{device.label || 'Webcam ' + device.deviceId.slice(0, 5)}</option>
            {/each}
          </select>
        {/if}

        {#if !isStreaming}
          <button class="btn-start" on:click={startCamera}>
            ▶ Start Live Camera
          </button>
        {:else}
          <button class="btn-stop" on:click={stopCamera}>
            ⏹ Stop Camera
          </button>
        {/if}
      </div>
    </div>

    <!-- Video Feed or File Drop View -->
    <div class="viewport-stage">
      <video
        bind:this={videoElement}
        autoplay
        playsinline
        muted
        class="video-feed"
        class:hidden={!isStreaming}
      ></video>
      <canvas bind:this={canvasElement} class="hidden-canvas"></canvas>

      {#if !isStreaming}
        <div class="drop-zone">
          <div class="drop-content">
            <span class="drop-icon">📷</span>
            <h3>Drag & Drop QR Image File</h3>
            <p>Supports PNG, JPG, BMP formats without external dependencies.</p>
            <label class="btn-file-select">
              <span>Select Local File</span>
              <input type="file" accept="image/png,image/jpeg,image/bmp" on:change={handleFileUpload} />
            </label>
          </div>
        </div>
      {/if}

      {#if isStreaming}
        <div class="reticle-overlay">
          <div class="reticle-box"></div>
          <span class="reticle-text">Position QR code inside bounds</span>
        </div>
      {/if}
    </div>

    {#if errorMessage}
      <div class="scanner-error">
        ⚠️ {errorMessage}
      </div>
    {/if}
  </div>

  <!-- Scan Result & Security Analysis Sidebar -->
  <div class="result-panel">
    {#if scanResult}
      <div class="result-card">
        <div class="result-header">
          <span class="type-pill">{scanResult.payload_type}</span>
          <span
            class="risk-pill"
            style="background: {getRiskBadgeColor(scanResult.security_report.risk_level)}22; color: {getRiskBadgeColor(scanResult.security_report.risk_level)}; border: 1px solid {getRiskBadgeColor(scanResult.security_report.risk_level)}55;"
          >
            Risk: {scanResult.security_report.risk_level} ({scanResult.security_report.risk_score}/100)
          </span>
        </div>

        {#if scanResult.is_encrypted}
          <div class="encrypted-banner">
            <span class="icon">🔒</span>
            <div>
              <strong>AES-GCM Encrypted Payload</strong>
              <p>Enter the passphrase used during creation to reveal contents.</p>
            </div>
            <div class="decrypt-row">
              <input
                type="password"
                placeholder="Passphrase..."
                bind:value={decryptPassphrase}
                class="decrypt-input"
              />
              <button class="btn-decrypt" on:click={unlockEncrypted}>Unlock</button>
            </div>
          </div>
        {/if}

        {#if scanResult.signature_status === 'Verified'}
          <div class="signature-banner verified">
            ✓ Ed25519 Cryptographic Signature Verified
          </div>
        {:else if scanResult.signature_status === 'SignatureMismatch'}
          <div class="signature-banner mismatch">
            ⚠️ Signature Mismatch! Payload has been tampered with.
          </div>
        {/if}

        <!-- Payload Content Box -->
        <div class="payload-box">
          <span class="box-label">Payload Content</span>
          <div class="raw-content-display">
            {scanResult.raw_payload}
          </div>
        </div>

        <!-- Structured Fields if Available -->
        {#if Object.keys(scanResult.structured_fields).length > 0}
          <div class="structured-box">
            <span class="box-label">Parsed Attributes</span>
            <div class="fields-list">
              {#each Object.entries(scanResult.structured_fields) as [k, v]}
                <div class="field-item">
                  <span class="field-key">{k}:</span>
                  <span class="field-val">{v}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Threat Findings Summary -->
        {#if scanResult.security_report.findings.length > 0}
          <div class="threats-box">
            <span class="box-label">Threat Heuristics</span>
            <ul class="threats-list">
              {#each scanResult.security_report.findings as finding}
                <li>⚠️ {finding}</li>
              {/each}
            </ul>
          </div>
        {/if}

        <!-- Actions -->
        <div class="result-actions">
          <button class="btn-action" on:click={copyText}>
            📋 Copy Payload
          </button>
          <button class="btn-action secondary" on:click={() => onInspectThreats(scanResult?.raw_payload || '')}>
            🛡️ Inspect Threats
          </button>
        </div>
      </div>
    {:else}
      <div class="empty-result">
        <span class="empty-icon">🔍</span>
        <h3>Ready to Scan</h3>
        <p>Start your camera or drag an image into the scanner viewport to detect and analyze QR codes in real-time.</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .scanner-container {
    display: grid;
    grid-template-columns: 1fr 440px;
    height: 100%;
    overflow: hidden;
    gap: 1.5rem;
    padding: 1.5rem;
  }

  .viewport-panel {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .viewport-header {
    padding: 1.25rem 1.5rem;
    border-bottom: 1px solid var(--border-color);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .header-titles h2 {
    font-size: 1.25rem;
    font-weight: 700;
  }

  .subtitle {
    font-size: 0.8rem;
    color: var(--text-muted);
    margin-top: 0.25rem;
  }

  .camera-controls {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .select-camera {
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    border-radius: 6px;
    padding: 0.45rem 0.65rem;
    font-size: 0.85rem;
  }

  .btn-start, .btn-stop {
    padding: 0.45rem 0.85rem;
    border-radius: 6px;
    font-weight: 600;
    font-size: 0.85rem;
    cursor: pointer;
    border: none;
    transition: all 0.15s ease;
  }

  .btn-start {
    background: var(--accent-emerald);
    color: #0b0f19;
  }

  .btn-start:hover {
    background: #34d399;
  }

  .btn-stop {
    background: var(--accent-rose);
    color: #fff;
  }

  .btn-stop:hover {
    background: #fb7185;
  }

  .viewport-stage {
    flex: 1;
    position: relative;
    background: #000000;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }

  .video-feed {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .video-feed.hidden, .hidden-canvas {
    display: none;
  }

  .drop-zone {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 2px dashed rgba(75, 85, 99, 0.4);
    margin: 1.5rem;
    border-radius: 12px;
  }

  .drop-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 0.75rem;
    max-width: 320px;
  }

  .drop-icon {
    font-size: 2.5rem;
  }

  .drop-content h3 {
    font-size: 1.1rem;
    color: var(--text-primary);
  }

  .drop-content p {
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .btn-file-select {
    display: inline-block;
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    padding: 0.5rem 1rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.85rem;
    font-weight: 600;
    margin-top: 0.5rem;
  }

  .btn-file-select input[type="file"] {
    display: none;
  }

  .reticle-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    pointer-events: none;
  }

  .reticle-box {
    width: 240px;
    height: 240px;
    border: 2px solid var(--accent-cyan);
    border-radius: 16px;
    box-shadow: 0 0 20px rgba(6, 182, 212, 0.4);
  }

  .reticle-text {
    margin-top: 1rem;
    background: rgba(0, 0, 0, 0.7);
    color: var(--text-secondary);
    font-size: 0.8rem;
    padding: 0.25rem 0.75rem;
    border-radius: 9999px;
  }

  .scanner-error {
    background: rgba(239, 68, 68, 0.15);
    border-top: 1px solid rgba(239, 68, 68, 0.4);
    color: #fca5a5;
    padding: 0.75rem 1.5rem;
    font-size: 0.85rem;
  }

  .result-panel {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    padding: 1.5rem;
  }

  .result-card {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .result-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .type-pill {
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    padding: 0.3rem 0.65rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--accent-cyan);
  }

  .risk-pill {
    padding: 0.3rem 0.65rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 700;
  }

  .signature-banner {
    padding: 0.5rem 0.75rem;
    border-radius: 6px;
    font-size: 0.8rem;
    font-weight: 600;
  }

  .signature-banner.verified {
    background: rgba(16, 185, 129, 0.15);
    color: var(--accent-emerald);
    border: 1px solid rgba(16, 185, 129, 0.3);
  }

  .signature-banner.mismatch {
    background: rgba(239, 68, 68, 0.15);
    color: var(--accent-rose);
    border: 1px solid rgba(239, 68, 68, 0.3);
  }

  .encrypted-banner {
    background: rgba(99, 102, 241, 0.15);
    border: 1px solid rgba(99, 102, 241, 0.3);
    border-radius: 8px;
    padding: 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .decrypt-row {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.25rem;
  }

  .decrypt-input {
    flex: 1;
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    border-radius: 4px;
    padding: 0.4rem 0.6rem;
    font-size: 0.85rem;
  }

  .btn-decrypt {
    background: var(--accent-indigo);
    color: #fff;
    border: none;
    border-radius: 4px;
    padding: 0.4rem 0.8rem;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
  }

  .payload-box, .structured-box, .threats-box {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .payload-box .box-label, .structured-box .box-label, .threats-box .box-label {
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .raw-content-display {
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    padding: 0.75rem;
    border-radius: 6px;
    font-family: var(--font-mono);
    font-size: 0.85rem;
    word-break: break-all;
    max-height: 140px;
    overflow-y: auto;
  }

  .fields-list {
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    padding: 0.5rem 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .field-item {
    font-size: 0.8rem;
    display: flex;
    justify-content: space-between;
  }

  .field-key {
    color: var(--text-muted);
  }

  .field-val {
    font-family: var(--font-mono);
    color: var(--accent-cyan);
  }

  .threats-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .threats-list li {
    font-size: 0.75rem;
    color: var(--accent-amber);
    background: rgba(245, 158, 11, 0.1);
    border: 1px solid rgba(245, 158, 11, 0.2);
    padding: 0.4rem 0.6rem;
    border-radius: 4px;
  }

  .result-actions {
    display: flex;
    gap: 0.75rem;
    margin-top: 0.5rem;
  }

  .btn-action {
    flex: 1;
    background: var(--accent-cyan);
    color: #0b0f19;
    border: none;
    border-radius: 6px;
    padding: 0.6rem;
    font-weight: 600;
    font-size: 0.85rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .btn-action:hover {
    background: #22d3ee;
  }

  .btn-action.secondary {
    background: var(--bg-elevated);
    color: var(--text-primary);
    border: 1px solid var(--border-color);
  }

  .btn-action.secondary:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .empty-result {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    height: 100%;
    gap: 0.75rem;
    color: var(--text-muted);
    padding: 2rem;
  }

  .empty-icon {
    font-size: 2.5rem;
  }

  .empty-result h3 {
    color: var(--text-primary);
    font-size: 1.1rem;
  }

  .empty-result p {
    font-size: 0.85rem;
    line-height: 1.5;
  }
</style>
