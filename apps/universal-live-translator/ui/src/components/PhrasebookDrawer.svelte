<script lang="ts">
  import { onMount } from 'svelte';
  import {
    SUPPORTED_LANGUAGES,
    tauriInvoke,
    type LanguageCode,
    type PhraseCategory,
    type PhrasebookItem,
    type GlossaryTerm,
  } from '../types';

  let currentSection: 'phrases' | 'glossary' = 'phrases';
  let selectedCategory: string = 'All';
  let favoriteOnly: boolean = false;
  let phrases: PhrasebookItem[] = [];

  // Add phrase modal state
  let showAddPhrase: boolean = false;
  let newCategory: PhraseCategory = 'Custom';
  let newSourceLang: LanguageCode = 'en';
  let newTargetLang: LanguageCode = 'es';
  let newSourceText: string = '';
  let newTargetText: string = '';

  // Glossary state
  let domains: string[] = [];
  let selectedDomain: string = 'medical';
  let glossaryTerms: GlossaryTerm[] = [];
  let newTermSource: string = '';
  let newTermTarget: string = '';

  const CATEGORIES: PhraseCategory[] = [
    'Greeting',
    'Travel',
    'Dining',
    'Emergency',
    'Business',
    'Medical',
    'Custom',
  ];

  onMount(async () => {
    await loadPhrases();
    await loadGlossaryDomains();
  });

  async function loadPhrases() {
    try {
      const cat = selectedCategory === 'All' ? null : selectedCategory;
      phrases = await tauriInvoke<PhrasebookItem[]>('list_phrases_cmd', {
        categoryStr: cat,
        favoriteOnly,
      });
    } catch (e) {
      console.error('Failed to load phrases:', e);
    }
  }

  async function toggleFavorite(id: string) {
    try {
      await tauriInvoke('toggle_favorite_phrase_cmd', { id });
      await loadPhrases();
    } catch (e) {
      console.error('Toggle favorite failed:', e);
    }
  }

  async function deletePhrase(id: string) {
    try {
      await tauriInvoke('delete_phrase_cmd', { id });
      await loadPhrases();
    } catch (e) {
      console.error('Delete phrase failed:', e);
    }
  }

  async function saveNewPhrase() {
    if (!newSourceText.trim() || !newTargetText.trim()) return;
    try {
      await tauriInvoke('add_phrase_cmd', {
        categoryStr: newCategory,
        sourceLangStr: newSourceLang,
        targetLangStr: newTargetLang,
        sourceText: newSourceText.trim(),
        targetText: newTargetText.trim(),
        notes: null,
      });
      showAddPhrase = false;
      newSourceText = '';
      newTargetText = '';
      await loadPhrases();
    } catch (e) {
      console.error('Failed to add phrase:', e);
    }
  }

  async function loadGlossaryDomains() {
    try {
      domains = await tauriInvoke<string[]>('list_glossary_domains_cmd', {});
      if (domains.length > 0 && !domains.includes(selectedDomain)) {
        selectedDomain = domains[0];
      }
      await loadDomainTerms();
    } catch (e) {
      console.error('Failed to load glossary domains:', e);
    }
  }

  async function loadDomainTerms() {
    if (!selectedDomain) return;
    try {
      glossaryTerms = await tauriInvoke<GlossaryTerm[]>('get_glossary_terms_cmd', {
        domain: selectedDomain,
      });
    } catch (e) {
      console.error('Failed to load terms:', e);
    }
  }

  async function addGlossaryTerm() {
    if (!newTermSource.trim() || !newTermTarget.trim() || !selectedDomain) return;
    try {
      await tauriInvoke('add_glossary_term_cmd', {
        domain: selectedDomain,
        sourceTerm: newTermSource.trim(),
        targetTerm: newTermTarget.trim(),
        caseSensitive: false,
      });
      newTermSource = '';
      newTermTarget = '';
      await loadDomainTerms();
    } catch (e) {
      console.error('Failed to add glossary term:', e);
    }
  }
</script>

