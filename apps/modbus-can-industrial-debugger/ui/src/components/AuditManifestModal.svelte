<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { FieldbusAuditManifest } from '../lib/types';

  export let show = false;
  export let manifest: FieldbusAuditManifest | null = null;
  const dispatch = createEventDispatcher();
  let copied = false;

  function close() {
    show = false;
    copied = false;
    dispatch('close');
  }

  function copyJson() {
    if (!manifest) return;
    navigator.clipboard.writeText(JSON.stringify(manifest, null, 2));
    copied = true;
    setTimeout(() => { copied = false; }, 2000);
  }
</script>

{#if show && manifest}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm p-4" on:click={close}>
    <div class="w-full max-w-2xl bg-slate-900 border border-slate-700 rounded-xl shadow-2xl p-6" on:click|stopPropagation>
      <div class="flex items-center justify-between pb-4 border-b border-slate-800">
        <div class="flex items-center gap-2">
          <span class="text-emerald-400 font-mono text-base">🛡️</span>
          <h2 class="text-base font-semibold text-slate-100">IEC 62443 Fieldbus Compliance Certificate</h2>
        </div>
        <button class="text-slate-400 hover:text-slate-200" on:click={close}>✕</button>
      </div>

      <div class="py-4 space-y-4">
        <div class="grid grid-cols-3 gap-3">
          <div class="p-3 bg-slate-800/60 rounded-lg border border-slate-700/60 text-center">
            <div class="text-xs font-mono text-slate-400 uppercase">Compliance Status</div>
            <div class="text-xl font-bold text-emerald-400 mt-1">{manifest.iec62443_compliance_status}</div>
          </div>
          <div class="p-3 bg-slate-800/60 rounded-lg border border-slate-700/60 text-center">
            <div class="text-xs font-mono text-slate-400 uppercase">Frames Audited</div>
            <div class="text-xl font-bold text-slate-200 mt-1">{manifest.total_frames}</div>
          </div>
          <div class="p-3 bg-slate-800/60 rounded-lg border border-slate-700/60 text-center">
            <div class="text-xs font-mono text-slate-400 uppercase">Security Alerts</div>
            <div class="text-xl font-bold {manifest.security_alerts_count > 0 ? 'text-amber-400' : 'text-slate-400'} mt-1">
              {manifest.security_alerts_count}
            </div>
          </div>
        </div>

        <div class="space-y-2 text-xs font-mono">
          <div class="p-3 bg-slate-950 rounded-lg border border-slate-800 space-y-2">
            <div>
              <span class="text-slate-500 uppercase text-[10px]">Session / Scope:</span>
              <div class="text-slate-200 font-semibold">{manifest.session_name} ({manifest.session_id})</div>
            </div>
            <div>
              <span class="text-slate-500 uppercase text-[10px]">BLAKE3 Telemetry Fingerprint:</span>
              <div class="text-amber-300 break-all">{manifest.trace_blake3_hash}</div>
            </div>
            <div>
              <span class="text-slate-500 uppercase text-[10px]">Ed25519 Auditor Public Key:</span>
              <div class="text-blue-300 break-all">{manifest.auditor_public_key_hex}</div>
            </div>
            <div>
              <span class="text-slate-500 uppercase text-[10px]">Cryptographic Ed25519 Signature:</span>
              <div class="text-emerald-400 break-all">{manifest.ed25519_signature_hex}</div>
            </div>
          </div>
        </div>

        <div class="p-3 bg-slate-800/40 border border-slate-700/50 rounded-lg text-xs text-slate-400">
          🔒 Certified air-gapped cryptographic record. Validated against IEC 62443 industrial network security controls with zero network egress.
        </div>
      </div>

      <div class="flex justify-end gap-3 pt-3 border-t border-slate-800">
        <button
          class="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 rounded-lg text-xs font-medium transition-colors"
          on:click={copyJson}
        >
          {copied ? 'Copied Manifest JSON!' : 'Copy Certificate JSON'}
        </button>
        <button class="px-4 py-2 bg-amber-600 hover:bg-amber-500 text-slate-950 font-semibold rounded-lg text-xs" on:click={close}>
          Done
        </button>
      </div>
    </div>
  </div>
{/if}
