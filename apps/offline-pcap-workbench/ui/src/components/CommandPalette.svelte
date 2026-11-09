<script lang="ts">
  import { onMount } from 'svelte';

  export let isOpen = false;
  export let onClose: () => void = () => {};
  export let onSelectCommand: (cmdId: string) => void = () => {};

  interface CommandItem {
    id: string;
    label: string;
    category: string;
    shortcut?: string;
  }

  const commands: CommandItem[] = [
    { id: 'filter_all', label: 'Reset Filter (Show All Packets)', category: 'Filter', shortcut: 'Esc' },
    { id: 'filter_tls', label: 'Filter: TLS 1.3 / HTTPS Only', category: 'Filter' },
    { id: 'filter_http', label: 'Filter: Plaintext HTTP Traffic', category: 'Filter' },
    { id: 'filter_dns', label: 'Filter: DNS Name Lookups', category: 'Filter' },
    { id: 'view_packets', label: 'Switch View: Packet List & Hex Inspector', category: 'View' },
    { id: 'view_waterfall', label: 'Switch View: TCP Stream Waterfall', category: 'View' },
    { id: 'open_forensics', label: 'View Forensic Security & Integrity Audit', category: 'Forensics' },
    { id: 'open_license', label: 'Manage Air-Gapped License Key', category: 'License' },
  ];

  let searchQuery = '';
  let selectedIndex = 0;

  $: filteredCommands = commands.filter((c) =>
    c.label.toLowerCase().includes(searchQuery.toLowerCase()) ||
    c.category.toLowerCase().includes(searchQuery.toLowerCase())
  );

  function execute(cmdId: string) {
    onSelectCommand(cmdId);
    onClose();
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (!isOpen) return;

    if (e.key === 'Escape') {
      onClose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = (selectedIndex + 1) % (filteredCommands.length || 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = (selectedIndex - 1 + filteredCommands.length) % (filteredCommands.length || 1);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (filteredCommands[selectedIndex]) {
        execute(filteredCommands[selectedIndex].id);
      }
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  });
</script>

{#if isOpen}
  <div class="cmd-backdrop" on:click={onClose} on:keydown={handleKeyDown} role="presentation">
    <div class="cmd-modal" on:click|stopPropagation role="dialog" aria-modal="true" aria-label="Command Palette" tabindex="-1">
      <div class="cmd-search-bar">
        <span class="cmd-icon">🔍</span>
        <input
          type="text"
          placeholder="Type a command or filter (e.g. TLS, DNS, Waterfall)..."
          bind:value={searchQuery}
          autofocus
        />
        <kbd class="kbd-esc">ESC</kbd>
      </div>

      <div class="cmd-list">
        {#each filteredCommands as cmd, i}
          <button
            type="button"
            class="cmd-item"
            class:selected={i === selectedIndex}
            on:click={() => execute(cmd.id)}
            on:mouseenter={() => (selectedIndex = i)}
          >
            <div class="cmd-left">
              <span class="cmd-category-tag">{cmd.category}</span>
              <span class="cmd-title">{cmd.label}</span>
            </div>
            {#if cmd.shortcut}
              <kbd class="cmd-shortcut">{cmd.shortcut}</kbd>
            {/if}
          </button>
        {:else}
          <div class="cmd-empty">No matching commands found</div>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .cmd-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 10vh;
    z-index: 9999;
  }
  .cmd-modal {
    background: #1e1e2e;
    border: 1px solid #313244;
    border-radius: 12px;
    width: 600px;
    max-width: 90vw;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.5);
    overflow: hidden;
    outline: none;
  }
  .cmd-search-bar {
    display: flex;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid #313244;
    gap: 12px;
    background: #181825;
  }
  .cmd-icon {
    font-size: 16px;
    opacity: 0.6;
  }
  .cmd-search-bar input {
    flex: 1;
    background: transparent;
    border: none;
    color: #cdd6f4;
    font-size: 15px;
    outline: none;
  }
  .kbd-esc {
    background: #313244;
    color: #a6adc8;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 11px;
    font-family: monospace;
  }
  .cmd-list {
    max-height: 340px;
    overflow-y: auto;
    padding: 8px;
  }
  .cmd-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 10px 14px;
    border-radius: 8px;
    background: transparent;
    border: none;
    color: #cdd6f4;
    cursor: pointer;
    font-size: 13px;
    text-align: left;
    transition: background 0.15s ease;
  }
  .cmd-item.selected {
    background: #313244;
    color: #89b4fa;
  }
  .cmd-left {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .cmd-category-tag {
    font-size: 10px;
    text-transform: uppercase;
    background: #11111b;
    color: #6c7086;
    padding: 2px 6px;
    border-radius: 4px;
    font-weight: 600;
  }
  .cmd-shortcut {
    font-size: 11px;
    background: #11111b;
    color: #89b4fa;
    padding: 2px 6px;
    border-radius: 4px;
    font-family: monospace;
  }
  .cmd-empty {
    padding: 24px;
    text-align: center;
    color: #6c7086;
    font-size: 13px;
  }
</style>
