<script lang="ts">
  export let components: any[] = [];
  export let filterQuery: string = '';

  $: filtered = components.filter((c) => {
    if (!filterQuery.trim()) return true;
    const q = filterQuery.toLowerCase();
    return (
      c.name.toLowerCase().includes(q) ||
      c.version.toLowerCase().includes(q) ||
      c.license_expression.toLowerCase().includes(q) ||
      (c.vulnerabilities && c.vulnerabilities.some((v: any) => v.cve_id.toLowerCase().includes(q)))
    );
  });
</script>

<div class="components-table-container">
  <div class="table-header">
    <span class="header-title">COMPONENTS & DEPENDENCY INVENTORY ({filtered.length})</span>
    <input
      type="text"
      placeholder="Filter by name, license, CVE..."
      bind:value={filterQuery}
      class="filter-input"
    />
  </div>

  <div class="table-scroll-wrap">
    <table class="inventory-table">
      <thead>
        <tr>
          <th>Component Name</th>
          <th>Version</th>
          <th>PURL / Identifier</th>
          <th>License</th>
          <th>License Hazard</th>
          <th>Vulnerabilities (CVEs)</th>
        </tr>
      </thead>
      <tbody>
        {#each filtered as c}
          <tr class:has-cve={c.vulnerabilities?.length > 0}>
            <td class="name-cell">
              <span class="pkg-icon">📦</span>
              <span class="name">{c.name}</span>
            </td>
            <td class="mono">{c.version}</td>
            <td class="mono purl-cell" title={c.purl || c.bom_ref}>
              {c.purl || c.bom_ref}
            </td>
            <td class="mono">{c.license_expression}</td>
            <td>
              <span class="hazard-badge" class:copyleft={c.license_risk === 'strong_copyleft'} class:weak={c.license_risk === 'weak_copyleft'}>
                {c.license_risk.replace('_', ' ').toUpperCase()}
              </span>
            </td>
            <td>
              {#if c.vulnerabilities?.length > 0}
                <div class="cve-tags">
                  {#each c.vulnerabilities as v}
                    <span class="cve-pill" class:crit={v.severity === 'critical'} class:high={v.severity === 'high'} title="{v.cve_id}: {v.summary}">
                      {v.cve_id} (CVSS {v.cvss_score.toFixed(1)})
                    </span>
                  {/each}
                </div>
              {:else}
                <span class="clean-pill">✓ Clean</span>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>

<style>
  .components-table-container {
    display: flex;
    flex-direction: column;
    background: #0d121a;
    border: 1px solid #1a2332;
    border-radius: 8px;
    overflow: hidden;
    flex: 1;
  }

  .table-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: #141b26;
    border-bottom: 1px solid #1a2332;
    font-family: monospace;
  }

  .header-title {
    font-size: 0.72rem;
    font-weight: 700;
    color: #94a3b8;
  }

  .filter-input {
    background: #090d14;
    border: 1px solid #28374d;
    color: #f1f5f9;
    padding: 3px 8px;
    border-radius: 4px;
    font-family: inherit;
    font-size: 0.72rem;
    width: 220px;
    outline: none;
  }

  .table-scroll-wrap {
    flex: 1;
    overflow: auto;
    max-height: 380px;
  }

  .inventory-table {
    width: 100%;
    border-collapse: collapse;
    font-family: monospace;
    font-size: 0.72rem;
    text-align: left;
  }

  th {
    background: #111722;
    color: #94a3b8;
    padding: 6px 12px;
    font-weight: 600;
    border-bottom: 1px solid #1a2332;
    position: sticky;
    top: 0;
    z-index: 10;
  }

  td {
    padding: 6px 12px;
    border-bottom: 1px solid #141c28;
    color: #f1f5f9;
    white-space: nowrap;
  }

  tr:hover td {
    background: rgba(248, 113, 113, 0.05);
  }

  tr.has-cve td {
    background: rgba(239, 68, 68, 0.08);
  }

  .name-cell {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
  }

  .mono {
    font-family: monospace;
  }

  .purl-cell {
    max-width: 240px;
    overflow: hidden;
    text-overflow: ellipsis;
    color: #64748b;
  }

  .hazard-badge {
    background: rgba(16, 185, 129, 0.15);
    border: 1px solid #10b98144;
    color: #34d399;
    padding: 1px 6px;
    border-radius: 3px;
    font-size: 0.65rem;
    font-weight: 600;
  }

  .hazard-badge.copyleft {
    background: rgba(239, 68, 68, 0.15);
    border-color: #ef4444;
    color: #f87171;
  }

  .hazard-badge.weak {
    background: rgba(245, 158, 11, 0.15);
    border-color: #f59e0b;
    color: #fbbf24;
  }

  .cve-tags {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }

  .cve-pill {
    background: rgba(239, 68, 68, 0.2);
    border: 1px solid #ef4444;
    color: #fca5a5;
    padding: 1px 5px;
    border-radius: 3px;
    font-size: 0.65rem;
    font-weight: 700;
  }

  .cve-pill.crit {
    background: #7f1d1d;
    border-color: #ef4444;
    color: #ffffff;
  }

  .clean-pill {
    color: #10b981;
    font-size: 0.68rem;
  }
</style>
