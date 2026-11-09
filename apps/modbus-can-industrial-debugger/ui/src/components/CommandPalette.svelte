<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let show = false;
  const dispatch = createEventDispatcher();
  let query = '';

  const commands = [
    { id: 'decode_frame', label: 'Dissect Raw Fieldbus Frame...', desc: 'Parse hex payload into Modbus RTU / CAN signals' },
    { id: 'sign_audit', label: 'Generate IEC 62443 Audit Manifest...', desc: 'Issue Ed25519-signed industrial security certificate' },
    { id: 'manage_license', label: 'Manage License & Enterprise Air-Gap...', desc: 'View offline activation status and apply license key' },
    { id: 'reload_sessions', label: 'Reload Trace Sessions', desc: 'Refresh local fieldbus captures and telemetry' },
  ];

  $: filtered = commands.filter(c => 
    c.label.toLowerCase().includes(query.toLowerCase()) || 
    c.desc.toLowerCase().includes(query.toLowerCase())
  );

  function execute(id: string) {
    dispatch('select', { id });
    close();
  }

  function close() {
    show = false;
    query = '';
    dispatch('close');
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if show}
  <div class="fixed inset-0 z-50 flex items-start justify-center pt-24 bg-black/60 backdrop-blur-sm" on:click={close}>
    <div class="w-full max-w-xl bg-slate-900 border border-slate-700 rounded-xl shadow-2xl overflow-hidden" on:click|stopPropagation>
      <div class="p-3 border-b border-slate-800 flex items-center gap-2">
        <span class="text-amber-400 font-mono text-sm px-2">⚡</span>
        <input
          type="text"
          bind:value={query}
          placeholder="Type an industrial command or action..."
          class="w-full bg-transparent text-slate-100 placeholder-slate-500 focus:outline-none text-sm font-sans"
          autofocus
        />
        <span class="text-xs text-slate-500 border border-slate-700 rounded px-1.5 py-0.5">ESC</span>
      </div>
      <div class="max-h-72 overflow-y-auto divide-y divide-slate-800/50">
        {#each filtered as cmd}
          <button
            class="w-full text-left px-4 py-3 hover:bg-slate-800/80 transition-colors flex flex-col gap-0.5 group"
            on:click={() => execute(cmd.id)}
          >
            <div class="text-sm font-medium text-slate-200 group-hover:text-amber-300">{cmd.label}</div>
            <div class="text-xs text-slate-400">{cmd.desc}</div>
          </button>
        {:else}
          <div class="p-6 text-center text-sm text-slate-500">No matching commands</div>
        {/each}
      </div>
    </div>
  </div>
{/if}
