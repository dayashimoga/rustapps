<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let activeTab: 'text' | 'voice' | 'camera' | 'phrasebook' = 'text';

  const dispatch = createEventDispatcher<{
    changeTab: 'text' | 'voice' | 'camera' | 'phrasebook';
    openPhrasebook: void;
  }>();

  function selectTab(tab: 'text' | 'voice' | 'camera' | 'phrasebook') {
    dispatch('changeTab', tab);
  }
</script>

<header class="header">
  <div class="brand">
    <div class="logo-badge">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M5 8l6 6"></path>
        <path d="M4 14l6-6 2-3"></path>
        <path d="M2 5h12"></path>
        <path d="M7 2h1"></path>
        <path d="M22 22l-5-10-5 10"></path>
        <path d="M14 18h6"></path>
      </svg>
    </div>
    <div class="brand-text">
      <h1 class="brand-title">Universal Live Translator</h1>
      <div class="brand-status">
        <span class="pulse-indicator"></span>
        <span class="offline-pill">100% OFFLINE</span>
        <span class="sla-badge">LID &lt;500µs</span>
        <span class="sla-badge">NMT &lt;150ms</span>
      </div>
    </div>
  </div>

  <nav class="nav-tabs">
    <button
      class="tab-btn"
      class:active={activeTab === 'text'}
      on:click={() => selectTab('text')}
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M4 7V4h16v3M9 20h6M12 4v16"/>
      </svg>
      Text
    </button>
    <button
      class="tab-btn"
      class:active={activeTab === 'voice'}
      on:click={() => selectTab('voice')}
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/>
        <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
        <line x1="12" x2="12" y1="19" y2="22"/>
      </svg>
      Voice &amp; Conversation
    </button>
    <button
      class="tab-btn"
      class:active={activeTab === 'camera'}
      on:click={() => selectTab('camera')}
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"/>
        <circle cx="12" cy="13" r="4"/>
      </svg>
      Camera &amp; Subtitles
    </button>
    <button
      class="tab-btn"
      class:active={activeTab === 'phrasebook'}
      on:click={() => selectTab('phrasebook')}
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H20v20H6.5a2.5 2.5 0 0 1-2.5-2.5Z"/>
        <path d="M6 6h10M6 10h10"/>
      </svg>
      Phrasebook
    </button>
  </nav>
</header>

<style>
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1.5rem;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-subtle);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .logo-badge {
    width: 38px;
    height: 38px;
    border-radius: var(--radius-md);
    background: linear-gradient(135deg, #6366f1, #06b6d4);
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
  }

  .brand-title {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-main);
  }

  .brand-status {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.15rem;
  }

  .pulse-indicator {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent-emerald);
    box-shadow: 0 0 8px var(--accent-emerald);
  }

  .offline-pill {
    font-size: 0.65rem;
    font-weight: 700;
    letter-spacing: 0.05em;
    padding: 1px 6px;
    border-radius: 4px;
    background: rgba(16, 185, 129, 0.15);
    color: var(--accent-emerald);
    border: 1px solid rgba(16, 185, 129, 0.3);
  }

  .sla-badge {
    font-size: 0.65rem;
    font-family: var(--font-mono);
    padding: 1px 5px;
    border-radius: 4px;
    background: rgba(99, 102, 241, 0.15);
    color: #a5b4fc;
    border: 1px solid rgba(99, 102, 241, 0.25);
  }

  .nav-tabs {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: var(--bg-tertiary);
    padding: 0.25rem;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
  }

  .tab-btn {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.45rem 0.85rem;
    border-radius: var(--radius-sm);
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.825rem;
    font-weight: 500;
    transition: all 0.15s ease-in-out;
  }

  .tab-btn:hover {
    color: var(--text-main);
    background: rgba(255, 255, 255, 0.04);
  }

  .tab-btn.active {
    background: var(--accent-primary);
    color: white;
    box-shadow: 0 2px 8px rgba(99, 102, 241, 0.4);
  }
</style>

