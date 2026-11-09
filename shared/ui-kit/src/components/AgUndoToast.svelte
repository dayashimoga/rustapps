<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte';
  import type { ToastMessage } from './types';

  export let toasts: ToastMessage[] = [];

  const dispatch = createEventDispatcher<{
    dismiss: { id: string };
    undo: { id: string; action: () => void | Promise<void> };
  }>();

  function dismiss(id: string) {
    toasts = toasts.filter((t) => t.id !== id);
    dispatch('dismiss', { id });
  }

  function handleUndo(toast: ToastMessage) {
    if (toast.undoAction) {
      toast.undoAction();
      dispatch('undo', { id: toast.id, action: toast.undoAction });
    }
    dismiss(toast.id);
  }
</script>

<div class="ag-toast-container">
  {#each toasts as toast (toast.id)}
    {@const duration = toast.durationMs ?? 8000}
    <div class="ag-toast-item ag-glass-panel ag-toast-{toast.type || 'info'}">
      <div class="ag-toast-main">
        <div class="ag-toast-content">
          <div class="ag-toast-title">{toast.title}</div>
          {#if toast.description}
            <div class="ag-toast-desc">{toast.description}</div>
          {/if}
        </div>
        <div class="ag-toast-actions">
          {#if toast.undoAction}
            <button
              type="button"
              class="ag-btn ag-btn-undo"
              on:click={() => handleUndo(toast)}
            >
              {toast.undoLabel || 'Undo'}
            </button>
          {/if}
          <button
            type="button"
            class="ag-toast-close"
            on:click={() => dismiss(toast.id)}
            aria-label="Dismiss toast"
          >
            ✕
          </button>
        </div>
      </div>
      <!-- Progress Bar Timer -->
      <div
        class="ag-toast-progress"
        style="animation-duration: {duration}ms;"
        on:animationend={() => dismiss(toast.id)}
      ></div>
    </div>
  {/each}
</div>

<style>
  .ag-toast-container {
    position: fixed;
    bottom: 24px;
    right: 24px;
    z-index: 10000;
    display: flex;
    flex-direction: column-reverse;
    gap: 10px;
    max-width: 380px;
    width: 100%;
    pointer-events: none;
  }

  .ag-toast-item {
    pointer-events: auto;
    border-radius: var(--ag-radius-md);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    box-shadow: var(--ag-shadow-lg);
    animation: ag-toast-slide-in 180ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes ag-toast-slide-in {
    from {
      opacity: 0;
      transform: translateY(12px) scale(0.96);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .ag-toast-main {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 14px;
  }

  .ag-toast-content {
    flex: 1;
    overflow: hidden;
  }

  .ag-toast-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--ag-text-primary);
  }

  .ag-toast-desc {
    font-size: 11px;
    color: var(--ag-text-muted);
    margin-top: 2px;
  }

  .ag-toast-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .ag-btn-undo {
    font-size: 11px;
    height: 24px;
    padding: 0 8px;
    background: var(--ag-accent-primary);
    color: white;
    border: none;
  }

  .ag-btn-undo:hover {
    background: var(--ag-accent-primary-hover);
  }

  .ag-toast-close {
    background: transparent;
    border: none;
    color: var(--ag-text-muted);
    cursor: pointer;
    font-size: 11px;
  }

  .ag-toast-close:hover {
    color: var(--ag-text-primary);
  }

  .ag-toast-progress {
    height: 2px;
    background: var(--ag-accent-primary);
    width: 100%;
    animation-name: ag-toast-shrink;
    animation-timing-function: linear;
    animation-fill-mode: forwards;
  }

  @keyframes ag-toast-shrink {
    from {
      width: 100%;
    }
    to {
      width: 0%;
    }
  }
</style>

