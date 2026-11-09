<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte';
  import type { CommandItem } from './types';

  export let open: boolean = false;
  export let commands: CommandItem[] = [];
  export let placeholder: string = 'Type a command or search actions...';

  const dispatch = createEventDispatcher<{
    close: void;
    select: CommandItem;
  }>();

  let query: string = '';
  let selectedIndex: number = 0;
  let inputEl: HTMLInputElement;

  $: filteredCommands = commands.filter((cmd) => {
    if (!query.trim()) return true;
    const q = query.toLowerCase();
    return (
      cmd.title.toLowerCase().includes(q) ||
      cmd.category.toLowerCase().includes(q) ||
      (cmd.description && cmd.description.toLowerCase().includes(q))
    );
  });

  $: if (filteredCommands.length > 0 && selectedIndex >= filteredCommands.length) {
    selectedIndex = 0;
  }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      open = !open;
      if (open) {
        query = '';
        selectedIndex = 0;
        setTimeout(() => inputEl?.focus(), 30);
      }
      return;
    }

    if (!open) return;

    if (e.key === 'Escape') {
      e.preventDefault();
      close();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (filteredCommands.length > 0) {
        selectedIndex = (selectedIndex + 1) % filteredCommands.length;
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (filteredCommands.length > 0) {
        selectedIndex = (selectedIndex - 1 + filteredCommands.length) % filteredCommands.length;
      }
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (filteredCommands[selectedIndex]) {
        executeCommand(filteredCommands[selectedIndex]);
      }
    }
  }

  function executeCommand(cmd: CommandItem) {
    close();
    dispatch('select', cmd);
    cmd.action();
  }

  function close() {
    open = false;
    dispatch('close');
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeydown);
  });

  onDestroy(() => {
    if (typeof window !== 'undefined') {
      window.removeEventListener('keydown', handleKeydown);
    }
  });

  $: if (open && inputEl) {
    setTimeout(() => inputEl.focus(), 30);
  }
</script>

{#if open}
  <div class="ag-palette-overlay" on:click|self={close} on:keydown|self={handleKeydown} role="presentation">
    <div class="ag-palette-modal ag-glass-panel">
      <div class="ag-palette-header">
        <svg class="ag-search-icon" viewBox="0 0 16 16" width="16" height="16" fill="currentColor">
          <path d="M11.742 10.344a6.5 6.5 0 1 0-1.397 1.398h-.001c.03.04.062.078.098.115l3.85 3.85a1 1 0 0 0 1.415-1.414l-3.85-3.85a1.007 1.007 0 0 0-.115-.1zM12 6.5a5.5 5.5 0 1 1-11 0 5.5 5.5 0 0 1 11 0z"/>
        </svg>
        <input
          bind:this={inputEl}
          bind:value={query}
          type="text"
          class="ag-palette-input"
          {placeholder}
          spellcheck="false"
          autocomplete="off"
        />
        <span class="ag-kbd">ESC</span>
      </div>

      <div class="ag-palette-list">
        {#if filteredCommands.length === 0}
          <div class="ag-palette-empty">No matching actions found</div>
        {:else}
          {#each filteredCommands as cmd, i}
            <button
              type="button"
              class="ag-palette-item"
              class:selected={i === selectedIndex}
              on:mouseenter={() => (selectedIndex = i)}
              on:click={() => executeCommand(cmd)}
            >
              <div class="ag-palette-item-main">
                <span class="ag-palette-item-cat">{cmd.category}</span>
                <span class="ag-palette-item-title">{cmd.title}</span>
                {#if cmd.description}
                  <span class="ag-palette-item-desc">{cmd.description}</span>
                {/if}
              </div>
              <div class="ag-palette-item-meta">
                {#if cmd.badge}
                  <span class="ag-badge ag-badge-pro">{cmd.badge}</span>
                {/if}
                {#if cmd.shortcut}
                  <span class="ag-kbd">{cmd.shortcut}</span>
                {/if}
              </div>
            </button>
          {/each}
        {/if}
      </div>

      <div class="ag-palette-footer">
        <span><kbd class="ag-kbd">↑</kbd> <kbd class="ag-kbd">↓</kbd> navigate</span>
        <span><kbd class="ag-kbd">↵</kbd> select</span>
        <span><kbd class="ag-kbd">esc</kbd> dismiss</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .ag-palette-overlay {
    position: fixed;
    inset: 0;
    z-index: 9999;
    background: oklch(0 0 0 / 0.55);
    backdrop-filter: blur(4px);
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 15vh;
  }

  .ag-palette-modal {
    width: 100%;
    max-width: 580px;
    border-radius: var(--ag-radius-lg);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .ag-palette-header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--ag-border);
  }

  .ag-search-icon {
    color: var(--ag-text-muted);
    flex-shrink: 0;
  }

  .ag-palette-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    font-size: 14px;
    font-family: var(--ag-font-sans);
    color: var(--ag-text-primary);
  }

  .ag-palette-input::placeholder {
    color: var(--ag-text-dim);
  }

  .ag-palette-list {
    max-height: 320px;
    overflow-y: auto;
    padding: 6px;
  }

  .ag-palette-empty {
    padding: 24px;
    text-align: center;
    color: var(--ag-text-muted);
    font-size: 13px;
  }

  .ag-palette-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 8px 10px;
    border-radius: var(--ag-radius-md);
    background: transparent;
    border: none;
    color: inherit;
    cursor: pointer;
    text-align: left;
    transition: background var(--ag-transition-fast);
  }

  .ag-palette-item.selected {
    background: var(--ag-bg-selected);
  }

  .ag-palette-item-main {
    display: flex;
    align-items: center;
    gap: 8px;
    overflow: hidden;
  }

  .ag-palette-item-cat {
    font-size: 10px;
    text-transform: uppercase;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--ag-text-muted);
    background: var(--ag-bg-panel);
    padding: 2px 6px;
    border-radius: var(--ag-radius-sm);
  }

  .ag-palette-item-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--ag-text-primary);
  }

  .ag-palette-item-desc {
    font-size: 11px;
    color: var(--ag-text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ag-palette-item-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .ag-palette-footer {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 8px 16px;
    background: var(--ag-bg-panel);
    border-top: 1px solid var(--ag-border);
    font-size: 11px;
    color: var(--ag-text-muted);
  }
</style>

