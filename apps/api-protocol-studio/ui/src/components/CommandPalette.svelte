<script lang="ts">
  import { tick, createEventDispatcher } from 'svelte';

  export interface CommandAction {
    id: string;
    title: string;
    subtitle?: string;
    category?: string;
    icon?: string;
    shortcut?: string;
    run: () => void | Promise<void>;
  }

  export let isOpen = false;
  export let actions: CommandAction[] = [];
  export let placeholder = 'Type an API command, action, or jump to... (Esc to close)';

  const dispatch = createEventDispatcher<{
    close: void;
  }>();

  let query = '';
  let selectedIndex = 0;
  let inputRef: HTMLInputElement | null = null;
  let listRef: HTMLDivElement | null = null;

  $: filteredActions = query.trim() === ''
    ? actions
    : actions.filter((a) => {
        const q = query.toLowerCase();
        return (
          a.title.toLowerCase().includes(q) ||
          (a.subtitle && a.subtitle.toLowerCase().includes(q)) ||
          (a.category && a.category.toLowerCase().includes(q)) ||
          (a.shortcut && a.shortcut.toLowerCase().includes(q))
        );
      });

  $: if (filteredActions.length > 0 && selectedIndex >= filteredActions.length) {
    selectedIndex = filteredActions.length - 1;
  }

  $: if (isOpen) {
    query = '';
    selectedIndex = 0;
    tick().then(() => {
      inputRef?.focus();
    });
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (!isOpen) return;

    if (e.key === 'Escape') {
      e.preventDefault();
      close();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (filteredActions.length > 0) {
        selectedIndex = (selectedIndex + 1) % filteredActions.length;
        scrollToSelected();
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (filteredActions.length > 0) {
        selectedIndex = (selectedIndex - 1 + filteredActions.length) % filteredActions.length;
        scrollToSelected();
      }
    } else if (e.key === 'Enter') {
      e.preventDefault();
      executeSelected();
    }
  }

  function scrollToSelected() {
    tick().then(() => {
      const activeEl = listRef?.querySelector(`[data-index="${selectedIndex}"]`);
      activeEl?.scrollIntoView({ block: 'nearest' });
    });
  }

  function executeSelected() {
    const action = filteredActions[selectedIndex];
    if (action) {
      close();
      action.run();
    }
  }

  function close() {
    dispatch('close');
  }
</script>

<svelte:window on:keydown={handleKeyDown} />

{#if isOpen}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
  <div
    class="palette-backdrop"
    role="dialog"
    aria-modal="true"
    aria-label="Command Palette"
    tabindex="-1"
    on:click|self={close}
  >
    <div class="palette-container glass-panel" role="document">
      <div class="palette-search-row">
        <span class="search-icon" aria-hidden="true">⚡</span>
        <input
          bind:this={inputRef}
          type="text"
          class="palette-input"
          {placeholder}
          bind:value={query}
          role="combobox"
          aria-autocomplete="list"
          aria-controls="palette-listbox"
          aria-expanded={filteredActions.length > 0}
          aria-activedescendant={filteredActions[selectedIndex] ? `cmd-${filteredActions[selectedIndex].id}` : undefined}
          spellcheck="false"
          autocomplete="off"
        />
        <button type="button" class="esc-badge" on:click={close} aria-label="Close command palette">ESC</button>
      </div>

      <div
        bind:this={listRef}
        id="palette-listbox"
        class="palette-list"
        role="listbox"
        aria-label="Command Suggestions"
      >
        {#if filteredActions.length === 0}
          <div class="empty-state">
            <span>No matching commands found for "{query}"</span>
          </div>
        {:else}
          {#each filteredActions as action, idx (action.id)}
            <!-- svelte-ignore a11y-click-events-have-key-events -->
            <div
              id={`cmd-${action.id}`}
              class="palette-item {idx === selectedIndex ? 'selected' : ''}"
              role="option"
              tabindex="-1"
              aria-selected={idx === selectedIndex}
              data-index={idx}
              on:mouseenter={() => (selectedIndex = idx)}
              on:click={() => {
                selectedIndex = idx;
                executeSelected();
              }}
            >
              <div class="item-left">
                {#if action.icon}
                  <span class="item-icon" aria-hidden="true">{action.icon}</span>
                {/if}
                <div class="item-text">
                  <div class="item-title">{action.title}</div>
                  {#if action.subtitle}
                    <div class="item-subtitle">{action.subtitle}</div>
                  {/if}
                </div>
              </div>
              <div class="item-right">
                {#if action.category}
                  <span class="item-category">{action.category}</span>
                {/if}
                {#if action.shortcut}
                  <kbd class="item-shortcut">{action.shortcut}</kbd>
                {/if}
              </div>
            </div>
          {/each}
        {/if}
      </div>

      <div class="palette-footer">
        <div class="footer-tip">
          <kbd>↑</kbd><kbd>↓</kbd> <span>Navigate</span>
          <kbd>↵</kbd> <span>Select</span>
          <kbd>ESC</kbd> <span>Dismiss</span>
        </div>
        <div class="footer-meta">100% Air-Gapped API Studio</div>
      </div>
    </div>
  </div>
{/if}

<style>
  .palette-backdrop {
    position: fixed;
    inset: 0;
    z-index: 9999;
    background: rgba(11, 19, 43, 0.75);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 10vh;
    animation: fadeIn 0.15s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .palette-container {
    width: 620px;
    max-width: 92vw;
    background: rgba(28, 37, 65, 0.95);
    border: 1px solid rgba(255, 255, 255, 0.12);
    box-shadow: 0 24px 48px -12px rgba(0, 0, 0, 0.75), 0 0 24px rgba(56, 189, 248, 0.2);
    border-radius: 12px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: slideDown 0.15s ease-out;
  }

  @keyframes slideDown {
    from { transform: translateY(-12px); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }

  .palette-search-row {
    display: flex;
    align-items: center;
    padding: 14px 18px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    gap: 12px;
    background: rgba(255, 255, 255, 0.02);
  }

  .search-icon {
    font-size: 16px;
    opacity: 0.8;
  }

  .palette-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: #f8fafc;
    font-size: 15px;
    font-family: inherit;
  }

  .palette-input::placeholder {
    color: #64748b;
  }

  .esc-badge {
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.15);
    color: #94a3b8;
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 4px;
    cursor: pointer;
  }

  .palette-list {
    max-height: 360px;
    overflow-y: auto;
    padding: 6px;
  }

  .palette-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-radius: 8px;
    cursor: pointer;
    transition: background 0.1s ease;
    border: 1px solid transparent;
  }

  .palette-item.selected {
    background: rgba(56, 189, 248, 0.2);
    border-color: rgba(56, 189, 248, 0.4);
  }

  .item-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .item-icon {
    font-size: 16px;
  }

  .item-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .item-title {
    color: #f8fafc;
    font-size: 13px;
    font-weight: 500;
  }

  .palette-item.selected .item-title {
    color: #38bdf8;
  }

  .item-subtitle {
    color: #94a3b8;
    font-size: 11px;
  }

  .item-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .item-category {
    font-size: 10px;
    color: #64748b;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.04);
  }

  .item-shortcut {
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid rgba(255, 255, 255, 0.15);
    color: #cbd5e1;
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .empty-state {
    padding: 32px 16px;
    text-align: center;
    color: #64748b;
    font-size: 13px;
  }

  .palette-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 18px;
    background: rgba(0, 0, 0, 0.25);
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    font-size: 11px;
    color: #64748b;
  }

  .footer-tip {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .footer-tip kbd {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.12);
    padding: 1px 4px;
    border-radius: 3px;
    font-size: 10px;
    color: #94a3b8;
  }

  .footer-meta {
    font-size: 10px;
    color: #38bdf8;
    font-weight: 500;
  }
</style>

