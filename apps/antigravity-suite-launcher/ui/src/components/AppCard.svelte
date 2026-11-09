<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { AppMetadata } from '../lib/types';

  export let app: AppMetadata;
  const dispatch = createEventDispatcher();

  function triggerDiagnostic() {
    dispatch('diagnostic', app);
  }

  function triggerLaunch() {
    dispatch('launch', app);
  }
</script>

<div class="bg-slate-900 border border-slate-800 hover:border-slate-700/80 rounded-xl p-4 flex flex-col justify-between transition-all hover:shadow-xl hover:shadow-black/40 group">
  <div>
    <!-- Top Row: Icon + Name + Category -->
    <div class="flex items-start justify-between gap-3 mb-2">
      <div class="flex items-center gap-2.5">
        <span class="text-2xl p-1.5 rounded-lg bg-slate-950 border border-slate-800">{app.icon_glyph}</span>
        <div>
          <h3 class="text-sm font-bold text-slate-100 group-hover:text-amber-300 transition-colors">{app.name}</h3>
          <span class="text-[10px] font-mono text-slate-500">v{app.version}</span>
        </div>
      </div>

      <span class="px-2 py-0.5 rounded text-[9px] font-mono font-semibold uppercase tracking-wider {app.category === 'enterprise_core' ? 'bg-blue-500/10 text-blue-300 border border-blue-500/30' : app.category === 'creator_media' ? 'bg-purple-500/10 text-purple-300 border border-purple-500/30' : 'bg-emerald-500/10 text-emerald-300 border border-emerald-500/30'}">
        {app.category === 'enterprise_core' ? 'DevOps' : app.category === 'creator_media' ? 'Creator' : 'Specialized'}
      </span>
    </div>

    <!-- Tagline -->
    <p class="text-xs text-slate-400 leading-relaxed mb-4 line-clamp-2">
      {app.tagline}
    </p>
  </div>

  <div>
    <!-- SLAs & Resource Budgets -->
    <div class="flex items-center justify-between text-[10px] font-mono text-slate-500 py-2 border-t border-slate-800/80 mb-3">
      <span>Startup: &lt;{app.cold_start_target_ms}ms</span>
      <span>RAM: &lt;{app.ram_idle_budget_mb}MB</span>
      <span class="text-emerald-400 font-semibold">100% Offline</span>
    </div>

    <!-- Actions -->
    <div class="flex items-center gap-2">
      <button
        class="flex-1 py-1.5 px-3 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded-lg text-xs font-mono font-medium transition-colors border border-slate-700/80"
        on:click={triggerDiagnostic}
      >
        Diagnostics
      </button>

      <button
        class="flex-1 py-1.5 px-3 bg-amber-600 hover:bg-amber-500 text-slate-950 rounded-lg text-xs font-mono font-bold transition-colors"
        on:click={triggerLaunch}
      >
        Launch ➔
      </button>
    </div>
  </div>
</div>
