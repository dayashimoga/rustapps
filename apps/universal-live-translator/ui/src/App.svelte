<script lang="ts">
  import Header from './components/Header.svelte';
  import TextTranslatorView from './components/TextTranslatorView.svelte';
  import VoiceConversationView from './components/VoiceConversationView.svelte';
  import CameraDocumentView from './components/CameraDocumentView.svelte';
  import PhrasebookDrawer from './components/PhrasebookDrawer.svelte';

  let activeTab: 'text' | 'voice' | 'camera' | 'phrasebook' = 'text';

  function handleTabChange(e: CustomEvent<'text' | 'voice' | 'camera' | 'phrasebook'>) {
    activeTab = e.detail;
  }
</script>

<div class="app-layout">
  <Header {activeTab} on:changeTab={handleTabChange} />

  <main class="main-content">
    {#if activeTab === 'text'}
      <TextTranslatorView />
    {:else if activeTab === 'voice'}
      <VoiceConversationView />
    {:else if activeTab === 'camera'}
      <CameraDocumentView />
    {:else if activeTab === 'phrasebook'}
      <PhrasebookDrawer />
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

