<script lang="ts">
  import type { FileItem } from '../types';

  export let items: FileItem[];
  export let selectedPaths: string[];
  export let selectedItem: FileItem | null = null;
  export let onNavigateToFolder: (folderPath: string) => void;
  export let onNavigateUp: () => void;
  export let onToggleSelect: (path: string) => void;
  export let onSelectAll: () => void;
  export let onDeselectAll: () => void;
  export let onSelectItem: (item: FileItem | null) => void = () => {};
  export let onInspectFile: (filePath: string) => void = () => {};
  export let onOpenInOs: (path: string) => void = () => {};

  let sortColumn: 'name' | 'type' | 'size' | 'modified' = 'name';
  let sortAsc = true;

  function toggleSort(col: 'name' | 'type' | 'size' | 'modified') {
    if (sortColumn === col) {
      sortAsc = !sortAsc;
    } else {
      sortColumn = col;
      sortAsc = true;
    }
  }

  $: sortedItems = [...items].sort((a, b) => {
    // Keep directories on top unless sorting by size
    if (a.is_dir && !b.is_dir) return -1;
    if (!a.is_dir && b.is_dir) return 1;

    let res = 0;
    if (sortColumn === 'name') {
      res = a.file_name.localeCompare(b.file_name, undefined, { sensitivity: 'base', numeric: true });
    } else if (sortColumn === 'type') {
      const extA = a.extension || (a.is_dir ? 'folder' : '');
      const extB = b.extension || (b.is_dir ? 'folder' : '');
      res = extA.localeCompare(extB);
    } else if (sortColumn === 'size') {
      res = a.size_bytes - b.size_bytes;
    } else if (sortColumn === 'modified') {
      res = a.modified_timestamp_secs - b.modified_timestamp_secs;
    }
    return sortAsc ? res : -res;
  });

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  function formatDate(secs: number): string {
    if (!secs) return '—';
    const d = new Date(secs * 1000);
    return d.toLocaleString(undefined, {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function getExtensionBadgeClass(ext: string | null): string {
    if (!ext) return 'badge-cyan';
    const e = ext.toLowerCase();
    if (['rs', 'ts', 'js', 'py', 'go', 'c', 'cpp'].includes(e)) return 'badge-cyan';
    if (['json', 'toml', 'yaml', 'yml', 'xml'].includes(e)) return 'badge-amber';
    if (['jpg', 'jpeg', 'png', 'svg', 'webp', 'gif'].includes(e)) return 'badge-purple';
    if (['pdf', 'docx', 'doc', 'txt', 'md'].includes(e)) return 'badge-emerald';
    return 'badge-cyan';
  }
</script>

<div class="browser-container glass-panel">
  <!-- Selection Bar -->
  <div class="selection-bar">
    <div class="selection-info">
      <span class="count-badge">{items.length} items</span>
      {#if selectedPaths.length > 0}
        <span class="selected-badge">{selectedPaths.length} selected</span>
      {/if}
      {#if selectedItem}
        <span class="active-preview-badge">Preview: {selectedItem.file_name}</span>
      {/if}
    </div>

    <div class="selection-actions">
      <button class="btn btn-secondary" on:click={onSelectAll}>Select All</button>
      {#if selectedPaths.length > 0}
        <button class="btn btn-secondary" on:click={onDeselectAll}>Clear Selection</button>
      {/if}
    </div>
  </div>

  <!-- Table View -->
  <div class="table-wrapper">
    <table class="file-table">
      <thead>
        <tr>
          <th style="width: 40px;"></th>
          <th class="sortable-th" on:click={() => toggleSort('name')}>
            Name {sortColumn === 'name' ? (sortAsc ? '▲' : '▼') : ''}
          </th>
          <th style="width: 100px;" class="sortable-th" on:click={() => toggleSort('type')}>
            Type {sortColumn === 'type' ? (sortAsc ? '▲' : '▼') : ''}
          </th>
          <th style="width: 120px;" class="sortable-th" on:click={() => toggleSort('size')}>
            Size {sortColumn === 'size' ? (sortAsc ? '▲' : '▼') : ''}
          </th>
          <th style="width: 170px;" class="sortable-th" on:click={() => toggleSort('modified')}>
            Modified {sortColumn === 'modified' ? (sortAsc ? '▲' : '▼') : ''}
          </th>
          <th style="width: 140px; text-align: right;">Actions</th>
        </tr>
      </thead>
      <tbody>
        <!-- Parent Directory Navigation -->
        <tr class="row-clickable parent-row" on:click={onNavigateUp}>
          <td></td>
          <td colspan="5" class="name-cell">
            <svg class="folder-icon" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
            </svg>
            <span class="file-name">.. (Parent Directory)</span>
          </td>
        </tr>

        {#if items.length === 0}
          <tr>
            <td colspan="6" class="empty-state">
              <p>Folder is empty</p>
            </td>
          </tr>
        {:else}
          {#each sortedItems as item (item.path)}
            {@const isChecked = selectedPaths.includes(item.path)}
            {@const isRowActive = selectedItem && selectedItem.path === item.path}
            <tr
              class="row-clickable {isChecked ? 'row-selected' : ''} {isRowActive ? 'row-active' : ''}"
              on:click={() => {
                if (item.is_dir) {
                  onNavigateToFolder(item.path);
                } else {
                  onSelectItem(item);
                }
              }}
              on:dblclick={() => {
                if (item.is_dir) {
                  onNavigateToFolder(item.path);
                } else {
                  onOpenInOs(item.path);
                }
              }}
            >
              <td class="checkbox-cell" on:click|stopPropagation>
                <input
                  type="checkbox"
                  checked={isChecked}
                  on:change={() => onToggleSelect(item.path)}
                />
              </td>

              <td class="name-cell">
                {#if item.is_dir}
                  <svg class="folder-icon" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
                  </svg>
                {:else}
                  <svg class="file-icon" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"></path>
                    <polyline points="13 2 13 9 20 9"></polyline>
                  </svg>
                {/if}
                <span class="file-name" title={item.path}>{item.file_name}</span>
              </td>

              <td>
                {#if item.is_dir}
                  <span class="type-tag folder-tag">Folder</span>
                {:else if item.extension}
                  <span class="badge {getExtensionBadgeClass(item.extension)}">{item.extension.toUpperCase()}</span>
                {:else}
                  <span class="type-tag file-tag">File</span>
                {/if}
              </td>

              <td class="size-cell">
                {item.is_dir ? '—' : formatBytes(item.size_bytes)}
              </td>

              <td class="date-cell">
                {formatDate(item.modified_timestamp_secs)}
              </td>

              <td class="action-cell" on:click|stopPropagation>
                {#if item.is_dir}
                  <button class="mini-action-btn" on:click={() => onNavigateToFolder(item.path)}>Open</button>
                {:else}
                  <button class="mini-action-btn primary" on:click={() => onSelectItem(item)}>Preview</button>
                  <button class="mini-action-btn" on:click={() => onInspectFile(item.path)} title="Inspect Text & Metadata">Inspect</button>
                {/if}
                <button class="mini-action-btn secondary" on:click={() => onOpenInOs(item.path)} title="Open in OS Default Application">OS</button>
              </td>
            </tr>
          {/each}
        {/if}
      </tbody>
    </table>
  </div>
</div>

<style>
  .browser-container {
    display: flex;
    flex-direction: column;
    flex: 1;
    margin: 0 16px 16px 16px;
    overflow: hidden;
  }

  .selection-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    background: rgba(11, 17, 30, 0.4);
    border-bottom: 1px solid var(--border-subtle);
  }

  .selection-info {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .count-badge {
    font-size: 12px;
    color: var(--text-muted);
  }

  .selected-badge {
    background: rgba(6, 182, 212, 0.2);
    color: var(--accent-cyan);
    border: 1px solid rgba(6, 182, 212, 0.4);
    border-radius: 9999px;
    padding: 2px 8px;
    font-size: 11px;
    font-weight: 600;
  }

  .selection-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .table-wrapper {
    flex: 1;
    overflow-y: auto;
  }

  .file-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
    text-align: left;
  }

  .file-table th {
    position: sticky;
    top: 0;
    background: rgba(11, 17, 30, 0.95);
    backdrop-filter: blur(8px);
    color: var(--text-faint);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border-subtle);
    z-index: 10;
  }

  .file-table td {
    padding: 10px 16px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.03);
    color: var(--text-muted);
  }

  .row-clickable {
    cursor: pointer;
    transition: background 0.15s ease;
  }
  .row-clickable:hover {
    background: rgba(255, 255, 255, 0.04);
  }
  .row-selected {
    background: rgba(6, 182, 212, 0.1) !important;
  }
  .row-active {
    background: rgba(59, 130, 246, 0.16) !important;
    border-left: 3px solid var(--accent-cyan);
  }

  .sortable-th {
    cursor: pointer;
    user-select: none;
    transition: color 0.15s ease;
  }
  .sortable-th:hover {
    color: var(--accent-cyan);
  }

  .active-preview-badge {
    background: rgba(59, 130, 246, 0.2);
    color: #93c5fd;
    border: 1px solid rgba(59, 130, 246, 0.4);
    border-radius: 9999px;
    padding: 2px 10px;
    font-size: 11px;
    font-weight: 500;
  }

  .checkbox-cell {
    text-align: center;
  }
  .checkbox-cell input {
    cursor: pointer;
    accent-color: var(--accent-cyan);
  }

  .name-cell {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--text-main);
    font-weight: 500;
  }

  .folder-icon {
    color: var(--accent-amber);
    flex-shrink: 0;
  }

  .file-icon {
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .file-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .type-tag {
    font-size: 11px;
    color: var(--text-faint);
  }
  .folder-tag {
    color: var(--accent-amber);
    font-weight: 500;
  }

  .size-cell, .date-cell {
    font-family: var(--font-mono);
    font-size: 12px;
  }

  .parent-row {
    color: var(--accent-cyan);
    font-style: italic;
  }

  .empty-state {
    text-align: center;
    padding: 40px;
    color: var(--text-faint);
  }

  .action-cell {
    text-align: right;
    white-space: nowrap;
  }

  .mini-action-btn {
    background: rgba(6, 182, 212, 0.14);
    border: 1px solid rgba(6, 182, 212, 0.35);
    color: var(--accent-cyan);
    border-radius: 5px;
    padding: 3px 8px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    margin-left: 4px;
    transition: all 0.15s ease;
  }

  .mini-action-btn:hover {
    background: rgba(6, 182, 212, 0.28);
  }

  .mini-action-btn.secondary {
    background: rgba(255, 255, 255, 0.05);
    border-color: rgba(255, 255, 255, 0.15);
    color: var(--text-muted);
  }

  .mini-action-btn.secondary:hover {
    color: var(--text-main);
    background: rgba(255, 255, 255, 0.12);
  }
</style>
