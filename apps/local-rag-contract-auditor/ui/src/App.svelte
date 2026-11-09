<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeIpc } from './lib/ipc';
  import type {
    AuditorStats,
    ClauseAuditFinding,
    ContractDocument,
    ContractSummary,
    LegalAuditCertificate,
    LicenseStatusReport,
    RiskLevel,
  } from './lib/types';

  import CommandPalette from './components/CommandPalette.svelte';
  import LicenseModal from './components/LicenseModal.svelte';
  import NewAuditModal from './components/NewAuditModal.svelte';
  import AuditCertificateModal from './components/AuditCertificateModal.svelte';
  import DiffComparisonModal from './components/DiffComparisonModal.svelte';
  import ClauseDetailPane from './components/ClauseDetailPane.svelte';

  let stats: AuditorStats = {
    total_contracts_audited: 0,
    total_clauses_analyzed: 0,
    critical_risks_prevented: 0,
    avg_risk_score: 0.0,
  };

  let contracts: ContractSummary[] = [];
  let currentContract: ContractDocument | null = null;
  let selectedClause: ClauseAuditFinding | null = null;
  let license: LicenseStatusReport = {
    tier: 'Community',
    is_valid: true,
    features: ['basic_rag_review'],
  };

  // Modals state
  let isPaletteOpen = false;
  let isLicenseOpen = false;
  let isNewAuditOpen = false;
  let isCertModalOpen = false;
  let isDiffModalOpen = false;
  let generatedCert: LegalAuditCertificate | null = null;

  // Filters
  let activeRiskFilter: 'all' | RiskLevel = 'all';
  let clauseSearchQuery = '';

  $: filteredClauses = (currentContract?.clauses || []).filter((c) => {
    const matchesRisk = activeRiskFilter === 'all' || c.risk_level === activeRiskFilter;
    const matchesSearch =
      c.heading.toLowerCase().includes(clauseSearchQuery.toLowerCase()) ||
      c.section_number.toLowerCase().includes(clauseSearchQuery.toLowerCase()) ||
      c.raw_text.toLowerCase().includes(clauseSearchQuery.toLowerCase());
    return matchesRisk && matchesSearch;
  });

  async function loadData() {
    try {
      const [s, cList, lic] = await Promise.all([
        invokeIpc<AuditorStats>('get_stats'),
        invokeIpc<ContractSummary[]>('list_contracts'),
        invokeIpc<LicenseStatusReport>('get_license_status'),
      ]);
      stats = s;
      contracts = cList;
      license = lic;

      if (cList.length > 0 && (!currentContract || !cList.some((c) => c.document_id === currentContract?.document_id))) {
        await selectContract(cList[0].document_id);
      }
    } catch (e) {
      console.error('Failed to load initial auditor data:', e);
    }
  }

  async function selectContract(docId: string) {
    try {
      const doc = await invokeIpc<ContractDocument>('get_contract', { document_id: docId });
      currentContract = doc;
      if (doc.clauses && doc.clauses.length > 0) {
        selectedClause = doc.clauses[0];
      } else {
        selectedClause = null;
      }
    } catch (e) {
      console.error('Failed to load contract:', e);
    }
  }

  async function issueCertificate() {
    if (!currentContract) return;
    try {
      const cert = await invokeIpc<LegalAuditCertificate>('generate_audit_certificate', {
        document_id: currentContract.document_id,
      });
      generatedCert = cert;
      isCertModalOpen = true;
    } catch (e: any) {
      alert(`Certificate signing failed: ${e.message || e}`);
    }
  }

  function handleCommand(cmdId: string) {
    switch (cmdId) {
      case 'new_audit':
        isNewAuditOpen = true;
        break;
      case 'compare_versions':
        isDiffModalOpen = true;
        break;
      case 'generate_cert':
        issueCertificate();
        break;
      case 'filter_critical':
        activeRiskFilter = 'critical';
        break;
      case 'filter_all':
        activeRiskFilter = 'all';
        break;
      case 'open_license':
        isLicenseOpen = true;
        break;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      isPaletteOpen = !isPaletteOpen;
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'n') {
      e.preventDefault();
      isNewAuditOpen = true;
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'd') {
      e.preventDefault();
      isDiffModalOpen = true;
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
      e.preventDefault();
      issueCertificate();
    }
  }

  onMount(() => {
    loadData();
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  });
</script>

