<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { BookmarkRecord } from '../types';

  export let bookmarks: BookmarkRecord[] = [];
  export let isOpen: boolean = false;

  const dispatch = createEventDispatcher<{
    close: void;
    jumpToLine: number;
    deleteBookmark: string;
  }>();
</script>

{#if isOpen}
  <!-- Backdrop -->
  <div class="backdrop" role="button" tabindex="0" on:click={() => dispatch('close')} on:keydown={(e) => e.key === 'Escape' && dispatch('close')}></div>

  <!-- Drawer -->
  <aside class="drawer glass-panel">
    <div class="drawer-header">
      <div class="title-wrap">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z"></path>
        </svg>
        <span class="drawer-title">Saved Bookmarks</span>
      </div>
      <button class="close-btn" on:click={() => dispatch('close')}>&times;</button>
    </div>

    <div class="drawer-content">
      {#if bookmarks.length === 0}
        <div class="empty-state">
          <p>No bookmarks saved yet.</p>
          <span class="hint">Double-click any log line to add a bookmark.</span>
        </div>
      {:else}
        <ul class="bookmark-list">
          {#each bookmarks as bm (bm.id)}
            <li class="bookmark-item">
              <div
                class="bm-info"
                role="button"
                tabindex="0"
                on:click={() => dispatch('jumpToLine', bm.line_number)}
                on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && dispatch('jumpToLine', bm.line_number)}
              >
                <span class="badge line-badge">Line {bm.line_number + 1}</span>
                <span class="bm-label">{bm.label}</span>
              </div>
              <button
                class="delete-btn"
                on:click|stopPropagation={() => dispatch('deleteBookmark', bm.id)}
                title="Remove bookmark"
              >
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="3 6 5 6 21 6"></polyline>
                  <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                </svg>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </aside>
{/if}

<style>
  .backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(2px);
    z-index: 200;
  }

  .drawer {
    position: fixed;
    top: 0;
    right: 0;
    width: 340px;
    height: 100vh;
    display: flex;
    flex-direction: column;
    border-right: none;
    border-top: none;
    border-bottom: none;
    z-index: 201;
    animation: slideIn 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes slideIn {
    from { transform: translateX(100%); }
    to { transform: translateX(0); }
  }

  .drawer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-color);
  }

  .title-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    font-weight: 600;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 22px;
    cursor: pointer;
    line-height: 1;
    transition: color var(--transition-fast);
  }

  .close-btn:hover {
    color: var(--text-primary);
  }

  .drawer-content {
    flex: 1;
    overflow-y: auto;
    padding: 14px 16px;
  }

  .empty-state {
    text-align: center;
    padding: 40px 10px;
    color: var(--text-muted);
    font-size: 13px;
  }

  .hint {
    display: block;
    margin-top: 6px;
    font-size: 11px;
    opacity: 0.7;
  }

  .bookmark-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .bookmark-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    background: hsla(220, 20%, 15%, 0.5);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .bookmark-item:hover {
    background: hsla(220, 20%, 22%, 0.8);
    border-color: var(--border-glow);
    transform: translateX(-2px);
  }

  .bm-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
    overflow: hidden;
  }

  .line-badge {
    align-self: flex-start;
    color: var(--accent-blue);
    background: hsla(217, 91%, 60%, 0.15);
  }

  .bm-label {
    font-size: 12px;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .delete-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all var(--transition-fast);
  }

  .delete-btn:hover {
    color: var(--accent-rose);
    background: hsla(346, 87%, 65%, 0.15);
  }
</style>
