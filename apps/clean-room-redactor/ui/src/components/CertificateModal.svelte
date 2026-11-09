<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let isOpen: boolean = false;
  export let certificate: any = null;
  export let certSvg: string = '';

  const dispatch = createEventDispatcher();

  function handleClose() {
    dispatch('close');
  }

  function handleCopyJson() {
    if (certificate) {
      navigator.clipboard.writeText(JSON.stringify(certificate, null, 2));
    }
  }
</script>

{#if isOpen && certificate}
  <div
    class="modal-backdrop"
    on:click={handleClose}
    on:keydown={(e) => e.key === 'Escape' && handleClose()}
    role="presentation"
  >
    <div
      class="modal-card"
      on:click|stopPropagation
      role="dialog"
      aria-modal="true"
      aria-label="Sanitization Certificate"
    >
      <div class="modal-header">
        <div class="badge-title">
          <span class="shield-icon">🛡️</span>
          <h3>CRYPTOGRAPHIC SANITIZATION CERTIFICATE</h3>
        </div>
        <button class="close-x" on:click={handleClose}>✕</button>
      </div>

      <div class="modal-body">
        {#if certSvg}
          <div class="svg-container">
            {@html certSvg}
          </div>
        {/if}

        <div class="cert-grid">
          <div class="grid-item">
            <span class="label">Certificate ID:</span>
            <span class="val mono">{certificate.certificate_id}</span>
          </div>
          <div class="grid-item">
            <span class="label">Status:</span>
            <span class="val status-ok">✓ {certificate.verification_status}</span>
          </div>
          <div class="grid-item">
            <span class="label">Entities Burned:</span>
            <span class="val">{certificate.total_redactions_applied} PII/PHI instances</span>
          </div>
          <div class="grid-item">
            <span class="label">Bates Range:</span>
            <span class="val mono gold">{certificate.bates_range || 'None'}</span>
          </div>
          <div class="grid-item full">
            <span class="label">Original BLAKE3:</span>
            <span class="val mono small">{certificate.original_blake3}</span>
          </div>
          <div class="grid-item full">
            <span class="label">Sanitized BLAKE3:</span>
            <span class="val mono small green">{certificate.sanitized_blake3}</span>
          </div>
        </div>
      </div>

      <div class="modal-footer">
        <button class="btn-copy" on:click={handleCopyJson}>Copy Certificate JSON</button>
        <button class="btn-done" on:click={handleClose}>Done</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(4, 7, 12, 0.8);
    backdrop-filter: blur(10px);
    z-index: 10002;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .modal-card {
    width: 680px;
    max-width: 92vw;
    background: #0d121a;
    border: 1px solid #10b98166;
    border-radius: 12px;
    box-shadow: 0 24px 48px rgba(0, 0, 0, 0.8), 0 0 30px rgba(16, 185, 129, 0.15);
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 20px;
    background: #111823;
    border-bottom: 1px solid #1e293b;
  }

  .badge-title {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .shield-icon {
    font-size: 1.1rem;
  }

  .modal-header h3 {
    margin: 0;
    font-size: 0.88rem;
    font-family: monospace;
    color: #34d399;
    letter-spacing: 0.5px;
  }

  .close-x {
    background: none;
    border: none;
    color: #64748b;
    font-size: 1rem;
    cursor: pointer;
  }

  .modal-body {
    padding: 18px 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-height: 75vh;
    overflow-y: auto;
  }

  .svg-container {
    width: 100%;
    border-radius: 8px;
    overflow: hidden;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
  }

  .cert-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    background: #121924;
    border: 1px solid #1f2a3c;
    border-radius: 8px;
    padding: 14px;
    font-size: 0.78rem;
  }

  .grid-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .grid-item.full {
    grid-column: span 2;
  }

  .label {
    color: #64748b;
    font-family: monospace;
    font-size: 0.7rem;
  }

  .val {
    color: #f1f5f9;
  }

  .mono {
    font-family: monospace;
  }

  .small {
    font-size: 0.68rem;
    word-break: break-all;
  }

  .green {
    color: #34d399;
  }

  .gold {
    color: #fbbf24;
  }

  .status-ok {
    color: #10b981;
    font-weight: 700;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 12px 20px;
    background: #0a0e16;
    border-top: 1px solid #161f2e;
  }

  .btn-copy {
    background: #182232;
    border: 1px solid #28374d;
    color: #cbd5e1;
    padding: 6px 14px;
    border-radius: 4px;
    font-size: 0.8rem;
    cursor: pointer;
  }

  .btn-done {
    background: #10b981;
    border: 1px solid #34d399;
    color: #042f1a;
    font-weight: 700;
    padding: 6px 18px;
    border-radius: 4px;
    font-size: 0.8rem;
    cursor: pointer;
  }
</style>
