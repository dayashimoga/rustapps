<script lang="ts">
  import { onMount, tick } from 'svelte';

  export interface CommandAction {
    id: string;
    title: string;
    subtitle?: string;
    category: 'Synthesis' | 'Privacy' | 'Export' | 'Security' | 'System';
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
          placeholder="Search commands, synthesis tools, shortcuts... (↑↓ to navigate, ↵ to run)"
          class="cmd-input"
          type="text"
          spellcheck="false"
        />
        <button class="esc-kbd" on:click={onClose}>ESC</button>
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
              on:keydown={(e) => e.key === 'Enter' && executeAction(action)}
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
        <div class="tip-item"><kbd>↑</kbd><kbd>↓</kbd> navigate</div>
        <div class="tip-item"><kbd>↵</kbd> select</div>
        <div class="tip-item"><kbd>esc</kbd> close</div>
      </div>
    </div>
  </div>
{/if}

<style>
  .cmd-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(4, 7, 12, 0.75);
    backdrop-filter: blur(8px);
    z-index: 10000;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 14vh;
  }

  .cmd-modal {
    width: 600px;
    max-width: 90vw;
    background: #0f141c;
    border: 1px solid #232c3d;
    border-radius: 12px;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.6), 0 0 0 1px rgba(56, 189, 248, 0.2);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .cmd-search-bar {
    display: flex;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid #1c2433;
    gap: 12px;
  }

  .cmd-icon {
    font-size: 1.1rem;
    color: #38bdf8;
    font-weight: 700;
  }

  .cmd-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: #f1f5f9;
    font-size: 0.95rem;
    font-family: inherit;
  }

  .cmd-input::placeholder {
    color: #475569;
  }

  .esc-kbd {
    background: #161e2b;
    border: 1px solid #283547;
    border-radius: 4px;
    color: #64748b;
    font-size: 0.7rem;
    padding: 3px 6px;
    font-family: monospace;
    cursor: pointer;
  }

  .cmd-list {
    max-height: 340px;
    overflow-y: auto;
    padding: 6px;
  }

  .cmd-empty {
    padding: 24px;
    text-align: center;
    color: #64748b;
    font-size: 0.85rem;
  }

  .cmd-item {
    display: flex;
    align-items: center;
    padding: 8px 12px;
    border-radius: 8px;
    gap: 12px;
    cursor: pointer;
    transition: all 0.1s ease;
  }

  .cmd-item:hover,
  .cmd-item.selected {
    background: rgba(56, 189, 248, 0.12);
    outline: 1px solid rgba(56, 189, 248, 0.3);
  }

  .action-icon {
    font-size: 1.2rem;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: #141b26;
    border-radius: 6px;
    border: 1px solid #1e293b;
  }

  .action-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .action-title {
    color: #f1f5f9;
    font-size: 0.88rem;
    font-weight: 500;
  }

  .action-cat {
    font-size: 0.68rem;
    color: #38bdf8;
    background: rgba(56, 189, 248, 0.1);
    padding: 1px 6px;
    border-radius: 4px;
    font-family: monospace;
    border: 1px solid rgba(56, 189, 248, 0.25);
  }

  .action-sub {
    color: #64748b;
    font-size: 0.75rem;
  }

  .shortcut-pills kbd {
    background: #141b26;
    border: 1px solid #283547;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 0.72rem;
    color: #94a3b8;
    font-family: monospace;
  }

  .cmd-footer {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 8px 16px;
    background: #090d14;
    border-top: 1px solid #161e2b;
    font-size: 0.72rem;
    color: #64748b;
  }

  .tip-item {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .tip-item kbd {
    background: #141b26;
    border: 1px solid #283547;
    padding: 1px 4px;
    border-radius: 3px;
    color: #94a3b8;
    font-family: monospace;
  }
</style>
