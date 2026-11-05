<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { SubtitleCue } from '../App.svelte';

  export let subtitles: SubtitleCue[] = [];
  export let currentTimeMs: number = 0;

  const dispatch = createEventDispatcher<{
    seek: number;
    exportSrt: void;
  }>();

  function formatTimestamp(ms: number): string {
    const s = Math.floor(ms / 1000);
    const m = Math.floor(s / 60);
    const remSec = s % 60;
    const millis = Math.floor(ms % 1000);
    return `${m}:${remSec.toString().padStart(2, '0')}.${millis.toString().padStart(3, '0')}`;
  }
</script>

<div class="subs-container">
  <div class="subs-header">
    <div class="header-text">
      <h3>Synchronized Subtitles ({subtitles.length})</h3>
      <span class="sub">100% offline speech-to-text cues</span>
    </div>
    <button class="export-btn" on:click={() => dispatch('exportSrt')}>
      Export .SRT
    </button>
  </div>

  <div class="subs-list">
    {#if subtitles.length === 0}
      <div class="empty-subs">No subtitles loaded for this session.</div>
    {:else}
      {#each subtitles as cue}
        <div
          class="cue-item"
          class:active={currentTimeMs >= cue.start_ms && currentTimeMs <= cue.end_ms}
          on:click={() => dispatch('seek', cue.start_ms)}
          role="button"
          tabindex="0"
          on:keydown={(e) => e.key === 'Enter' && dispatch('seek', cue.start_ms)}
        >
          <div class="cue-top">
            <span class="cue-time">{formatTimestamp(cue.start_ms)} → {formatTimestamp(cue.end_ms)}</span>
            {#if cue.speaker}
              <span class="cue-speaker">{cue.speaker}</span>
            {/if}
          </div>
          <p class="cue-text">{cue.text}</p>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .subs-container {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    height: 100%;
  }

  .subs-header {
    padding: 12px 16px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .subs-header h3 {
    font-size: 0.92rem;
    color: var(--text-main);
  }

  .subs-header .sub {
    font-size: 0.72rem;
    color: var(--text-sub);
  }

  .export-btn {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-muted);
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .export-btn:hover {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
  }

  .subs-list {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow-y: auto;
    flex: 1;
  }

  .cue-item {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 10px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .cue-item:hover {
    border-color: var(--accent-indigo);
    background: var(--bg-card-hover);
  }

  .cue-item.active {
    border-color: var(--accent-indigo);
    background: rgba(99, 102, 241, 0.12);
  }

  .cue-top {
    display: flex;
    justify-content: space-between;
    font-size: 0.7rem;
    margin-bottom: 4px;
  }

  .cue-time {
    font-family: var(--font-mono);
    color: var(--accent-cyan);
  }

  .cue-speaker {
    font-weight: 700;
    color: var(--text-sub);
  }

  .cue-text {
    font-size: 0.8rem;
    color: var(--text-main);
    line-height: 1.4;
  }

  .empty-subs {
    padding: 30px;
    text-align: center;
    color: var(--text-sub);
    font-size: 0.82rem;
  }
</style>
