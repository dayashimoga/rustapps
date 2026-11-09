<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { IndustrialBusFrame, IndustrialTraceSession } from '../lib/types';

  export let session: IndustrialTraceSession | null = null;
  export let selectedFrame: IndustrialBusFrame | null = null;

  const dispatch = createEventDispatcher();
  let filterType = 'all';

  $: frames = session?.frames || [];
  $: filteredFrames = frames.filter(f => {
    if (filterType === 'modbus') return f.protocol.startsWith('modbus');
    if (filterType === 'can') return f.protocol.startsWith('can');
    if (filterType === 'alerts') return !!f.security_alert;
    if (filterType === 'errors') return (f.modbus && !f.modbus.crc_valid) || (f.modbus && f.modbus.is_exception);
    return true;
  });

  function select(frame: IndustrialBusFrame) {
    dispatch('select', frame);
  }
</script>

<div class="flex flex-col h-full bg-slate-900 border-r border-slate-800">
  <div class="p-3 border-b border-slate-800 flex items-center justify-between bg-slate-950/40">
    <div class="flex items-center gap-2">
      <span class="text-xs font-mono uppercase text-slate-400 font-bold">Packet Waterfall</span>
      <span class="text-[11px] font-mono px-2 py-0.5 rounded-full bg-slate-800 text-slate-300">
        {filteredFrames.length} / {frames.length} frames
      </span>
    </div>
    <div class="flex gap-1 text-[11px] font-mono">
      <button
        class="px-2 py-0.5 rounded {filterType === 'all' ? 'bg-amber-500/20 text-amber-300 border border-amber-500/40' : 'text-slate-400 hover:text-slate-200'}"
        on:click={() => filterType = 'all'}
      >
        All
      </button>
      <button
        class="px-2 py-0.5 rounded {filterType === 'modbus' ? 'bg-amber-500/20 text-amber-300 border border-amber-500/40' : 'text-slate-400 hover:text-slate-200'}"
        on:click={() => filterType = 'modbus'}
      >
        Modbus
      </button>
      <button
        class="px-2 py-0.5 rounded {filterType === 'can' ? 'bg-amber-500/20 text-amber-300 border border-amber-500/40' : 'text-slate-400 hover:text-slate-200'}"
        on:click={() => filterType = 'can'}
      >
        CAN
      </button>
      <button
        class="px-2 py-0.5 rounded {filterType === 'alerts' ? 'bg-red-500/20 text-red-300 border border-red-500/40' : 'text-slate-400 hover:text-slate-200'}"
        on:click={() => filterType = 'alerts'}
      >
        Alerts
      </button>
    </div>
  </div>

  <div class="flex-1 overflow-y-auto divide-y divide-slate-800/60 font-mono text-xs">
    {#each filteredFrames as frame}
      <!-- svelte-ignore a11y-click-events-have-key-events -->
      <!-- svelte-ignore a11y-no-static-element-interactions -->
      <div
        class="p-2.5 cursor-pointer transition-colors hover:bg-slate-800/60 flex items-center justify-between gap-2 {selectedFrame?.frame_index === frame.frame_index ? 'bg-amber-950/30 border-l-2 border-amber-400' : ''}"
        on:click={() => select(frame)}
      >
        <div class="flex items-center gap-2 min-w-0">
          <span class="text-slate-500 w-8 text-right">#{frame.frame_index}</span>
          <span class="text-slate-400 text-[10px] w-14">{frame.timestamp_micros}µs</span>
          
          <span class="px-1.5 py-0.2 rounded text-[10px] font-bold {frame.protocol.startsWith('modbus') ? 'bg-blue-500/20 text-blue-300 border border-blue-500/30' : 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/30'}">
            {frame.protocol.toUpperCase().replace('_', ' ')}
          </span>

          <span class="text-[10px] text-slate-400">{frame.direction}</span>

          {#if frame.modbus}
            <span class="truncate text-slate-200">
              Unit {frame.modbus.unit_id} : {frame.modbus.function_name}
            </span>
          {:else if frame.can}
            <span class="truncate text-slate-200">
              ID {frame.can.can_id_hex} ({frame.can.dlc}B)
            </span>
          {/if}
        </div>

        <div class="flex items-center gap-1.5 shrink-0">
          {#if frame.modbus && !frame.modbus.crc_valid}
            <span class="px-1.5 py-0.2 rounded text-[10px] bg-red-500/20 text-red-300 border border-red-500/30 font-bold">
              CRC ERR
            </span>
          {/if}

          {#if frame.security_alert}
            <span class="px-1.5 py-0.2 rounded text-[10px] bg-red-900/40 text-red-300 border border-red-700/50 font-bold animate-pulse">
              ⚠️ {frame.security_alert.severity.toUpperCase()}
            </span>
          {/if}
        </div>
      </div>
    {:else}
      <div class="p-8 text-center text-slate-500 text-xs">
        No fieldbus frames found matching criteria.
      </div>
    {/each}
  </div>
</div>
