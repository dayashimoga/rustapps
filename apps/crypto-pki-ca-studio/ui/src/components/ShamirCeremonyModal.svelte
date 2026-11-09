<script lang="ts">
  import { invokeIpc } from '../lib/ipc';
  import type { ShamirCeremony } from '../lib/types';

  export let isOpen = false;
  export let ceremony: ShamirCeremony | null = null;
  export let onClose: () => void = () => {};

  let selectedIndices: number[] = [];
  let reconstructionResult = '';
  let isReconstructing = false;
  let statusMessage = '';

  function toggleShard(idx: number) {
    if (selectedIndices.includes(idx)) {
      selectedIndices = selectedIndices.filter((i) => i !== idx);
    } else {
      selectedIndices = [...selectedIndices, idx];
    }
  }

  async function handleReconstruct() {
    if (!ceremony) return;

    if (selectedIndices.length < ceremony.threshold_m) {
      statusMessage = `Requires at least ${ceremony.threshold_m} shards (currently selected: ${selectedIndices.length})`;
      reconstructionResult = '';
      return;
    }

    isReconstructing = true;
    statusMessage = '';

    const selectedShards = ceremony.shards
      .filter((s) => selectedIndices.includes(s.shard_index))
      .map((s) => ({
        index: s.shard_index,
        hex: s.shard_hex,
      }));

    try {
      const res = await invokeIpc<{
        success: boolean;
        reconstructed_key_hex: string;
      }>('reconstruct_shamir_key', {
        shards: selectedShards,
        threshold_m: ceremony.threshold_m,
      });

      reconstructionResult = res.reconstructed_key_hex;
      statusMessage = `Quorum validated! Successfully reconstructed private key via ${selectedIndices.length}-of-${ceremony.total_n} Lagrange interpolation.`;
    } catch (e: any) {
      reconstructionResult = '';
      statusMessage = `Reconstruction failed: ${e.message || e}`;
    } finally {
      isReconstructing = false;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }
</script>

{#if isOpen && ceremony}
  <div class="modal-backdrop" on:click={onClose} on:keydown={handleKeyDown} role="presentation">
    <div class="modal-card" on:click|stopPropagation role="dialog" aria-modal="true" aria-label="Shamir Ceremony" tabindex="-1">
      <div class="modal-header">
        <div class="modal-title">
          <span class="icon">🛡️</span>
          <span>Shamir {ceremony.threshold_m}-of-{ceremony.total_n} Root Key Custodian Ceremony</span>
        </div>
        <button type="button" class="close-btn" on:click={onClose}>✕</button>
      </div>

      <div class="modal-body">
        <div class="ceremony-summary">
          <p>
            The private key for <strong>{ceremony.ca_common_name}</strong> is partitioned across
            <strong>{ceremony.total_n} distinct key custodians</strong> over Galois Field GF(2^8).
            Any <strong>{ceremony.threshold_m} custodians</strong> can assemble to authorize root operations.
          </p>
        </div>

        <!-- Shards List -->
        <div class="shards-table">
          {#each ceremony.shards as shard}
            {@const isSelected = selectedIndices.includes(shard.shard_index)}
            <div
              class="shard-card"
              class:selected={isSelected}
              role="button"
              tabindex="0"
              on:click={() => toggleShard(shard.shard_index)}
              on:keydown={(e) => {
                if (e.key === 'Enter') toggleShard(shard.shard_index);
              }}
            >
              <input
                type="checkbox"
                checked={isSelected}
                on:click|stopPropagation={() => toggleShard(shard.shard_index)}
              />
              <div class="shard-info">
                <div class="shard-custodian">{shard.custodian_label}</div>
                <div class="shard-hash">Commitment: {shard.commitment_hash.substring(0, 24)}...</div>
              </div>
              <span class="shard-idx-pill">Shard #{shard.shard_index}</span>
            </div>
          {/each}
        </div>

        <!-- Quorum Tracker -->
        <div class="quorum-box">
          <div class="quorum-header">
            <span>Selected Quorum: {selectedIndices.length} / {ceremony.threshold_m} Required</span>
            <span class="quorum-status" class:ready={selectedIndices.length >= ceremony.threshold_m}>
              {selectedIndices.length >= ceremony.threshold_m ? '✓ QUORUM MET' : 'AWAITING CUSTODIANS'}
            </span>
          </div>

          <button
            type="button"
            class="btn-reconstruct"
            disabled={selectedIndices.length < ceremony.threshold_m || isReconstructing}
            on:click={handleReconstruct}
          >
            {isReconstructing ? 'Reconstructing...' : 'Simulate Quorum Reconstruction'}
          </button>
        </div>

        {#if statusMessage}
          <div class="status-msg" class:success={!!reconstructionResult} class:error={!reconstructionResult}>
            {statusMessage}
          </div>
        {/if}

        {#if reconstructionResult}
          <div class="result-box">
            <span class="lbl">Reconstructed Private Key (Simulated In-Memory Session):</span>
            <code class="key-code">{reconstructionResult}</code>
          </div>
        {/if}
      </div>

      <div class="modal-footer">
        <button type="button" class="btn-done" on:click={onClose}>Close</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10000;
  }
  .modal-card {
    background: #181825;
    border: 1px solid #313244;
    border-radius: 12px;
    width: 640px;
    max-width: 90vw;
    box-shadow: 0 25px 50px rgba(0, 0, 0, 0.5);
    overflow: hidden;
    outline: none;
  }
  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    background: #11111b;
    border-bottom: 1px solid #313244;
  }
  .modal-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 15px;
    font-weight: 600;
    color: #cdd6f4;
  }
  .close-btn {
    background: none;
    border: none;
    color: #6c7086;
    font-size: 16px;
    cursor: pointer;
  }
  .close-btn:hover {
    color: #cdd6f4;
  }
  .modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-height: 75vh;
    overflow-y: auto;
  }
  .ceremony-summary p {
    margin: 0;
    font-size: 13px;
    color: #a6adc8;
    line-height: 1.5;
  }
  .shards-table {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .shard-card {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    background: #1e1e2e;
    border: 1px solid #313244;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .shard-card:hover {
    border-color: #89b4fa;
  }
  .shard-card.selected {
    border-color: #a6e3a1;
    background: rgba(166, 227, 161, 0.08);
  }
  .shard-info {
    flex: 1;
    overflow: hidden;
  }
  .shard-custodian {
    font-size: 12px;
    font-weight: 600;
    color: #cdd6f4;
  }
  .shard-hash {
    font-size: 10px;
    font-family: monospace;
    color: #6c7086;
  }
  .shard-idx-pill {
    background: #11111b;
    color: #f9e2af;
    font-size: 10px;
    font-family: monospace;
    padding: 2px 6px;
    border-radius: 4px;
  }
  .quorum-box {
    background: #11111b;
    border: 1px solid #313244;
    border-radius: 8px;
    padding: 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .quorum-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12px;
    font-weight: 600;
    color: #cdd6f4;
  }
  .quorum-status {
    color: #fab387;
    font-size: 11px;
    font-weight: 700;
  }
  .quorum-status.ready {
    color: #a6e3a1;
  }
  .btn-reconstruct {
    background: #89b4fa;
    color: #11111b;
    border: none;
    padding: 8px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 700;
    cursor: pointer;
  }
  .btn-reconstruct:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .status-msg {
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 12px;
  }
  .status-msg.success {
    background: rgba(166, 227, 161, 0.15);
    border: 1px solid #a6e3a1;
    color: #a6e3a1;
  }
  .status-msg.error {
    background: rgba(243, 139, 168, 0.15);
    border: 1px solid #f38ba8;
    color: #f38ba8;
  }
  .result-box {
    background: #11111b;
    border: 1px solid #a6e3a1;
    border-radius: 8px;
    padding: 10px 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .result-box .lbl {
    font-size: 10px;
    color: #a6e3a1;
    text-transform: uppercase;
    font-weight: 600;
  }
  .key-code {
    font-family: monospace;
    font-size: 11px;
    color: #94e2d5;
    word-break: break-all;
  }
  .modal-footer {
    padding: 12px 20px;
    background: #11111b;
    border-top: 1px solid #313244;
    display: flex;
    justify-content: flex-end;
  }
  .btn-done {
    background: #313244;
    color: #cdd6f4;
    border: none;
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 13px;
    cursor: pointer;
  }
</style>
