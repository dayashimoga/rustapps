<script lang="ts">
  import { onMount, tick } from 'svelte';

  export interface CommandAction {
    id: string;
    title: string;
    subtitle?: string;
    category: 'Navigation' | 'Actions' | 'Security' | 'System';
    icon: string;
    shortcut?: string;
    run: () => void;
  }

  export let isOpen: boolean = false;
  export let actions: CommandAction[] = [];
  export let onClose: () => void = () => {};

  let query: string = '';
  let selectedIndex: number = 0;
  let inputEl: HTMLInputElement;

  $: filteredActions = actions.filter((a) => {
    if (!query.trim()) return true;
    const q = query.toLowerCase();
    return (
      a.title.toLowerCase().includes(q) ||
      (a.subtitle && a.subtitle.toLowerCase().includes(q)) ||
      a.category.toLowerCase().includes(q)
    );
  });

  $: if (filteredActions.length > 0 && selectedIndex >= filteredActions.length) {
    selectedIndex = 0;
  }

  $: if (isOpen) {
    query = '';
    selectedIndex = 0;
    tick().then(() => {
      inputEl?.focus();
    });
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = (selectedIndex + 1) % Math.max(1, filteredActions.length);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = (selectedIndex - 1 + filteredActions.length) % Math.max(1, filteredActions.length);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      executeAction(filteredActions[selectedIndex]);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    }
  }

  function executeAction(action?: CommandAction) {
    if (!action) return;
    action.run();
    onClose();
  }
</script>

{#if isOpen}
  <div class="cmd-backdrop" on:click={onClose} on:keydown={handleKeyDown} role="presentation">
    <div class="cmd-modal" on:click|stopPropagation role="dialog" aria-modal="true" aria-label="Command Palette">
      <!-- Search input header -->
      <div class="cmd-search-bar">
        <span class="cmd-icon">⌘</span>
        <input
          bind:this={inputEl}
          bind:value={query}
          on:keydown={handleKeyDown}
          placeholder="Search commands, tools, or shortcuts... (↑↓ to navigate, ↵ to run)"
          class="cmd-input"
          type="text"
          spellcheck="false"
        />
        <kbd class="esc-kbd" on:click={onClose}>ESC</kbd>
      </div>

      <!-- Action list items -->
      <div class="cmd-list">
        {#if filteredActions.length === 0}
          <div class="cmd-empty">
            <span>No matching commands for "{query}"</span>
          </div>
        {:else}
          {#each filteredActions as action, idx}
            <div
              class="cmd-item"
              class:selected={idx === selectedIndex}
              on:mouseenter={() => (selectedIndex = idx)}
              on:click={() => executeAction(action)}
              role="button"
              tabindex="0"
            >
              <span class="action-icon">{action.icon}</span>
              <div class="action-info">
                <div class="title-row">
                  <span class="action-title">{action.title}</span>
                  <span class="action-cat">{action.category}</span>
                </div>
                {#if action.subtitle}
                  <span class="action-sub">{action.subtitle}</span>
                {/if}
              </div>
              {#if action.shortcut}
                <div class="shortcut-pills">
                  <kbd>{action.shortcut}</kbd>
                </div>
              {/if}
            </div>
          {/each}
        {/if}
      </div>

      <!-- Footer tips -->
      <div class="cmd-footer">
        <div class="footer-tip">
          <kbd>↑</kbd><kbd>↓</kbd> <span>Navigate</span>
        </div>
        <div class="footer-tip">
          <kbd>↵</kbd> <span>Select</span>
        </div>
        <div class="footer-tip">
          <kbd>Esc</kbd> <span>Close</span>
        </div>
        <span class="footer-brand">QR Code Studio • 100% Offline Air-Gapped</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .cmd-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(10, 14, 23, 0.75);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 12vh;
    z-index: 9999;
    animation: fadeIn 0.12s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .cmd-modal {
    width: 620px;
    max-width: 92vw;
    background: #0f172a;
    border: 1px solid rgba(6, 182, 212, 0.35);
    border-radius: 12px;
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.7), 0 0 20px rgba(6, 182, 212, 0.15);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: slideDown 0.14s ease-out;
  }

  @keyframes slideDown {
    from {
      opacity: 0;
      transform: translateY(-8px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .cmd-search-bar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 18px;
    border-bottom: 1px solid rgba(71, 85, 105, 0.35);
    background: #131d2e;
  }

  .cmd-icon {
    font-size: 1.1rem;
    font-weight: 800;
    color: var(--accent-cyan);
    user-select: none;
  }

  .cmd-input {
    flex: 1;
    background: transparent;
    border: none;
    color: #f8fafc;
    font-size: 0.95rem;
    outline: none;
    font-family: var(--font-sans);
  }

  .cmd-input::placeholder {
    color: #64748b;
  }

  .esc-kbd {
    background: #1e293b;
    border: 1px solid #334155;
    color: #94a3b8;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 0.7rem;
    cursor: pointer;
  }

  .cmd-list {
    max-height: 380px;
    overflow-y: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .cmd-empty {
    padding: 32px 16px;
    text-align: center;
    color: #64748b;
    font-size: 0.85rem;
  }

  .cmd-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-radius: 8px;
    cursor: pointer;
    transition: background 0.1s ease;
    border: 1px solid transparent;
  }

  .cmd-item:hover,
  .cmd-item.selected {
    background: rgba(6, 182, 212, 0.12);
    border-color: rgba(6, 182, 212, 0.3);
  }

  .action-icon {
    font-size: 1.15rem;
    width: 26px;
    text-align: center;
    flex-shrink: 0;
  }

  .action-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow: hidden;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .action-title {
    font-size: 0.88rem;
    font-weight: 600;
    color: #f1f5f9;
  }

  .action-cat {
    font-size: 0.65rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #06b6d4;
    background: rgba(6, 182, 212, 0.15);
    padding: 1px 6px;
    border-radius: 4px;
  }

  .action-sub {
    font-size: 0.75rem;
    color: #94a3b8;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .shortcut-pills {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .shortcut-pills kbd {
    background: #1e293b;
    border: 1px solid #334155;
    color: #cbd5e1;
    font-size: 0.72rem;
    padding: 2px 6px;
    border-radius: 4px;
    font-family: inherit;
  }

  .cmd-footer {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 8px 16px;
    background: #0b1120;
    border-top: 1px solid rgba(71, 85, 105, 0.25);
    font-size: 0.72rem;
    color: #64748b;
  }

  .footer-tip {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .footer-tip kbd {
    background: #1e293b;
    border: 1px solid #334155;
    padding: 1px 4px;
    border-radius: 3px;
    font-size: 0.68rem;
    color: #94a3b8;
  }

  .footer-brand {
    margin-left: auto;
    font-size: 0.68rem;
    color: #475569;
  }
</style>

