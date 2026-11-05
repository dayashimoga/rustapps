<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { TutorialStep } from '../App.svelte';

  export let steps: TutorialStep[] = [];
  export let currentTimeMs: number = 0;

  const dispatch = createEventDispatcher<{
    seek: number;
    stepSelect: TutorialStep;
  }>();

  function formatTime(ms: number): string {
    const s = Math.floor(ms / 1000);
    const m = Math.floor(s / 60);
    const rem = s % 60;
    return `${m}:${rem.toString().padStart(2, '0')}`;
  }
</script>

<div class="steps-container">
  <div class="steps-header">
    <h3>Generated Tutorial Steps ({steps.length})</h3>
    <span class="sub">AI-extracted interactive click milestones</span>
  </div>

  <div class="steps-list">
    {#if steps.length === 0}
      <div class="empty-steps">No tutorial milestones generated yet.</div>
    {:else}
      {#each steps as step}
        <div
          class="step-item"
          class:active={Math.abs(currentTimeMs - step.timestamp_ms) < 1500}
          on:click={() => dispatch('seek', step.timestamp_ms)}
          role="button"
          tabindex="0"
          on:keydown={(e) => e.key === 'Enter' && dispatch('seek', step.timestamp_ms)}
        >
          <div class="step-top">
            <span class="step-num">STEP {step.step_number}</span>
            <span class="step-time">{formatTime(step.timestamp_ms)}</span>
          </div>

          <div class="step-body">
            <h4 class="step-title">{step.title}</h4>
            <p class="step-inst">{step.instruction}</p>
          </div>

          <div class="step-meta">
            <span class="action-tag">{step.action_type}</span>
            <span class="coord-tag">({(step.target_x * 100).toFixed(0)}%, {(step.target_y * 100).toFixed(0)}%)</span>
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .steps-container {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    height: 100%;
  }

  .steps-header {
    padding: 14px 16px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
  }

  .steps-header h3 {
    font-size: 0.95rem;
    color: var(--text-main);
  }

  .steps-header .sub {
    font-size: 0.72rem;
    color: var(--text-sub);
  }

  .steps-list {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow-y: auto;
    flex: 1;
  }

  .step-item {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 12px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .step-item:hover {
    border-color: var(--accent-blue);
    background: var(--bg-card-hover);
  }

  .step-item.active {
    border-color: var(--accent-cyan);
    background: rgba(56, 189, 248, 0.08);
  }

  .step-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
  }

  .step-num {
    background: var(--accent-blue);
    color: #fff;
    font-size: 0.68rem;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .step-time {
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--text-sub);
  }

  .step-title {
    font-size: 0.85rem;
    color: var(--text-main);
    margin-bottom: 4px;
  }

  .step-inst {
    font-size: 0.78rem;
    color: var(--text-muted);
    line-height: 1.4;
  }

  .step-meta {
    margin-top: 8px;
    display: flex;
    justify-content: space-between;
    font-size: 0.7rem;
  }

  .action-tag {
    color: var(--accent-cyan);
    font-weight: 600;
  }

  .coord-tag {
    font-family: var(--font-mono);
    color: var(--text-sub);
  }

  .empty-steps {
    padding: 30px;
    text-align: center;
    color: var(--text-sub);
    font-size: 0.82rem;
  }
</style>