<div class="drawer-container">
  <!-- Nav tabs -->
  <div class="nav-bar">
    <div class="tab-group">
      <button
        class="nav-btn"
        class:active={currentSection === 'phrases'}
        on:click={() => (currentSection = 'phrases')}
      >
        📖 Categorized Phrasebook
      </button>
      <button
        class="nav-btn"
        class:active={currentSection === 'glossary'}
        on:click={() => { currentSection = 'glossary'; loadGlossaryDomains(); }}
      >
        📑 Domain Terminology Glossaries
      </button>
    </div>

    {#if currentSection === 'phrases'}
      <button class="primary-btn" on:click={() => (showAddPhrase = true)}>
        + Add Phrase
      </button>
    {/if}
  </div>

  <!-- PHRASEBOOK SECTION -->
  {#if currentSection === 'phrases'}
    <div class="filter-bar">
      <div class="category-pills">
        <button
          class="pill"
          class:active={selectedCategory === 'All'}
          on:click={() => { selectedCategory = 'All'; loadPhrases(); }}
        >
          All
        </button>
        {#each CATEGORIES as cat}
          <button
            class="pill"
            class:active={selectedCategory === cat}
            on:click={() => { selectedCategory = cat; loadPhrases(); }}
          >
            {cat}
          </button>
        {/each}
      </div>

      <label class="fav-toggle">
        <input
          type="checkbox"
          bind:checked={favoriteOnly}
          on:change={loadPhrases}
        />
        <span>★ Favorites Only</span>
      </label>
    </div>

    <!-- Phrase list -->
    <div class="phrase-list">
      {#if phrases.length === 0}
        <div class="empty-state">
          <p>No phrases found. Add customized phrases or bookmark translations for offline reference.</p>
        </div>
      {:else}
        {#each phrases as p}
          <div class="phrase-card">
            <div class="card-left">
              <div class="tags">
                <span class="category-badge">{p.category}</span>
                <span class="lang-badge">{p.sourceLang.toUpperCase()} → {p.targetLang.toUpperCase()}</span>
              </div>
              <div class="phrase-body">
                <p class="source-p">{p.sourceText}</p>
                <p class="target-p">{p.targetText}</p>
              </div>
            </div>
            <div class="card-actions">
              <button
                class="icon-btn fav-btn"
                class:is-fav={p.favorite}
                on:click={() => toggleFavorite(p.id)}
                title="Toggle favorite"
              >
                ★
              </button>
              <button
                class="icon-btn delete-btn"
                on:click={() => deletePhrase(p.id)}
                title="Delete phrase"
              >
                ✕
              </button>
            </div>
          </div>
        {/each}
      {/if}
    </div>
  {/if}

  <!-- GLOSSARY SECTION -->
  {#if currentSection === 'glossary'}
    <div class="glossary-section">
      <div class="glossary-header">
        <div class="domain-picker">
          <span class="label">Select Domain:</span>
          <select bind:value={selectedDomain} on:change={loadDomainTerms} class="select-box">
            {#each domains as dom}
              <option value={dom}>{dom.toUpperCase()}</option>
            {/each}
          </select>
        </div>

        <!-- Add term row -->
        <div class="add-term-row">
          <input
            type="text"
            bind:value={newTermSource}
            placeholder="Source Term (e.g. myocardial infarction)"
            class="input-box"
          />
          <input
            type="text"
            bind:value={newTermTarget}
            placeholder="Target Term (e.g. infarto de miocardio)"
            class="input-box"
          />
          <button class="primary-btn" on:click={addGlossaryTerm}>+ Add Term</button>
        </div>
      </div>

      <div class="terms-table-wrap">
        <table class="terms-table">
          <thead>
            <tr>
              <th>Source Term</th>
              <th>Target Translation</th>
              <th>Domain</th>
            </tr>
          </thead>
          <tbody>
            {#if glossaryTerms.length === 0}
              <tr>
                <td colspan="3" class="empty-cell">No terminology registered for this domain yet.</td>
              </tr>
            {:else}
              {#each glossaryTerms as term}
                <tr>
                  <td class="font-bold">{term.sourceTerm}</td>
                  <td class="font-accent">{term.targetTerm}</td>
                  <td class="font-muted">{selectedDomain}</td>
                </tr>
              {/each}
            {/if}
          </tbody>
        </table>
      </div>
    </div>
  {/if}

  <!-- Add Phrase Modal Dialog -->
  {#if showAddPhrase}
    <div class="modal-backdrop">
      <div class="modal-card">
        <h3 class="modal-title">Add New Offline Phrase</h3>
        <div class="modal-fields">
          <label>
            Category:
            <select bind:value={newCategory} class="select-box">
              {#each CATEGORIES as cat}
                <option value={cat}>{cat}</option>
              {/each}
            </select>
          </label>
          <div class="modal-langs">
            <label>
              Source:
              <select bind:value={newSourceLang} class="select-box">
                {#each SUPPORTED_LANGUAGES as l}
                  <option value={l.code}>{l.displayName}</option>
                {/each}
              </select>
            </label>
            <label>
              Target:
              <select bind:value={newTargetLang} class="select-box">
                {#each SUPPORTED_LANGUAGES as l}
                  <option value={l.code}>{l.displayName}</option>
                {/each}
              </select>
            </label>
          </div>
          <label>
            Source Text:
            <input type="text" bind:value={newSourceText} placeholder="e.g. Where is the nearest hospital?" class="input-box" />
          </label>
          <label>
            Target Text:
            <input type="text" bind:value={newTargetText} placeholder="e.g. ¿Dónde está el hospital más cercano?" class="input-box" />
          </label>
        </div>
        <div class="modal-footer">
          <button class="cancel-btn" on:click={() => (showAddPhrase = false)}>Cancel</button>
          <button class="primary-btn" on:click={saveNewPhrase}>Save Phrase</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .drawer-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 1rem 1.5rem;
    overflow-y: auto;
    gap: 1rem;
  }

  .nav-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--bg-card);
    padding: 0.6rem 1rem;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
  }

  .tab-group {
    display: flex;
    gap: 0.5rem;
  }

  .nav-btn {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--text-muted);
    padding: 0.4rem 0.8rem;
    border-radius: var(--radius-sm);
    font-size: 0.825rem;
    font-weight: 500;
  }

  .nav-btn.active {
    background: var(--accent-primary);
    color: white;
    border-color: var(--accent-primary);
  }

  .primary-btn {
    background: var(--accent-primary);
    border: none;
    color: white;
    padding: 0.4rem 0.9rem;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    font-weight: 500;
    cursor: pointer;
  }

  .filter-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }

  .category-pills {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .pill {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--text-muted);
    padding: 0.3rem 0.65rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    cursor: pointer;
  }

  .pill.active {
    background: rgba(99, 102, 241, 0.2);
    color: #a5b4fc;
    border-color: var(--accent-primary);
  }

  .fav-toggle {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.8rem;
    color: var(--accent-amber);
    cursor: pointer;
    white-space: nowrap;
  }

  .phrase-list {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .phrase-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 0.75rem 1rem;
  }

  .card-left {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .tags {
    display: flex;
    gap: 0.4rem;
  }

  .category-badge, .lang-badge {
    font-size: 0.65rem;
    font-family: var(--font-mono);
    padding: 1px 6px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-muted);
  }

  .source-p {
    font-size: 0.85rem;
    color: var(--text-muted);
  }

  .target-p {
    font-size: 0.95rem;
    font-weight: 500;
    color: var(--text-main);
  }

  .card-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .icon-btn {
    background: transparent;
    border: none;
    font-size: 1rem;
    cursor: pointer;
    color: var(--text-faint);
  }

  .fav-btn.is-fav {
    color: var(--accent-amber);
  }

  .delete-btn:hover {
    color: var(--accent-rose);
  }

  .empty-state {
    text-align: center;
    color: var(--text-faint);
    padding: 2rem;
  }

  .glossary-section {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .glossary-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--bg-card);
    padding: 0.75rem 1rem;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
    gap: 1rem;
  }

  .domain-picker {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .label {
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .select-box, .input-box {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--text-main);
    padding: 0.4rem 0.65rem;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
  }

  .add-term-row {
    display: flex;
    gap: 0.5rem;
  }

  .terms-table-wrap {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .terms-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  .terms-table th {
    background: var(--bg-tertiary);
    text-align: left;
    padding: 0.6rem 1rem;
    color: var(--text-muted);
    font-weight: 600;
    font-size: 0.75rem;
    text-transform: uppercase;
  }

  .terms-table td {
    padding: 0.6rem 1rem;
    border-top: 1px solid var(--border-subtle);
  }

  .font-bold { font-weight: 500; color: var(--text-main); }
  .font-accent { color: var(--accent-cyan); font-weight: 500; }
  .font-muted { color: var(--text-faint); font-family: var(--font-mono); }
  .empty-cell { text-align: center; color: var(--text-faint); padding: 1.5rem; }

  /* Modal */
  .modal-backdrop {
    position: fixed;
    top: 0; left: 0; right: 0; bottom: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .modal-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    padding: 1.5rem;
    width: 440px;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .modal-title {
    font-size: 1rem;
    color: var(--text-main);
  }

  .modal-fields {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .modal-fields label {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .modal-langs {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }

  .cancel-btn {
    background: transparent;
    border: 1px solid var(--border-subtle);
    color: var(--text-muted);
    padding: 0.4rem 0.85rem;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    cursor: pointer;
  }
</style>

