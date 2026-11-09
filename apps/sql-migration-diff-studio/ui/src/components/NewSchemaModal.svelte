<script lang="ts">
  import { invokeIpc } from '../lib/ipc';
  import type { DatabaseSchema, SqlDialect } from '../lib/types';

  export let isOpen = false;
  export let onClose: () => void = () => {};
  export let onSchemaParsed: (schema: DatabaseSchema) => void = () => {};

  let title = '';
  let dialect: SqlDialect = 'postgresql';
  let rawDdl = '';
  let isLoading = false;
  let errorMessage = '';

  const sampleDdl = `CREATE TABLE orders (
    id BIGSERIAL PRIMARY KEY,
    customer_id BIGINT NOT NULL,
    order_status VARCHAR(40) DEFAULT 'pending',
    total_amount NUMERIC(12, 2) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_orders_customer ON orders (customer_id);

CREATE TABLE order_items (
    id BIGSERIAL PRIMARY KEY,
    order_id BIGINT NOT NULL,
    sku VARCHAR(60) NOT NULL,
    quantity INT NOT NULL DEFAULT 1,
    unit_price NUMERIC(10, 2) NOT NULL
);

CREATE INDEX idx_items_order ON order_items (order_id);`;

  function loadSample() {
    title = 'E-Commerce Order Management Schema';
    dialect = 'postgresql';
    rawDdl = sampleDdl;
  }

  async function parseDdl() {
    if (!title.trim()) {
      errorMessage = 'Please provide a schema title';
      return;
    }
    if (!rawDdl.trim()) {
      errorMessage = 'Please paste SQL DDL statements';
      return;
    }

    isLoading = true;
    errorMessage = '';

    try {
      const schema = await invokeIpc<DatabaseSchema>('parse_schema', {
        title: title.trim(),
        dialect,
        raw_ddl: rawDdl.trim(),
      });

      onSchemaParsed(schema);
      onClose();
    } catch (e: any) {
      errorMessage = `Parsing failed: ${e.message || e}`;
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
    <div class="modal-card" on:click|stopPropagation role="dialog" aria-modal="true" aria-label="Import SQL Schema" tabindex="-1">
      <div class="modal-header">
        <div class="modal-title">
          <span class="icon">💾</span>
          <span>Import SQL DDL Schema</span>
        </div>
        <button type="button" class="close-btn" on:click={onClose}>✕</button>
      </div>

      <div class="modal-body">
        <div class="header-row">
          <p class="desc">
            Parse CREATE TABLE, constraints, and indexes into an air-gapped schema AST for visual inspection and differential migration planning.
          </p>
          <button type="button" class="sample-btn" on:click={loadSample}>Load Sample DDL</button>
        </div>

        <div class="grid-inputs">
          <div class="input-group">
            <label for="sch-title">Schema Name / Environment</label>
            <input
              id="sch-title"
              type="text"
              bind:value={title}
              placeholder="e.g. Analytics Warehouse v3.0"
            />
          </div>

          <div class="input-group">
            <label for="sch-dialect">SQL Dialect</label>
            <select id="sch-dialect" bind:value={dialect}>
              <option value="postgresql">PostgreSQL</option>
              <option value="mysql">MySQL</option>
              <option value="sqlite">SQLite</option>
              <option value="oracle">Oracle Database</option>
              <option value="mssql">Microsoft SQL Server</option>
            </select>
          </div>
        </div>

        <div class="input-group full">
          <label for="raw-ddl">SQL DDL Source Text</label>
          <textarea
            id="raw-ddl"
            bind:value={rawDdl}
            placeholder="CREATE TABLE ...; CREATE INDEX ...;"
            rows="10"
          ></textarea>
        </div>

        {#if errorMessage}
          <div class="error-box">{errorMessage}</div>
        {/if}
      </div>

      <div class="modal-footer">
        <button type="button" class="btn secondary" on:click={onClose}>Cancel</button>
        <button type="button" class="btn primary" on:click={parseDdl} disabled={isLoading}>
          {isLoading ? 'Parsing AST...' : 'Parse & Register Schema'}
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