<div class="app-shell">
  <!-- Top Application Bar -->
  <header class="app-header">
    <div class="brand">
      <div class="logo-mark">⚖️</div>
      <div class="brand-titles">
        <h1 class="app-title">Local RAG Contract Auditor</h1>
        <span class="tagline">Air-Gapped Offline Legal & Compliance Intelligence</span>
      </div>
    </div>

    <!-- Live Executive Metrics Counters -->
    <div class="metrics-bar">
      <div class="metric-item">
        <span class="m-val">{stats.total_contracts_audited}</span>
        <span class="m-lbl">Contracts</span>
      </div>
      <div class="metric-divider"></div>
      <div class="metric-item">
        <span class="m-val">{stats.total_clauses_analyzed}</span>
        <span class="m-lbl">Clauses Evaluated</span>
      </div>
      <div class="metric-divider"></div>
      <div class="metric-item crit">
        <span class="m-val">{stats.critical_risks_prevented}</span>
        <span class="m-lbl">Critical Risks Flagged</span>
      </div>
      <div class="metric-divider"></div>
      <div class="metric-item">
        <span class="m-val">{stats.avg_risk_score.toFixed(1)} / 10</span>
        <span class="m-lbl">Avg Risk Exposure</span>
      </div>
    </div>

    <!-- Status badges and tools -->
    <div class="header-tools">
      <span class="airgap-badge" title="Zero Telemetry & 100% Offline Autonomy">
        <span class="dot"></span> Offline Air-Gapped
      </span>

      <button type="button" class="license-pill" on:click={() => (isLicenseOpen = true)}>
        <span class="lic-icon">🔑</span>
        <span>{license.tier}</span>
      </button>

      <button
        type="button"
        class="palette-btn"
        on:click={() => (isPaletteOpen = true)}
        title="Open Command Palette (Ctrl+K)"
      >
        <span>Ctrl+K</span>
      </button>
    </div>
  </header>

  <!-- Sub-header Action Ribbon -->
  <div class="action-ribbon">
    <div class="ribbon-left">
      <span class="ribbon-lbl">Active Contract:</span>
      <select
        class="contract-dropdown"
        value={currentContract?.document_id || ''}
        on:change={(e) => selectContract(e.currentTarget.value)}
      >
        {#each contracts as c}
          <option value={c.document_id}>
            {c.title} — Grade: {c.risk_grade.substring(0, 2)} ({c.total_clauses} clauses)
          </option>
        {/each}
      </select>

      {#if currentContract}
        <span class="grade-tag">{currentContract.risk_grade}</span>
      {/if}
    </div>

    <div class="ribbon-actions">
      <button type="button" class="btn action-btn primary" on:click={() => (isNewAuditOpen = true)}>
        <span class="btn-icon">➕</span> Audit New Contract
      </button>
      <button type="button" class="btn action-btn" on:click={() => (isDiffModalOpen = true)}>
        <span class="btn-icon">⚖️</span> Version Diff Studio
      </button>
      <button type="button" class="btn action-btn cert" on:click={issueCertificate}>
        <span class="btn-icon">🛡️</span> Issue Audit Certificate
      </button>
    </div>
  </div>

  <!-- Workspace Split Pane -->
  <main class="main-workspace">
    <!-- Left Clause Checklist Sidebar -->
    <aside class="clause-sidebar">
      <div class="sidebar-search">
        <input
          type="text"
          placeholder="Filter clauses by text or section..."
          bind:value={clauseSearchQuery}
        />
      </div>

      <div class="filter-pills">
        <button
          type="button"
          class="f-pill"
          class:active={activeRiskFilter === 'all'}
          on:click={() => (activeRiskFilter = 'all')}
        >
          All ({(currentContract?.clauses || []).length})
        </button>
        <button
          type="button"
          class="f-pill crit"
          class:active={activeRiskFilter === 'critical'}
          on:click={() => (activeRiskFilter = 'critical')}
        >
          Critical
        </button>
        <button
          type="button"
          class="f-pill high"
          class:active={activeRiskFilter === 'high'}
          on:click={() => (activeRiskFilter = 'high')}
        >
          High
        </button>
        <button
          type="button"
          class="f-pill med"
          class:active={activeRiskFilter === 'medium'}
          on:click={() => (activeRiskFilter = 'medium')}
        >
          Medium
        </button>
        <button
          type="button"
          class="f-pill fav"
          class:active={activeRiskFilter === 'favorable'}
          on:click={() => (activeRiskFilter = 'favorable')}
        >
          Favorable
        </button>
      </div>

      <div class="clause-list">
        {#each filteredClauses as c}
          <div
            class="clause-item"
            class:selected={selectedClause?.clause_id === c.clause_id}
            on:click={() => (selectedClause = c)}
            role="button"
            tabindex="0"
          >
            <div class="item-top">
              <span class="item-sec">{c.section_number}</span>
              <span class="item-score {c.risk_level}">{c.risk_score.toFixed(1)}</span>
            </div>
            <div class="item-title">{c.heading}</div>
            <div class="item-bottom">
              <span class="item-cat">{c.category.replace(/_/g, ' ')}</span>
              <span class="item-level {c.risk_level}">{c.risk_level.toUpperCase()}</span>
            </div>
          </div>
        {/each}

        {#if filteredClauses.length === 0}
          <div class="empty-list">No clauses match the active filters.</div>
        {/if}
      </div>
    </aside>

    <!-- Center/Right Deep Dive Pane -->
    <section class="detail-workspace">
      <ClauseDetailPane clause={selectedClause} />
    </section>
  </main>

  <!-- Modals -->
  <CommandPalette
    isOpen={isPaletteOpen}
    onClose={() => (isPaletteOpen = false)}
    onSelectCommand={handleCommand}
  />

  <LicenseModal
    isOpen={isLicenseOpen}
    onClose={() => (isLicenseOpen = false)}
    onLicenseUpdated={loadData}
  />

  <NewAuditModal
    isOpen={isNewAuditOpen}
    onClose={() => (isNewAuditOpen = false)}
    onAuditCompleted={async (doc) => {
      await loadData();
      await selectContract(doc.document_id);
    }}
  />

  <DiffComparisonModal
    isOpen={isDiffModalOpen}
    contracts={contracts}
    onClose={() => (isDiffModalOpen = false)}
  />

  <AuditCertificateModal
    isOpen={isCertModalOpen}
    certificate={generatedCert}
    onClose={() => (isCertModalOpen = false)}
  />
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    background: #030712;
    color: #f3f4f6;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
    overflow: hidden;
    user-select: none;
  }

  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    background: #030712;
  }

  .app-header {
    height: 56px;
    background: #090e1a;
    border-bottom: 1px solid #1a2338;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 18px;
    z-index: 10;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .logo-mark {
    font-size: 1.5rem;
  }

  .brand-titles {
    display: flex;
    flex-direction: column;
  }

  .app-title {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 700;
    color: #f8fafc;
    letter-spacing: -0.01em;
  }

  .tagline {
    font-size: 0.72rem;
    color: #64748b;
    font-weight: 500;
  }

  .metrics-bar {
    display: flex;
    align-items: center;
    gap: 16px;
    background: #060a12;
    padding: 6px 16px;
    border-radius: 8px;
    border: 1px solid #151f33;
  }

  .metric-item {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .m-val {
    font-size: 1rem;
    font-weight: 800;
    color: #38bdf8;
    line-height: 1.1;
  }

  .metric-item.crit .m-val {
    color: #ef4444;
  }

  .m-lbl {
    font-size: 0.65rem;
    color: #94a3b8;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .metric-divider {
    width: 1px;
    height: 22px;
    background: #1e293b;
  }

  .header-tools {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .airgap-badge {
    display: flex;
    align-items: center;
    gap: 6px;
    background: rgba(16, 185, 129, 0.12);
    border: 1px solid rgba(16, 185, 129, 0.3);
    color: #34d399;
    font-size: 0.75rem;
    font-weight: 600;
    padding: 4px 10px;
    border-radius: 9999px;
  }

  .airgap-badge .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #10b981;
    box-shadow: 0 0 6px #10b981;
  }

  .license-pill {
    display: flex;
    align-items: center;
    gap: 6px;
    background: #132338;
    border: 1px solid #1e3a8a;
    color: #60a5fa;
    font-size: 0.76rem;
    font-weight: 600;
    padding: 4px 10px;
    border-radius: 6px;
    cursor: pointer;
  }

  .palette-btn {
    background: #1e293b;
    border: 1px solid #334155;
    color: #94a3b8;
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 0.75rem;
    font-family: monospace;
    cursor: pointer;
  }

  .action-ribbon {
    height: 48px;
    background: #060a12;
    border-bottom: 1px solid #151f33;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 18px;
  }

  .ribbon-left {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .ribbon-lbl {
    font-size: 0.82rem;
    color: #94a3b8;
    font-weight: 500;
  }

  .contract-dropdown {
    background: #0d121f;
    border: 1px solid #1f293d;
    color: #f1f5f9;
    font-size: 0.84rem;
    padding: 5px 10px;
    border-radius: 6px;
    min-width: 320px;
    outline: none;
  }

  .grade-tag {
    background: #1e293b;
    color: #fbbf24;
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 0.76rem;
    font-weight: 700;
  }

  .ribbon-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
    border: 1px solid #1f293d;
    background: #0d121f;
    color: #cbd5e1;
    transition: background 0.15s ease;
  }

  .action-btn:hover {
    background: #1e293b;
  }

  .action-btn.primary {
    background: #2563eb;
    border-color: #3b82f6;
    color: #fff;
  }

  .action-btn.primary:hover {
    background: #1d4ed8;
  }

  .action-btn.cert {
    border-color: #6d28d9;
    color: #c4b5fd;
    background: rgba(109, 40, 217, 0.15);
  }

  .action-btn.cert:hover {
    background: rgba(109, 40, 217, 0.3);
  }

  .main-workspace {
    flex: 1;
    display: flex;
    overflow: hidden;
  }

  .clause-sidebar {
    width: 380px;
    min-width: 320px;
    max-width: 440px;
    background: #070b14;
    border-right: 1px solid #151f33;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar-search {
    padding: 10px;
    border-bottom: 1px solid #151f33;
  }

  .sidebar-search input {
    width: 100%;
    box-sizing: border-box;
    background: #0d121f;
    border: 1px solid #1f293d;
    border-radius: 6px;
    color: #e2e8f0;
    padding: 6px 10px;
    font-size: 0.82rem;
    outline: none;
  }

  .filter-pills {
    display: flex;
    gap: 6px;
    padding: 8px 10px;
    border-bottom: 1px solid #151f33;
    overflow-x: auto;
  }

  .f-pill {
    background: #0d121f;
    border: 1px solid #1f293d;
    color: #94a3b8;
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 0.72rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }

  .f-pill.active {
    background: #1e293b;
    color: #f1f5f9;
    border-color: #3b82f6;
  }

  .f-pill.crit.active { border-color: #ef4444; color: #f87171; }
  .f-pill.high.active { border-color: #f97316; color: #fb923c; }
  .f-pill.med.active { border-color: #f59e0b; color: #fcd34d; }
  .f-pill.fav.active { border-color: #10b981; color: #34d399; }

  .clause-list {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .clause-item {
    background: #0a0f1d;
    border: 1px solid #172033;
    border-radius: 6px;
    padding: 10px 12px;
    cursor: pointer;
    transition: background 0.15s ease, border-color 0.15s ease;
  }

  .clause-item:hover {
    background: #101726;
  }

  .clause-item.selected {
    background: #131c30;
    border-color: #2563eb;
  }

  .item-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 4px;
  }

  .item-sec {
    font-size: 0.75rem;
    font-weight: 700;
    color: #38bdf8;
  }

  .item-score {
    font-size: 0.78rem;
    font-weight: 800;
    padding: 1px 6px;
    border-radius: 4px;
  }

  .item-score.critical { background: rgba(239, 68, 68, 0.2); color: #f87171; }
  .item-score.high { background: rgba(249, 115, 22, 0.2); color: #fb923c; }
  .item-score.medium { background: rgba(245, 158, 11, 0.2); color: #fcd34d; }
  .item-score.low { background: rgba(59, 130, 246, 0.2); color: #60a5fa; }
  .item-score.favorable { background: rgba(16, 185, 129, 0.2); color: #34d399; }

  .item-title {
    font-size: 0.86rem;
    font-weight: 600;
    color: #f1f5f9;
    margin-bottom: 6px;
  }

  .item-bottom {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .item-cat {
    font-size: 0.72rem;
    color: #64748b;
    text-transform: capitalize;
  }

  .item-level {
    font-size: 0.68rem;
    font-weight: 700;
  }

  .item-level.critical { color: #f87171; }
  .item-level.high { color: #fb923c; }
  .item-level.medium { color: #fcd34d; }
  .item-level.low { color: #60a5fa; }
  .item-level.favorable { color: #34d399; }

  .empty-list {
    text-align: center;
    padding: 24px;
    color: #64748b;
    font-size: 0.84rem;
  }

  .detail-workspace {
    flex: 1;
    background: #030712;
    overflow: hidden;
  }
</style>
