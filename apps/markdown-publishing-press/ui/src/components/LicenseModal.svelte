<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invokeIpc } from '../lib/ipc';
  import type { LicenseStatus } from '../lib/types';

  export let show = false;
  export let status: LicenseStatus | null = null;

  const dispatch = createEventDispatcher();
  let licenseKey = '';
  let activating = false;
  let errorMsg = '';
  let successMsg = '';

  function close() {
    show = false;
    licenseKey = '';
    errorMsg = '';
    successMsg = '';
    dispatch('close');
  }

  async function activate() {
    if (!licenseKey.trim()) return;
    activating = true;
    errorMsg = '';
    successMsg = '';

    try {
      const res = await invokeIpc<{ success: boolean; tier: string; licensee?: string }>('apply_license_key', {
        license_key: licenseKey.trim(),
      });
      successMsg = `Activated: ${res.tier.toUpperCase()} license for ${res.licensee || 'Academic Author'}`;
      dispatch('activated');
    } catch (err: any) {
      errorMsg = err.message || 'License activation failed';
    } finally {
      activating = false;
    }
  }
</script>

{#if show}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4" on:click={close}>
    <div class="w-full max-w-lg bg-slate-900 border border-slate-700 rounded-xl shadow-2xl p-6" on:click|stopPropagation>
      <div class="flex items-center justify-between pb-4 border-b border-slate-800">
        <div class="flex items-center gap-2">
          <div class="w-3 h-3 rounded-full bg-emerald-500 animate-pulse"></div>
          <h2 class="text-base font-semibold text-slate-100">Enterprise Air-Gap Licensing</h2>
        </div>
        <button class="text-slate-400 hover:text-slate-200" on:click={close}>✕</button>
      </div>

      <div class="py-4 space-y-4">
        <div class="p-4 bg-slate-800/60 rounded-lg border border-slate-700/60">
          <div class="text-xs font-mono text-slate-400 uppercase tracking-wider mb-2">Current Tier</div>
          <div class="flex items-center justify-between">
            <span class="text-lg font-bold text-amber-400 capitalize">{status?.tier || 'Community'}</span>
            <span class="text-xs px-2.5 py-1 rounded font-medium {status?.active ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/30' : 'bg-red-500/20 text-red-300 border border-red-500/30'}">
              {status?.active ? 'Active Offline' : 'Inactive'}
            </span>
          </div>
          {#if status?.licensee}
            <div class="mt-2 text-xs text-slate-300">Licensee: <span class="font-mono">{status.licensee}</span></div>
          {/if}
          <div class="mt-2 text-xs text-slate-400">100% Offline Autonomy • IEEE & ACM Typst Output • Zero Telemetry Egress</div>
        </div>

        <div>
          <label for="lic-key" class="block text-xs font-mono text-slate-400 uppercase mb-1">Apply Air-Gapped License Key</label>
          <input
            id="lic-key"
            type="text"
            bind:value={licenseKey}
            placeholder="XXXX-XXXX-XXXX-XXXX"
            class="w-full px-3 py-2 bg-slate-950 border border-slate-700 rounded-lg text-slate-100 text-sm font-mono focus:border-amber-500 focus:outline-none"
          />
        </div>

        {#if errorMsg}
          <div class="p-3 bg-red-950/40 border border-red-800/60 rounded-lg text-xs text-red-300">{errorMsg}</div>
        {/if}
        {#if successMsg}
          <div class="p-3 bg-emerald-950/40 border border-emerald-800/60 rounded-lg text-xs text-emerald-300">{successMsg}</div>
        {/if}
      </div>

      <div class="flex justify-end gap-3 pt-3 border-t border-slate-800">
        <button class="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded-lg text-xs font-medium" on:click={close}>
          Close
        </button>
        <button
          class="px-4 py-2 bg-amber-600 hover:bg-amber-500 text-slate-950 font-semibold rounded-lg text-xs transition-colors disabled:opacity-50"
          on:click={activate}
          disabled={activating || !licenseKey.trim()}
        >
          {activating ? 'Validating...' : 'Activate License'}
        </button>
      </div>
    </div>
  </div>
{/if}
