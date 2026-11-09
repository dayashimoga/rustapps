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
    { id: 'upload_media', label: 'Inspect New Media File / Document', category: 'Inspection', shortcut: 'Ctrl+N' },
    { id: 'sanitize_active', label: 'Purge All Metadata & Sanitize Active File', category: 'Sanitization', shortcut: 'Ctrl+P' },
    { id: 'sign_certificate', label: 'Issue Ed25519 Zero-Trace Privacy Certificate', category: 'Certification', shortcut: 'Ctrl+S' },
    { id: 'filter_critical', label: 'Filter Critical Leaks (GPS, Serials)', category: 'Filter' },
    { id: 'filter_all', label: 'Show All Detected Metadata Tags', category: 'Filter' },
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
  <div class="palette-backdrop" on:click={onClose} on:keydown={handleKeyDown} role="presentation">
    <div class="palette-modal" on:click|stopPropagation role="dialog" aria-modal="true" aria-label="Command Palette" tabindex="-1">
      <div class="search-bar">
        <span class="search-icon">🔍</span>
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Type an action or filter (e.g. Inspect, Sanitize, GPS, Certificate)..."
        />
        <kbd class="esc-badge">ESC</kbd>
      </div>

      <div class="results-list">
        {#each filteredCommands as cmd, i}
          <div
            class="command-row"
            class:selected={i === selectedIndex}
            on:click={() => execute(cmd.id)}
            on:keydown={(e) => e.key === 'Enter' && execute(cmd.id)}
            on:mouseenter={() => (selectedIndex = i)}
            role="button"
            tabindex="0"
          >
            <div class="command-info">
              <span class="category-badge">{cmd.category}</span>
              <span class="cmd-label">{cmd.label}</span>
            </div>
            {#if cmd.shortcut}
              <kbd class="shortcut">{cmd.shortcut}</kbd>
            {/if}
          </div>
        {/each}
        {#if filteredCommands.length === 0}
          <div class="empty-state">No matching commands found.</div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .palette-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(4, 7, 14, 0.78);
    backdrop-filter: blur(8px);
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 15vh;
    z-index: 1000;
  }

  .palette-modal {
    background: #0d121f;
    border: 1px solid #1f293d;
    border-radius: 12px;
    width: 620px;
    max-width: 90vw;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.7);
    overflow: hidden;
  }

  .search-bar {
    display: flex;
    align-items: center;
    padding: 14px 18px;
    border-bottom: 1px solid #1a2336;
    background: #090e18;
  }

  .search-icon {
    font-size: 1.1rem;
    margin-right: 12px;
    opacity: 0.7;
  }

  input {
    flex: 1;
    background: transparent;
    border: none;
    color: #e2e8f0;
    font-size: 1.05rem;
    outline: none;
  }

  .esc-badge {
    background: #192336;
    color: #94a3b8;
    border: 1px solid #2a3750;
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 0.75rem;
    font-family: monospace;
  }

  .results-list {
    max-height: 340px;
    overflow-y: auto;
    padding: 8px;
  }

  .command-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-radius: 8px;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .command-row.selected {
    background: #1e293b;
  }

  .command-info {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .category-badge {
    background: #132338;
    color: #38bdf8;
    border: 1px solid #1e3a8a;
    font-size: 0.7rem;
    padding: 2px 8px;
    border-radius: 4px;
    text-transform: uppercase;
    font-weight: 600;
    letter-spacing: 0.05em;
  }

  .cmd-label {
    color: #f1f5f9;
    font-size: 0.95rem;
  }

  .shortcut {
    background: #111827;
    border: 1px solid #374151;
    color: #9ca3af;
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 0.75rem;
    font-family: monospace;
  }

  .empty-state {
    padding: 24px;
    text-align: center;
    color: #64748b;
    font-size: 0.9rem;
  }
</style>
