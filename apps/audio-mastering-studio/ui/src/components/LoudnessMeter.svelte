<script lang="ts">
  export interface LoudnessMetrics {
    integrated_lufs: number;
    short_term_lufs: number;
    momentary_lufs: number;
    loudness_range_lu: number;
    true_peak_dbfs: number;
  }

  export let original: LoudnessMetrics = {
    integrated_lufs: -24.0,
    short_term_lufs: -23.5,
    momentary_lufs: -22.8,
    loudness_range_lu: 6.5,
    true_peak_dbfs: -4.5,
  };

  export let mastered: LoudnessMetrics = {
    integrated_lufs: -14.0,
    short_term_lufs: -13.5,
    momentary_lufs: -12.8,
    loudness_range_lu: 5.5,
    true_peak_dbfs: -0.5,
  };

  export let targetLufs: number = -14.0;

  function lufsToPercent(lufs: number): number {
    // Range from -40 LUFS (0%) to 0 LUFS (100%)
    const clamped = Math.max(-40, Math.min(0, lufs));
    return ((clamped + 40) / 40) * 100;
  }
</script>

<div class="loudness-meter-container">
  <div class="meter-header">
    <span class="title">EBU R128 LOUDNESS COMPLIANCE</span>
    <span class="target-tag">Target: {targetLufs.toFixed(1)} LUFS</span>
  </div>

  <div class="meters-body">
    <!-- Original Track Meter -->
    <div class="meter-channel">
      <div class="chan-title">PRE-MASTER</div>

      <div class="bar-wrap">
        <div class="bar-fill pre-fill" style="height: {lufsToPercent(original.integrated_lufs)}%;"></div>
        <div class="target-line" style="bottom: {lufsToPercent(targetLufs)}%;"></div>
      </div>

      <div class="metrics-column">
        <div class="metric-row">
          <span class="lbl">INTEGRATED</span>
          <span class="val">{original.integrated_lufs.toFixed(1)} LUFS</span>
        </div>
        <div class="metric-row">
          <span class="lbl">SHORT-TERM</span>
          <span class="val">{original.short_term_lufs.toFixed(1)} LUFS</span>
        </div>
        <div class="metric-row">
          <span class="lbl">TRUE PEAK</span>
          <span class="val">{original.true_peak_dbfs.toFixed(1)} dBFS</span>
        </div>
      </div>
    </div>

    <!-- Divider -->
    <div class="divider"></div>

    <!-- Mastered Track Meter -->
    <div class="meter-channel">
      <div class="chan-title master-title">AI MASTERED</div>

      <div class="bar-wrap">
        <div class="bar-fill post-fill" style="height: {lufsToPercent(mastered.integrated_lufs)}%;"></div>
        <div class="target-line" style="bottom: {lufsToPercent(targetLufs)}%;"></div>
      </div>

      <div class="metrics-column">
        <div class="metric-row">
          <span class="lbl">INTEGRATED</span>
          <span class="val highlight">{mastered.integrated_lufs.toFixed(1)} LUFS</span>
        </div>
        <div class="metric-row">
          <span class="lbl">SHORT-TERM</span>
          <span class="val highlight">{mastered.short_term_lufs.toFixed(1)} LUFS</span>
        </div>
        <div class="metric-row">
          <span class="lbl">TRUE PEAK</span>
          <span class="val" class:peak-safe={mastered.true_peak_dbfs <= 0}>{mastered.true_peak_dbfs.toFixed(1)} dBFS</span>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .loudness-meter-container {
    display: flex;
    flex-direction: column;
    background: #11151c;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    overflow: hidden;
    height: 100%;
  }

  .meter-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 12px;
    background: #161b24;
    border-bottom: 1px solid var(--border-color);
    font-family: var(--font-mono);
  }

  .title {
    font-size: 11px;
    font-weight: 700;
    color: var(--accent-gold);
  }

  .target-tag {
    font-size: 9px;
    font-weight: 600;
    color: #38bdf8;
    background: #0b1f33;
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid #0284c7;
  }

  .meters-body {
    display: flex;
    gap: 12px;
    padding: 10px 14px;
    flex: 1;
    background: #0d1016;
    align-items: center;
  }

  .meter-channel {
    flex: 1;
    display: flex;
    gap: 12px;
    align-items: center;
    font-family: var(--font-mono);
  }

  .chan-title {
    font-size: 10px;
    font-weight: 700;
    color: #64748b;
    writing-mode: vertical-lr;
    transform: rotate(180deg);
  }

  .master-title {
    color: var(--accent-gold);
  }

  .bar-wrap {
    width: 22px;
    height: 120px;
    background: #080b10;
    border: 1px solid #1e2838;
    border-radius: 4px;
    position: relative;
    overflow: hidden;
  }

  .bar-fill {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    border-radius: 2px;
    transition: height 0.3s ease;
  }

  .pre-fill {
    background: linear-gradient(to top, #0284c7, #38bdf8);
  }

  .post-fill {
    background: linear-gradient(to top, #e5a93c, #fbbf24);
  }

  .target-line {
    position: absolute;
    left: 0;
    right: 0;
    height: 2px;
    background: #ef4444;
    box-shadow: 0 0 4px #ef4444;
    z-index: 2;
  }

  .metrics-column {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
  }

  .metric-row {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .lbl {
    font-size: 8px;
    color: #64748b;
    font-weight: 600;
  }

  .val {
    font-size: 11px;
    color: #cbd5e1;
    font-weight: 700;
  }

  .val.highlight {
    color: var(--accent-gold);
  }

  .peak-safe {
    color: #4ade80;
  }

  .divider {
    width: 1px;
    height: 100px;
    background: #1e2838;
  }
</style>

