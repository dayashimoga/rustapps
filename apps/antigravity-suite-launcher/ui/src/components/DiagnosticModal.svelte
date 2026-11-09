<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { AppMetadata, AppSelfTestResult } from '../lib/types';

  export let show = false;
  export let app: AppMetadata | null = null;
  export let result: AppSelfTestResult | null = null;
  export let running = false;

  const dispatch = createEventDispatcher();

  function close() {
    show = false;
    dispatch('close');
  }

  function reRun() {
    if (app) {
      dispatch('run', app.id);
    }
  }
</script>

{#if show && app}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm p-4" on:click={close}>
    <div class="w-full max-w-xl bg-slate-900 border border-slate-700 rounded-xl shadow-2xl p-6" on:click|stopPropagation>
      <div class="flex items-center justify-between pb-4 border-b border-slate-800">
        <div class="flex items-center gap-2">
          <span class="text-xl">{app.icon_glyph}</span>
          <div>
            <h2 class="text-base font-semibold text-slate-100">{app.name} Diagnostic Check</h2>
            <div class="text-[11px] font-mono text-slate-400">{app.binary_name} v{app.version}</div>
          </div>
        </div>
        <button class="text-slate-400 hover:text-slate-200" on:click={close}>✕</button>
      </div>

      <div class="py-4 space-y-4">
        {#if running}
          <div class="p-8 text-center text-amber-400 font-mono text-xs animate-pulse">
            Executing IPC loopback self-test checks against {app.binary_name}...
          </div>
        {:else if result}
          <div class="grid grid-cols-2 gap-3">
            <div class="p-3 bg-slate-800/60 rounded-lg border border-slate-700/60 text-center">
              <div class="text-xs font-mono text-slate-400 uppercase">Self-Test Status</div>
              <div class="text-lg font-bold {result.passed ? 'text-emerald-400' : 'text-red-400'} mt-1">
                {result.passed ? 'PASSED (100%)' : 'FAILED'}
              </div>
            </div>
            <div class="p-3 bg-slate-800/60 rounded-lg border border-slate-700/60 text-center">
              <div class="text-xs font-mono text-slate-400 uppercase">IPC Checks</div>
              <div class="text-lg font-bold text-slate-200 mt-1">
                {result.ipc_checks_passed} / {result.ipc_checks_total} OK
              </div>
            </div>
          </div>

          <div class="p-3 bg-slate-950 rounded-lg border border-slate-800 font-mono text-xs text-slate-300">
            {result.message}
          </div>

          <div class="p-3 bg-slate-800/40 border border-slate-700/50 rounded-lg text-xs text-slate-400 space-y-1">
            <div>• SLA Target Cold Startup: &lt; {app.cold_start_target_ms}ms</div>
            <div>• Base Idle RAM Budget: &lt; {app.ram_idle_budget_mb}MB</div>
            <div>• Air-Gap Telemetry Status: {app.air_gap_status}</div>
          </div>
        {/if}
      </div>

      <div class="flex justify-end gap-3 pt-3 border-t border-slate-800">
        <button
          class="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 rounded-lg text-xs font-medium transition-colors"
          on:click={reRun}
          disabled={running}
        >
          {running ? 'Testing...' : 'Re-run Diagnostic'}
        </button>
        <button class="px-4 py-2 bg-amber-600 hover:bg-amber-500 text-slate-950 font-semibold rounded-lg text-xs" on:click={close}>
          Done
        </button>
      </div>
    </div>
  </div>
{/if}
