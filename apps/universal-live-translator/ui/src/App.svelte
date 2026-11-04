<script lang="ts">
  import Header from './components/Header.svelte';
  import TextTranslatorView from './components/TextTranslatorView.svelte';
  import VoiceConversationView from './components/VoiceConversationView.svelte';
  import CameraDocumentView from './components/CameraDocumentView.svelte';
  import PhrasebookDrawer from './components/PhrasebookDrawer.svelte';
  import CommandPalette, { type CommandAction } from './components/CommandPalette.svelte';
  import type { LanguageCode } from './types';

  let activeTab: 'text' | 'voice' | 'camera' | 'phrasebook' = 'text';
  let preloadText: string = '';
  let preloadSourceLang: 'auto' | LanguageCode = 'auto';
  let preloadTargetLang: LanguageCode = 'es';
  let isCommandPaletteOpen = false;

  $: commandActions = [
    {
      id: 'tab-text',
      title: 'Text Translation Mode',
      subtitle: 'Neural MT with language identification & glossary support',
      category: 'Mode',
      icon: '✍️',
      shortcut: 'Ctrl+1',
      run: () => { activeTab = 'text'; },
    },
    {
      id: 'tab-voice',
      title: 'Voice & Dual-Conversation',
      subtitle: 'Whisper offline streaming ASR and two-party dialog',
      category: 'Mode',
      icon: '🎙️',
      shortcut: 'Ctrl+2',
      run: () => { activeTab = 'voice'; },
    },
    {
      id: 'tab-camera',
      title: 'Camera & Document OCR',
      subtitle: 'Live image and document text extraction translation',
      category: 'Mode',
      icon: '📷',
      shortcut: 'Ctrl+3',
      run: () => { activeTab = 'camera'; },
    },
    {
      id: 'tab-phrasebook',
      title: 'Offline Phrasebook & Glossary',
      subtitle: 'Custom offline domain dictionaries and saved phrases',
      category: 'Mode',
      icon: '📖',
      shortcut: 'Ctrl+4',
      run: () => { activeTab = 'phrasebook'; },
    },
    {
      id: 'pair-es',
      title: 'Target: Spanish (Español)',
      subtitle: 'Set target language to Spanish',
      category: 'Language Pair',
      icon: '🇪🇸',
      shortcut: '',
      run: () => { preloadTargetLang = 'es'; activeTab = 'text'; },
    },
    {
      id: 'pair-de',
      title: 'Target: German (Deutsch)',
      subtitle: 'Set target language to German',
      category: 'Language Pair',
      icon: '🇩🇪',
      shortcut: '',
      run: () => { preloadTargetLang = 'de'; activeTab = 'text'; },
    },
    {
      id: 'pair-fr',
      title: 'Target: French (Français)',
      subtitle: 'Set target language to French',
      category: 'Language Pair',
      icon: '🇫🇷',
      shortcut: '',
      run: () => { preloadTargetLang = 'fr'; activeTab = 'text'; },
    },
    {
      id: 'pair-ja',
      title: 'Target: Japanese (日本語)',
      subtitle: 'Set target language to Japanese',
      category: 'Language Pair',
      icon: '🇯🇵',
      shortcut: '',
      run: () => { preloadTargetLang = 'ja'; activeTab = 'text'; },
    },
    {
      id: 'pair-zh',
      title: 'Target: Chinese (中文)',
      subtitle: 'Set target language to Simplified Chinese',
      category: 'Language Pair',
      icon: '🇨🇳',
      shortcut: '',
      run: () => { preloadTargetLang = 'zh'; activeTab = 'text'; },
    },
    {
      id: 'pair-ta',
      title: 'Target: Tamil (தமிழ்)',
      subtitle: 'Set target language to Tamil',
      category: 'Language Pair',
      icon: '🇮🇳',
      shortcut: '',
      run: () => { preloadTargetLang = 'ta'; activeTab = 'text'; },
    },
    {
      id: 'pair-ru',
      title: 'Target: Russian (Русский)',
      subtitle: 'Set target language to Russian',
      category: 'Language Pair',
      icon: '🇷🇺',
      shortcut: '',
      run: () => { preloadTargetLang = 'ru'; activeTab = 'text'; },
    },
    {
      id: 'act-clear',
      title: 'Clear Current Input',
      subtitle: 'Reset translation text box',
      category: 'Actions',
      icon: '🧹',
      shortcut: 'Ctrl+Shift+X',
      run: () => { preloadText = ''; activeTab = 'text'; },
    },
    {
      id: 'act-swap',
      title: 'Swap Languages',
      subtitle: 'Invert source and target language pairs',
      category: 'Actions',
      icon: '🔁',
      shortcut: 'Ctrl+S',
      run: () => {
        if (preloadSourceLang !== 'auto') {
          const temp = preloadSourceLang;
          preloadSourceLang = preloadTargetLang;
          preloadTargetLang = temp;
        }
      },
    },
  ];

  function handleTabChange(e: CustomEvent<'text' | 'voice' | 'camera' | 'phrasebook'>) {
    activeTab = e.detail;
  }

  function handleUsePhrase(e: CustomEvent<{ sourceText: string; targetText: string; sourceLang: string; targetLang: string }>) {
    preloadText = e.detail.sourceText;
    if (e.detail.sourceLang) preloadSourceLang = e.detail.sourceLang as 'auto' | LanguageCode;
    if (e.detail.targetLang) preloadTargetLang = e.detail.targetLang as LanguageCode;
    activeTab = 'text';
  }

  function handleGlobalKeyDown(e: KeyboardEvent) {
    const isCtrl = e.ctrlKey || e.metaKey;

    if (isCtrl && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      isCommandPaletteOpen = !isCommandPaletteOpen;
      return;
    }

    if (e.key === 'Escape') {
      if (isCommandPaletteOpen) {
        e.preventDefault();
        isCommandPaletteOpen = false;
        return;
      }
      if (activeTab === 'phrasebook') {
        e.preventDefault();
        activeTab = 'text';
        return;
      }
      return;
    }

    // Ignore single keys if typing in an input
    const activeTagName = (document.activeElement?.tagName || '').toLowerCase();
    const isInputActive = activeTagName === 'input' || activeTagName === 'textarea';
    if (isInputActive) return;

    if (isCtrl && e.key === '1') {
      e.preventDefault();
      activeTab = 'text';
    } else if (isCtrl && e.key === '2') {
      e.preventDefault();
      activeTab = 'voice';
    } else if (isCtrl && e.key === '3') {
      e.preventDefault();
      activeTab = 'camera';
    } else if (isCtrl && e.key === '4') {
      e.preventDefault();
      activeTab = 'phrasebook';
    } else if (isCtrl && e.key.toLowerCase() === 's') {
      e.preventDefault();
      if (preloadSourceLang !== 'auto') {
        const temp = preloadSourceLang;
        preloadSourceLang = preloadTargetLang;
        preloadTargetLang = temp;
      }
    }
  }
</script>

<svelte:window on:keydown={handleGlobalKeyDown} />

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

  <!-- Global Command Palette -->
  <CommandPalette
    isOpen={isCommandPaletteOpen}
    actions={commandActions}
    on:close={() => (isCommandPaletteOpen = false)}
  />
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

