<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte';
  import {
    SUPPORTED_LANGUAGES,
    tauriInvoke,
    type LanguageCode,
    type PhraseCategory,
    type PhrasebookItem,
    type GlossaryTerm,
    type TranslationResponse,
  } from '../types';

  const dispatch = createEventDispatcher<{
    usePhrase: { sourceText: string; targetText: string; sourceLang: string; targetLang: string };
  }>();

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
  let domains: string[] = ['medical', 'software', 'legal', 'aviation'];
  let selectedDomain: string = 'medical';
  let glossaryTerms: GlossaryTerm[] = [];
  let newTermSource: string = '';
  let newTermTarget: string = '';
  let newDomainInput: string = '';
  let showAddDomain: boolean = false;

  // Live Glossary Override Testing
  let testSentence: string = 'The doctor confirmed an acute myocardial infarction.';
  let testResultText: string = '';
  let isTestingGlossary: boolean = false;

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
      const doms = await tauriInvoke<string[]>('list_glossary_domains_cmd', {});
      if (doms && doms.length > 0) {
        domains = doms;
        if (!domains.includes(selectedDomain)) {
          selectedDomain = domains[0];
        }
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

  async function deleteGlossaryTerm(sourceTerm: string) {
    if (!selectedDomain || !sourceTerm) return;
    try {
      await tauriInvoke('delete_glossary_term_cmd', {
        domain: selectedDomain,
        sourceTerm,
      });
      await loadDomainTerms();
    } catch (e) {
      console.error('Failed to delete glossary term:', e);
    }
  }

  function handleAddCustomDomain() {
    const trimmed = newDomainInput.trim().toLowerCase();
    if (!trimmed) return;
    if (!domains.includes(trimmed)) {
      domains = [...domains, trimmed];
    }
    selectedDomain = trimmed;
    newDomainInput = '';
    showAddDomain = false;
    glossaryTerms = [];
  }

  async function testGlossaryOverride() {
    if (!testSentence.trim() || !selectedDomain) return;
    isTestingGlossary = true;
    testResultText = '';
    try {
      const res = await tauriInvoke<TranslationResponse>('translate_text_cmd', {
        request: {
          text: testSentence.trim(),
          sourceLang: 'en',
          targetLang: 'es',
          preserveFormatting: true,
          domainGlossary: selectedDomain,
        },
      });
      testResultText = res.translatedText;
    } catch (e) {
      console.error('Glossary override test failed:', e);
    } finally {
      isTestingGlossary = false;
    }
  }
</script>

<div class="drawer-container">
  <!-- Nav tabs -->
  <div class="nav-bar">
    <div class="tab-group">
      <button
        type="button"
        class="nav-btn"
        class:active={currentSection === 'phrases'}
        on:click={() => (currentSection = 'phrases')}
      >
        📖 Categorized Phrasebook
      </button>
      <button
        type="button"
        class="nav-btn"
        class:active={currentSection === 'glossary'}
        on:click={() => { currentSection = 'glossary'; loadGlossaryDomains(); }}
      >
        📑 Domain Terminology Glossaries
      </button>
    </div>

    {#if currentSection === 'phrases'}
      <button type="button" class="primary-btn" on:click={() => (showAddPhrase = true)}>
        + Add Phrase
      </button>
    {/if}
  </div>

  <!-- PHRASEBOOK SECTION -->
  {#if currentSection === 'phrases'}
    <div class="filter-bar">
      <div class="category-pills">
        <button
          type="button"
          class="pill"
          class:active={selectedCategory === 'All'}
          on:click={() => { selectedCategory = 'All'; loadPhrases(); }}
        >
          All
        </button>
        {#each CATEGORIES as cat}
          <button
            type="button"
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
                type="button"
                class="use-btn"
                on:click={() => dispatch('usePhrase', {
                  sourceText: p.sourceText,
                  targetText: p.targetText,
                  sourceLang: p.sourceLang,
                  targetLang: p.targetLang,
                })}
                title="Send to Text Translator"
              >
                Use
              </button>
              <button
                type="button"
                class="icon-btn fav-btn"
                class:is-fav={p.favorite}
                on:click={() => toggleFavorite(p.id)}
                title="Toggle favorite"
              >
                ★
              </button>
              <button
                type="button"
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
        <div class="domain-picker-row">
          <div class="domain-picker">
            <span class="label">Active Domain:</span>
            <select bind:value={selectedDomain} on:change={loadDomainTerms} class="select-box">
              {#each domains as dom}
                <option value={dom}>{dom.toUpperCase()}</option>
              {/each}
            </select>
          </div>
          <button
            type="button"
            class="domain-toggle-btn"
            on:click={() => (showAddDomain = !showAddDomain)}
          >
            {showAddDomain ? 'Cancel' : '+ New Domain'}
          </button>
        </div>

        {#if showAddDomain}
          <div class="add-domain-row">
            <input
              type="text"
              bind:value={newDomainInput}
              placeholder="e.g. automotive, fintech, defense"
              class="input-box domain-input"
            />
            <button type="button" class="primary-btn" on:click={handleAddCustomDomain}>Create</button>
          </div>
        {/if}

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
          <button type="button" class="primary-btn" on:click={addGlossaryTerm}>+ Add Term</button>
        </div>
      </div>

      <!-- Terms Table -->
      <div class="terms-table-wrap">
        <table class="terms-table">
          <thead>
            <tr>
              <th>Source Term</th>
              <th>Target Translation</th>
              <th>Domain</th>
              <th>Action</th>
            </tr>
          </thead>
          <tbody>
            {#if glossaryTerms.length === 0}
              <tr>
                <td colspan="4" class="empty-cell">No terminology registered for this domain yet. Add terms above to enforce automatic translation overrides.</td>
              </tr>
            {:else}
              {#each glossaryTerms as term}
                <tr>
                  <td class="font-bold">{term.sourceTerm}</td>
                  <td class="font-accent">{term.targetTerm}</td>
                  <td class="font-muted">{selectedDomain}</td>
                  <td>
                    <button
                      type="button"
                      class="delete-term-btn"
                      on:click={() => deleteGlossaryTerm(term.sourceTerm)}
                      title="Remove glossary term"
                    >
                      ✕
                    </button>
                  </td>
                </tr>
              {/each}
            {/if}
          </tbody>
        </table>
      </div>

      <!-- Live Glossary Override Testing Console -->
      <div class="glossary-test-card">
        <div class="test-header">
          <span class="test-title">⚡ Live Domain Override Verification</span>
          <span class="test-domain-pill">{selectedDomain.toUpperCase()} GLOSSARY ENFORCED</span>
        </div>
        <p class="test-desc">
          Verifies that the NMT engine matches and substitutes registered domain terminology before general machine translation.
        </p>
        <div class="test-row">
          <input
            type="text"
            bind:value={testSentence}
            placeholder="Sentence containing registered terminology..."
            class="input-box test-input"
          />
          <button
            type="button"
            class="primary-btn test-btn"
            disabled={isTestingGlossary}
            on:click={testGlossaryOverride}
          >
            {isTestingGlossary ? 'Translating...' : 'Verify Override'}
          </button>
        </div>

        {#if testResultText}
          <div class="test-result-box">
            <span class="result-label">Translated Output (EN → ES):</span>
            <p class="result-text">{testResultText}</p>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Add Phrase Modal Dialog -->
  {#if showAddPhrase}
    <div class="modal-backdrop" role="dialog" aria-modal="true" tabindex="-1">
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
          <button type="button" class="cancel-btn" on:click={() => (showAddPhrase = false)}>Cancel</button>
          <button type="button" class="primary-btn" on:click={saveNewPhrase}>Save Phrase</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .drawer-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-card);
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
    overflow: hidden;
  }

  .nav-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    background: var(--bg-tertiary);
    border-bottom: 1px solid var(--border-subtle);
  }

  .tab-group {
    display: flex;
    gap: 0.5rem;
  }

  .nav-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 0.85rem;
    font-weight: 500;
    padding: 0.35rem 0.75rem;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .nav-btn.active {
    background: var(--bg-card);
    color: var(--accent-cyan);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
  }

  .primary-btn {
    background: var(--accent-primary);
    color: white;
    border: none;
    padding: 0.4rem 0.85rem;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    font-weight: 500;
    cursor: pointer;
  }

  .primary-btn:hover {
    opacity: 0.9;
  }

  .filter-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.6rem 1rem;
    border-bottom: 1px solid var(--border-subtle);
  }

  .category-pills {
    display: flex;
    gap: 0.35rem;
    overflow-x: auto;
  }

  .pill {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--text-muted);
    font-size: 0.75rem;
    padding: 0.2rem 0.6rem;
    border-radius: 999px;
    cursor: pointer;
  }

  .pill.active {
    background: var(--accent-primary);
    color: white;
    border-color: var(--accent-primary);
  }

  .fav-toggle {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.8rem;
    color: var(--text-muted);
    cursor: pointer;
  }

  .phrase-list {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .empty-state {
    text-align: center;
    color: var(--text-muted);
    padding: 2rem;
    font-size: 0.85rem;
  }

  .phrase-card {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: 0.75rem 1rem;
  }

  .card-left {
    flex: 1;
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
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.05);
    color: var(--text-muted);
  }

  .category-badge {
    color: var(--accent-cyan);
  }

  .phrase-body {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .source-p {
    font-size: 0.85rem;
    color: var(--text-muted);
    margin: 0;
  }

  .target-p {
    font-size: 0.95rem;
    font-weight: 500;
    color: var(--text-main);
    margin: 0;
  }

  .card-actions {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .use-btn {
    background: rgba(56, 189, 248, 0.15);
    border: 1px solid rgba(56, 189, 248, 0.3);
    color: var(--accent-cyan);
    font-size: 0.75rem;
    padding: 2px 8px;
    border-radius: 4px;
    cursor: pointer;
  }

  .use-btn:hover {
    background: var(--accent-cyan);
    color: #0f172a;
  }

  .icon-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 1rem;
    padding: 4px;
  }

  .fav-btn.is-fav {
    color: #eab308;
  }

  .delete-btn:hover {
    color: #ef4444;
  }

  /* Glossary Section */
  .glossary-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 1rem;
    gap: 1rem;
    overflow-y: auto;
  }

  .glossary-header {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    background: var(--bg-tertiary);
    padding: 0.75rem 1rem;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
  }

  .domain-picker-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .domain-picker {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .domain-toggle-btn {
    background: transparent;
    border: 1px solid var(--border-subtle);
    color: var(--accent-cyan);
    font-size: 0.75rem;
    padding: 2px 8px;
    border-radius: 4px;
    cursor: pointer;
  }

  .add-domain-row {
    display: flex;
    gap: 0.5rem;
  }

  .domain-input {
    flex: 1;
  }

  .add-term-row {
    display: grid;
    grid-template-columns: 1fr 1fr auto;
    gap: 0.5rem;
  }

  .input-box {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    color: var(--text-main);
    padding: 0.4rem 0.65rem;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
  }

  .select-box {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    color: var(--text-main);
    padding: 0.35rem 0.65rem;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
  }

  .terms-table-wrap {
    flex: 1;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    overflow: auto;
  }

  .terms-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8rem;
  }

  .terms-table th, .terms-table td {
    padding: 0.5rem 0.75rem;
    text-align: left;
    border-bottom: 1px solid var(--border-subtle);
  }

  .terms-table th {
    background: var(--bg-tertiary);
    color: var(--text-muted);
    font-weight: 500;
  }

  .font-bold { font-weight: 600; color: var(--text-main); }
  .font-accent { color: var(--accent-emerald); }
  .font-muted { color: var(--text-muted); font-size: 0.75rem; }

  .delete-term-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 0.85rem;
  }

  .delete-term-btn:hover {
    color: #ef4444;
  }

  .empty-cell {
    text-align: center;
    color: var(--text-muted);
    padding: 1.5rem !important;
  }

  /* Live Override Testing Card */
  .glossary-test-card {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: 0.85rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .test-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .test-title {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-main);
  }

  .test-domain-pill {
    font-size: 0.65rem;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(16, 185, 129, 0.15);
    color: var(--accent-emerald);
    border: 1px solid rgba(16, 185, 129, 0.3);
  }

  .test-desc {
    font-size: 0.75rem;
    color: var(--text-muted);
    margin: 0;
  }

  .test-row {
    display: flex;
    gap: 0.5rem;
  }

  .test-input {
    flex: 1;
  }

  .test-result-box {
    background: rgba(0, 0, 0, 0.3);
    border-radius: var(--radius-sm);
    padding: 0.5rem 0.75rem;
    margin-top: 0.25rem;
  }

  .result-label {
    font-size: 0.7rem;
    color: var(--text-muted);
    display: block;
    margin-bottom: 2px;
  }

  .result-text {
    margin: 0;
    font-size: 0.85rem;
    color: var(--accent-emerald);
    font-weight: 500;
  }

  /* Modal */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-card {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    width: 440px;
    max-width: 90vw;
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .modal-title {
    margin: 0;
    font-size: 1rem;
    color: var(--text-main);
  }

  .modal-fields {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .modal-fields label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.8rem;
    color: var(--text-muted);
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
