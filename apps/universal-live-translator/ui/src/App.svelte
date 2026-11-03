<script lang="ts">
  import Header from './components/Header.svelte';
  import TextTranslatorView from './components/TextTranslatorView.svelte';
  import VoiceConversationView from './components/VoiceConversationView.svelte';
  import CameraDocumentView from './components/CameraDocumentView.svelte';
  import PhrasebookDrawer from './components/PhrasebookDrawer.svelte';
  import type { LanguageCode } from './types';

  let activeTab: 'text' | 'voice' | 'camera' | 'phrasebook' = 'text';
  let preloadText: string = '';
  let preloadSourceLang: 'auto' | LanguageCode = 'auto';
  let preloadTargetLang: LanguageCode = 'es';

  function handleTabChange(e: CustomEvent<'text' | 'voice' | 'camera' | 'phrasebook'>) {
    activeTab = e.detail;
  }

  function handleUsePhrase(e: CustomEvent<{ sourceText: string; targetText: string; sourceLang: string; targetLang: string }>) {
    preloadText = e.detail.sourceText;
    if (e.detail.sourceLang) preloadSourceLang = e.detail.sourceLang as 'auto' | LanguageCode;
    if (e.detail.targetLang) preloadTargetLang = e.detail.targetLang as LanguageCode;
    activeTab = 'text';
  }
</script>

<div class="app-layout">
  <Header {activeTab} on:changeTab={handleTabChange} />

  <main class="main-content">
    {#if activeTab === 'text'}
      <TextTranslatorView
        initialText={preloadText}
        initialSourceLang={preloadSourceLang}
        initialTargetLang={preloadTargetLang}
      />
    {:else if activeTab === 'voice'}
      <VoiceConversationView />
    {:else if activeTab === 'camera'}
      <CameraDocumentView />
    {:else if activeTab === 'phrasebook'}
      <PhrasebookDrawer on:usePhrase={handleUsePhrase} />
    {/if}
  </main>
</div>

<style>
  .app-layout {
    width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: transparent;
  }
</style>

