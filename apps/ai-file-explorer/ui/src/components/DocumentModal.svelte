<script lang="ts">
  import type { ExtractedDocument } from '../types';
  import { getLocalFileUrl } from '../lib/ipc';

  export let doc: ExtractedDocument | null = null;
  export let isOpen = false;
  export let onClose: () => void = () => {};
  export let onOpenInOs: (path: string) => void = () => {};

  let copySuccess = false;

  $: isImageFile = doc && doc.extension && ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'svg'].includes(doc.extension.toLowerCase());
  $: imagePreviewUrl = doc && isImageFile ? getLocalFileUrl(doc.path) : '';

  function copyToClipboard() {
    if (!doc) return;
    navigator.clipboard.writeText(doc.content_text);
    copySuccess = true;
    setTimeout(() => {
      copySuccess = false;
    }, 2000);
  }
</script>

{#if isOpen && doc}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
  <div class="modal-backdrop" on:click={onClose} role="dialog" aria-modal="true" aria-labelledby="doc-modal-title" tabindex="-1">
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div class="modal-dialog" on:click|stopPropagation role="document">
      <div class="modal-header">
        <div class="title-section">
          <h2 id="doc-modal-title">{doc.metadata.title || doc.file_name}</h2>
          <span class="path-sub">{doc.path}</span>
        </div>
        <button class="close-btn" on:click={onClose}>&times;</button>
      </div>

      <div class="metadata-bar">
        <div class="meta-item">
          <span class="label">Format:</span>
          <span class="val badge">{doc.metadata.format}</span>
        </div>
        {#if doc.metadata.author}
          <div class="meta-item">
            <span class="label">Author:</span>
            <span class="val">{doc.metadata.author}</span>
          </div>
        {/if}
        {#if doc.metadata.page_count}
          <div class="meta-item">
            <span class="label">Pages:</span>
            <span class="val">{doc.metadata.page_count}</span>
          </div>
        {/if}
        <div class="meta-item">
          <span class="label">Words:</span>
          <span class="val">{doc.metadata.word_count.toLocaleString()}</span>
        </div>
        {#if doc.language}
          <div class="meta-item">
            <span class="label">Language:</span>
            <span class="val uppercase">{doc.language}</span>
          </div>
        {/if}
      </div>

      <div class="modal-body">
        {#if isImageFile && imagePreviewUrl}
          <div class="image-preview-box">
            <img src={imagePreviewUrl} alt={doc.file_name} class="preview-img" />
          </div>
        {/if}
        <div class="content-toolbar">
          <span class="toolbar-label">Extracted Plaintext Content:</span>
          <div style="display: flex; gap: 8px;">
            <button class="copy-btn" on:click={() => doc && onOpenInOs(doc.path)}>
              Open in OS Default App
            </button>
            <button class="copy-btn" on:click={copyToClipboard}>
              {#if copySuccess}
                Copied!
              {:else}
                Copy Text
              {/if}
            </button>
          </div>
        </div>
        <pre class="content-display">{doc.content_text}</pre>
      </div>

      <div class="modal-footer">
        <button class="primary-btn" on:click={onClose}>Done</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-dialog {
    background: #0f172a;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 12px;
    width: 90%;
    max-width: 800px;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.6);
  }

  .modal-header {
    padding: 16px 20px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
  }

  .title-section h2 {
    font-size: 1.15rem;
    font-weight: 600;
    color: #f8fafc;
    margin: 0;
  }

  .path-sub {
    font-size: 0.75rem;
    color: #64748b;
    font-family: monospace;
    display: block;
    margin-top: 4px;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #94a3b8;
    font-size: 1.5rem;
    cursor: pointer;
    line-height: 1;
  }

  .close-btn:hover {
    color: #ffffff;
  }

  .metadata-bar {
    display: flex;
    gap: 16px;
    padding: 10px 20px;
    background: rgba(0, 0, 0, 0.25);
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    flex-wrap: wrap;
  }

  .meta-item {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.78rem;
  }

  .meta-item .label {
    color: #64748b;
  }

  .meta-item .val {
    color: #e2e8f0;
    font-weight: 500;
  }

  .meta-item .val.badge {
    background: rgba(14, 165, 233, 0.15);
    color: #38bdf8;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .meta-item .val.uppercase {
    text-transform: uppercase;
  }

  .modal-body {
    padding: 16px 20px;
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .content-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .toolbar-label {
    font-size: 0.75rem;
    color: #94a3b8;
    font-weight: 500;
    text-transform: uppercase;
  }

  .copy-btn {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #cbd5e1;
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .copy-btn:hover {
    background: rgba(14, 165, 233, 0.15);
    border-color: rgba(14, 165, 233, 0.4);
    color: #38bdf8;
  }

  .content-display {
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 6px;
    padding: 14px;
    font-family: 'Fira Code', monospace;
    font-size: 0.82rem;
    color: #e2e8f0;
    line-height: 1.5;
    white-space: pre-wrap;
    max-height: 400px;
    overflow-y: auto;
    margin: 0;
  }

  .modal-footer {
    padding: 12px 20px;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    display: flex;
    justify-content: flex-end;
  }

  .primary-btn {
    background: #0ea5e9;
    color: #ffffff;
    border: none;
    padding: 8px 18px;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
  }

  .primary-btn:hover {
    background: #0284c7;
  }

  .image-preview-box {
    display: flex;
    justify-content: center;
    align-items: center;
    background: rgba(0, 0, 0, 0.45);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    padding: 12px;
    max-height: 260px;
    overflow: hidden;
  }

  .preview-img {
    max-width: 100%;
    max-height: 236px;
    object-fit: contain;
    border-radius: 4px;
  }
</style>
