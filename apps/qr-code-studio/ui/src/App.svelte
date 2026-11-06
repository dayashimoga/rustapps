<script lang="ts">
  import GeneratorStudio from './components/GeneratorStudio.svelte';
  import ScannerViewport from './components/ScannerViewport.svelte';
  import SecurityInspector from './components/SecurityInspector.svelte';
  import BatchProcessor from './components/BatchProcessor.svelte';
  import HistoryAnalytics from './components/HistoryAnalytics.svelte';
  import LicenseModal from './components/LicenseModal.svelte';

  type NavView = 'generator' | 'scanner' | 'security' | 'batch' | 'history';
  let activeView: NavView = 'generator';

  let inspectTargetPayload = '';
  let isLicenseModalOpen = false;

  function navigateToSecurity(payload: string) {
    inspectTargetPayload = payload;
    activeView = 'security';
  }

  function handleSelectHistoryPayload(payload: string) {
    inspectTargetPayload = payload;
    activeView = 'security';
  }
</script>

<div class="app-layout">
  <!-- Top App Header -->
  <header class="app-header">
    <div class="brand-section">
      <div class="brand-logo">QR</div>
      <div class="brand-titles">
        <h1 class="brand-name">QR Code Studio</h1>
        <span class="offline-tag">100% Offline Air-Gapped Suite</span>
      </div>
    </div>

    <!-- Main Navigation Bar -->
    <nav class="nav-links">
      <button
        class="nav-btn"
        class:active={activeView === 'generator'}
        on:click={() => (activeView = 'generator')}
      >
        <span class="icon">✨</span>
        <span>Generator</span>
      </button>

      <button
        class="nav-btn"
        class:active={activeView === 'scanner'}
        on:click={() => (activeView = 'scanner')}
      >
        <span class="icon">📷</span>
        <span>Live Scanner</span>
      </button>

      <button
        class="nav-btn"
        class:active={activeView === 'security'}
        on:click={() => (activeView = 'security')}
      >
        <span class="icon">🛡️</span>
        <span>Threat Inspector</span>
      </button>

      <button
        class="nav-btn"
        class:active={activeView === 'batch'}
        on:click={() => (activeView = 'batch')}
      >
        <span class="icon">⚡</span>
        <span>Batch Engine</span>
      </button>

      <button
        class="nav-btn"
        class:active={activeView === 'history'}
        on:click={() => (activeView = 'history')}
      >
        <span class="icon">📊</span>
        <span>History & Stats</span>
      </button>
    </nav>

    <!-- Header Actions -->
    <div class="header-actions">
      <div class="airgap-badge">
        <span class="dot"></span>
        <span>Air-Gap Active</span>
      </div>

      <button class="btn-license" on:click={() => (isLicenseModalOpen = true)}>
        🔑 License
      </button>
    </div>
  </header>

  <!-- Main View Area -->
  <main class="app-main">
    {#if activeView === 'generator'}
      <GeneratorStudio />
    {:else if activeView === 'scanner'}
      <ScannerViewport
        onInspectThreats={navigateToSecurity}
      />
    {:else if activeView === 'security'}
      <SecurityInspector
        initialPayload={inspectTargetPayload}
      />
    {:else if activeView === 'batch'}
      <BatchProcessor />
    {:else if activeView === 'history'}
      <HistoryAnalytics
        onSelectPayload={handleSelectHistoryPayload}
      />
    {/if}
  </main>

  <!-- Offline License Modal -->
  <LicenseModal
    isOpen={isLicenseModalOpen}
    onClose={() => (isLicenseModalOpen = false)}
  />
</div>

<style>
  .app-layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    background: var(--bg-primary);
    color: var(--text-primary);
  }

  .app-header {
    height: 60px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 1.5rem;
    user-select: none;
    z-index: 10;
  }

  .brand-section {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .brand-logo {
    width: 32px;
    height: 32px;
    background: linear-gradient(135deg, var(--accent-cyan), var(--accent-indigo));
    color: #0b0f19;
    font-weight: 900;
    font-size: 0.9rem;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
    letter-spacing: -0.05em;
  }

  .brand-titles {
    display: flex;
    flex-direction: column;
  }

  .brand-name {
    font-size: 1rem;
    font-weight: 800;
    letter-spacing: -0.02em;
    color: var(--text-primary);
  }

  .offline-tag {
    font-size: 0.65rem;
    color: var(--accent-cyan);
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .nav-links {
    display: flex;
    gap: 0.5rem;
  }

  .nav-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.45rem 0.85rem;
    background: transparent;
    border: 1px solid transparent;
    color: var(--text-secondary);
    border-radius: 8px;
    cursor: pointer;
    font-size: 0.85rem;
    font-weight: 500;
    transition: all 0.15s ease;
  }

  .nav-btn:hover {
    color: var(--text-primary);
    background: var(--bg-elevated);
  }

  .nav-btn.active {
    background: rgba(6, 182, 212, 0.15);
    color: var(--accent-cyan);
    border-color: rgba(6, 182, 212, 0.35);
    font-weight: 600;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .airgap-badge {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--accent-emerald);
    background: rgba(16, 185, 129, 0.1);
    border: 1px solid rgba(16, 185, 129, 0.3);
    padding: 0.25rem 0.65rem;
    border-radius: 9999px;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent-emerald);
    box-shadow: 0 0 6px var(--accent-emerald);
  }

  .btn-license {
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    padding: 0.35rem 0.75rem;
    border-radius: 6px;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .btn-license:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .app-main {
    flex: 1;
    overflow: hidden;
  }
</style>
