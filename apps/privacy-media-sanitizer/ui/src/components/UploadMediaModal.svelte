<script lang="ts">
  import { invokeIpc } from '../lib/ipc';
  import type { MediaInspectionReport } from '../lib/types';

  export let isOpen = false;
  export let onClose: () => void = () => {};
  export let onMediaInspected: (report: MediaInspectionReport) => void = () => {};

  let filename = '';
  let payloadText = '';
  let isLoading = false;
  let errorMessage = '';

  function loadSampleJpeg() {
    filename = 'Surveillance_Field_Photo.jpg';
    payloadText = 'EXIF Sony Alpha ILCE-7RM5 GPS 37.7749N 122.4194W SN-SONY-991204 Lens FE 24-70mm GM II';
  }

  function loadSamplePdf() {
    filename = 'Unredacted_Investigation_Report.pdf';
    payloadText = '%PDF-1.7 /Author (Federal Forensic Investigator) /Producer (Adobe Acrobat Pro 2026) <x:xmpmeta>UUID-8941</x:xmpmeta>';
  }

  async function startInspection() {
    if (!filename.trim()) {
      errorMessage = 'Please provide a media filename';
      return;
    }
    if (!payloadText.trim()) {
      errorMessage = 'Please provide media payload bytes or text';
      return;
    }

    isLoading = true;
    errorMessage = '';

    try {
      const rep = await invokeIpc<MediaInspectionReport>('inspect_upload', {
        filename: filename.trim(),
        raw_text: payloadText.trim(),
      });

      onMediaInspected(rep);
      onClose();
    } catch (e: any) {
      errorMessage = `Inspection failed: ${e.message || e}`;
    } finally {
      isLoading = false;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }
</script>

{#if isOpen}
  <div class="modal-backdrop" on:click={onClose} on:keydown={handleKeyDown} role="presentation">
    <div class="modal-card" on:click|stopPropagation role="dialog" aria-modal="true" aria-label="Inspect Media File" tabindex="-1">
      <div class="modal-header">
        <div class="modal-title">
          <span class="icon">🔍</span>
          <span>Inspect Media for Hidden Metadata Leaks</span>
        </div>
        <button type="button" class="close-btn" on:click={onClose}>✕</button>
      </div>

      <div class="modal-body">
        <div class="header-row">
          <p class="desc">
            Analyze binary EXIF, XMP, IPTC, and PDF dictionaries to detect GPS coordinates, serial numbers, and creator identities.
          </p>
          <div class="sample-actions">
            <button type="button" class="sample-btn" on:click={loadSampleJpeg}>Sample JPEG</button>
            <button type="button" class="sample-btn" on:click={loadSamplePdf}>Sample PDF</button>
          </div>
        </div>

        <div class="input-group">
          <label for="media-name">Target Media Filename</label>
          <input
            id="media-name"
            type="text"
            bind:value={filename}
            placeholder="e.g. Field_Evidence_Photo.jpg or Contract.pdf"
          />
        </div>

        <div class="input-group full">
          <label for="media-payload">Binary Content / Text Stream</label>
          <textarea
            id="media-payload"
            bind:value={payloadText}
            placeholder="Paste text or metadata stream here..."
            rows="6"
          ></textarea>
        </div>

        {#if errorMessage}
          <div class="error-box">{errorMessage}</div>
        {/if}
      </div>

      <div class="modal-footer">
        <button type="button" class="btn secondary" on:click={onClose}>Cancel</button>
        <button type="button" class="btn primary" on:click={startInspection} disabled={isLoading}>
          {isLoading ? 'Scanning Binary Headers...' : 'Run Forensic Inspection'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(4, 7, 14, 0.82);
    backdrop-filter: blur(8px);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 1000;
  }

  .modal-card {
    background: #0d121f;
    border: 1px solid #1f293d;
    border-radius: 12px;
    width: 680px;
    max-width: 90vw;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.7);
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid #1a2336;
    background: #090e18;
  }

  .modal-title {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 1.1rem;
    font-weight: 600;
    color: #f1f5f9;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #64748b;
    font-size: 1.2rem;
    cursor: pointer;
  }

  .close-btn:hover {
    color: #f1f5f9;
  }

  .modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .header-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }

  .desc {
    color: #94a3b8;
    font-size: 0.86rem;
    line-height: 1.4;
    margin: 0;
    flex: 1;
  }

  .sample-actions {
    display: flex;
    gap: 6px;
  }

  .sample-btn {
    background: #1e293b;
    border: 1px solid #334155;
    color: #38bdf8;
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 0.76rem;
    cursor: pointer;
    font-weight: 500;
  }

  .sample-btn:hover {
    background: #2563eb;
    color: #fff;
  }

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  label {
    color: #cbd5e1;
    font-size: 0.82rem;
    font-weight: 500;
  }

  input, textarea {
    background: #060a12;
    border: 1px solid #1e293b;
    border-radius: 6px;
    color: #e2e8f0;
    padding: 9px 12px;
    font-size: 0.88rem;
    outline: none;
  }

  textarea {
    font-family: monospace;
    font-size: 0.84rem;
    resize: vertical;
  }

  input:focus, textarea:focus {
    border-color: #3b82f6;
  }

  .error-box {
    padding: 10px 14px;
    border-radius: 6px;
    font-size: 0.85rem;
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.4);
    color: #f87171;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 14px 20px;
    border-top: 1px solid #1a2336;
    background: #090e18;
  }

  .btn {
    padding: 8px 18px;
    border-radius: 6px;
    font-size: 0.88rem;
    font-weight: 500;
    cursor: pointer;
    border: none;
  }

  .btn.secondary {
    background: #1e293b;
    color: #cbd5e1;
  }

  .btn.secondary:hover {
    background: #334155;
  }

  .btn.primary {
    background: #2563eb;
    color: #fff;
  }

  .btn.primary:hover:not(:disabled) {
    background: #1d4ed8;
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
