<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { CopyrightCertificate } from '../lib/types';

  export let show = false;
  export let certificate: CopyrightCertificate | null = null;
  const dispatch = createEventDispatcher();
  let copied = false;

  function close() {
    show = false;
    copied = false;
    dispatch('close');
  }

  function copyJson() {
    if (!certificate) return;
    navigator.clipboard.writeText(JSON.stringify(certificate, null, 2));
    copied = true;
    setTimeout(() => { copied = false; }, 2000);
  }
</script>

{#if show && certificate}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm p-4" on:click={close}>
    <div class="w-full max-w-2xl bg-slate-900 border border-slate-700 rounded-xl shadow-2xl p-6" on:click|stopPropagation>
      <div class="flex items-center justify-between pb-4 border-b border-slate-800">
        <div class="flex items-center gap-2">
          <span class="text-emerald-400 font-mono text-base">🛡️</span>
          <h2 class="text-base font-semibold text-slate-100">Ed25519 Copyright & IP Certificate</h2>
        </div>
        <button class="text-slate-400 hover:text-slate-200" on:click={close}>✕</button>
      </div>

      <div class="py-4 space-y-4">
        <div class="p-4 bg-slate-950 rounded-lg border border-slate-800 space-y-2 text-xs font-mono">
          <div>
            <span class="text-slate-500 uppercase text-[10px]">Manuscript Title:</span>
            <div class="text-slate-200 font-semibold">{certificate.title}</div>
          </div>
          <div>
            <span class="text-slate-500 uppercase text-[10px]">Primary Author / Entity:</span>
            <div class="text-slate-300">{certificate.primary_author}</div>
          </div>
          <div>
            <span class="text-slate-500 uppercase text-[10px]">Document BLAKE3 Fingerprint:</span>
            <div class="text-amber-300 break-all">{certificate.document_blake3_hash}</div>
          </div>
          <div>
            <span class="text-slate-500 uppercase text-[10px]">Author Ed25519 Public Key:</span>
            <div class="text-blue-300 break-all">{certificate.author_public_key_hex}</div>
          </div>
          <div>
            <span class="text-slate-500 uppercase text-[10px]">Cryptographic Ed25519 Signature:</span>
            <div class="text-emerald-400 break-all">{certificate.ed25519_signature_hex}</div>
          </div>
        </div>

        <div class="p-3 bg-slate-800/40 border border-slate-700/50 rounded-lg text-xs text-slate-400">
          🔒 Cryptographically certified intellectual property token. Proves prior invention, authorship attribution, and immutable document state at timestamp with zero cloud transmission.
        </div>
      </div>

      <div class="flex justify-end gap-3 pt-3 border-t border-slate-800">
        <button
          class="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 rounded-lg text-xs font-medium transition-colors"
          on:click={copyJson}
        >
          {copied ? 'Copied Certificate JSON!' : 'Copy Certificate JSON'}
        </button>
        <button class="px-4 py-2 bg-amber-600 hover:bg-amber-500 text-slate-950 font-semibold rounded-lg text-xs" on:click={close}>
          Done
        </button>
      </div>
    </div>
  </div>
{/if}
