<script lang="ts">
  import { invokeIpc } from '../lib/ipc';
  import type { ContractDocument, ContractType } from '../lib/types';

  export let isOpen = false;
  export let onClose: () => void = () => {};
  export let onAuditCompleted: (doc: ContractDocument) => void = () => {};

  let title = '';
  let contractType: ContractType = 'saas_master_services_agreement';
  let rawText = '';
  let isLoading = false;
  let errorMessage = '';

  const sampleAgreement = `Section 1.0 Services and Scope
Vendor provides cloud-hosted analytics. Customer subscribes on an annual enterprise term.

Section 2.0 Indemnification
Customer agrees to defend, indemnify, and hold harmless Vendor against all third-party intellectual property claims and lawsuits.

Section 3.0 Limitation of Liability
VENDOR'S TOTAL AGGREGATE LIABILITY SHALL BE CAPPED AT $50.00. CUSTOMER'S LIABILITY IS UNLIMITED UNDER ANY THEORY OF LAW.

Section 4.0 Data Ownership
Vendor shall exclusively own all derivative data models, intelligence insights, and customizations produced from Customer inputs.`;

  function loadSample() {
    title = 'Cloud Provider Enterprise Terms of Service';
    contractType = 'saas_master_services_agreement';
    rawText = sampleAgreement;
  }

  async function startAudit() {
    if (!title.trim()) {
      errorMessage = 'Please provide a contract title';
      return;
    }
    if (!rawText.trim()) {
      errorMessage = 'Please paste contract terms text to audit';
      return;
    }

    isLoading = true;
    errorMessage = '';

    try {
      const doc = await invokeIpc<ContractDocument>('audit_contract_text', {
        title: title.trim(),
        contract_type: contractType,
        raw_text: rawText.trim(),
      });

      onAuditCompleted(doc);
      onClose();
    } catch (e: any) {
      errorMessage = `Audit failed: ${e.message || e}`;
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
    <div class="modal-card" on:click|stopPropagation role="dialog" aria-modal="true" aria-label="New Contract Audit" tabindex="-1">
      <div class="modal-header">
        <div class="modal-title">
          <span class="icon">📜</span>
          <span>Air-Gapped Contract Audit</span>
        </div>
        <button type="button" class="close-btn" on:click={onClose}>✕</button>
      </div>

      <div class="modal-body">
        <div class="header-row">
          <p class="desc">
            Parse clauses, evaluate liability exposure against institutional legal playbooks, and generate redlines completely offline.
          </p>
          <button type="button" class="sample-btn" on:click={loadSample}>Load Aggressive Sample</button>
        </div>

        <div class="grid-inputs">
          <div class="input-group">
            <label for="doc-title">Document Title</label>
            <input
              id="doc-title"
              type="text"
              bind:value={title}
              placeholder="e.g. Master Services Agreement v2.1"
            />
          </div>

          <div class="input-group">
            <label for="doc-type">Contract Type</label>
            <select id="doc-type" bind:value={contractType}>
              <option value="saas_master_services_agreement">SaaS Master Services Agreement (MSA)</option>
              <option value="non_disclosure_agreement">Non-Disclosure Agreement (NDA)</option>
              <option value="data_processing_agreement">Data Processing Agreement (DPA)</option>
              <option value="vendor_procurement">Vendor Procurement & Licensing</option>
              <option value="employment_agreement">Employment / Contractor</option>
              <option value="other">General Commercial Agreement</option>
            </select>
          </div>
        </div>

        <div class="input-group full">
          <label for="raw-contract">Contract Text (Markdown / Sections / Clauses)</label>
          <textarea
            id="raw-contract"
            bind:value={rawText}
            placeholder="Paste raw contract terms, sections (e.g. Section 1.0, Article II), or clauses here..."
            rows="10"
          ></textarea>
        </div>

        {#if errorMessage}
          <div class="error-box">{errorMessage}</div>
        {/if}
      </div>

      <div class="modal-footer">
        <button type="button" class="btn secondary" on:click={onClose}>Cancel</button>
        <button type="button" class="btn primary" on:click={startAudit} disabled={isLoading}>
          {isLoading ? 'Running RAG Analysis...' : 'Start Air-Gapped Audit'}
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
    width: 760px;
    max-width: 92vw;
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
  }

  .desc {
    color: #94a3b8;
    font-size: 0.88rem;
    line-height: 1.4;
    margin: 0;
    flex: 1;
  }

  .sample-btn {
    background: #1e293b;
    border: 1px solid #334155;
    color: #38bdf8;
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 0.78rem;
    cursor: pointer;
    font-weight: 500;
  }

  .sample-btn:hover {
    background: #2563eb;
    color: #fff;
  }

  .grid-inputs {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
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

  input, select, textarea {
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

  input:focus, select:focus, textarea:focus {
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
