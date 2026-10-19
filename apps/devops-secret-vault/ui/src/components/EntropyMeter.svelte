<script lang="ts">
  import type { PasswordStrength } from '../types';

  export let strength: PasswordStrength;

  $: percent = Math.min(100, Math.round((strength.entropy_bits / 90) * 100));

  $: barColor = {
    very_weak: '#ef4444',
    weak: '#f97316',
    fair: '#eab308',
    strong: '#10b981',
    very_strong: '#06b6d4',
  }[strength.rating] || '#64748b';

  $: ratingLabel = {
    very_weak: 'Very Weak',
    weak: 'Weak',
    fair: 'Fair',
    strong: 'Strong',
    very_strong: 'Military-Grade',
  }[strength.rating] || 'Unknown';
</script>

<div class="entropy-container">
  <div class="entropy-header">
    <span class="entropy-title">Password Strength</span>
    <span class="entropy-score" style="color: {barColor}">
      {ratingLabel} ({Math.round(strength.entropy_bits)} bits)
    </span>
  </div>
  <div class="bar-track">
    <div
      class="bar-fill"
      style="width: {percent}%; background-color: {barColor};"
    ></div>
  </div>
  <div class="rules-chips">
    <span class="chip {strength.has_lowercase ? 'active' : ''}">a-z</span>
    <span class="chip {strength.has_uppercase ? 'active' : ''}">A-Z</span>
    <span class="chip {strength.has_digits ? 'active' : ''}">0-9</span>
    <span class="chip {strength.has_symbols ? 'active' : ''}">#$&</span>
    <span class="chip {strength.length >= 16 ? 'active' : ''}">16+ chars</span>
  </div>
</div>

<style>
  .entropy-container {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px;
    background: rgba(10, 14, 23, 0.6);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
  }

  .entropy-header {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    font-weight: 600;
  }

  .entropy-title {
    color: var(--text-muted);
  }

  .bar-track {
    width: 100%;
    height: 6px;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 999px;
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    transition: width 0.3s ease, background-color 0.3s ease;
    border-radius: 999px;
  }

  .rules-chips {
    display: flex;
    gap: 6px;
    margin-top: 2px;
  }

  .chip {
    font-size: 10px;
    font-family: var(--font-mono);
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.04);
    color: var(--text-faint);
    border: 1px solid transparent;
    transition: all 0.2s ease;
  }

  .chip.active {
    background: rgba(16, 185, 129, 0.15);
    color: #6ee7b7;
    border-color: rgba(16, 185, 129, 0.3);
  }
</style>
