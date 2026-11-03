<script lang="ts">
  import { onMount } from 'svelte';
  import {
    SUPPORTED_LANGUAGES,
    tauriInvoke,
    type LanguageCode,
    type ConversationSession,
    type ConversationTurn,
    type TtsSynthesisRequest,
    type TtsSynthesisResponse,
  } from '../types';

  let partyALang: LanguageCode = 'en';
  let partyBLang: LanguageCode = 'es';

  let currentSession: ConversationSession | null = null;
  let turns: ConversationTurn[] = [];

  let inputPartyA: string = '';
  let inputPartyB: string = '';
  let activeSpeaker: 'none' | 'party_a' | 'party_b' = 'none';

  onMount(async () => {
    await startSession();
  });

  async function startSession() {
    try {
      currentSession = await tauriInvoke<ConversationSession>('start_conversation_session_cmd', {
        partyALangStr: partyALang,
        partyBLangStr: partyBLang,
      });
      turns = currentSession.turns || [];
    } catch (e) {
      console.error('Failed to start conversation session:', e);
    }
  }

  async function submitTurn(speaker: 'party_a' | 'party_b') {
    if (!currentSession) return;
    const text = (speaker === 'party_a' ? inputPartyA : inputPartyB).trim();
    if (!text) return;

    activeSpeaker = speaker;
    try {
      const turn = await tauriInvoke<ConversationTurn>('add_conversation_turn_cmd', {
        sessionId: currentSession.id,
        speaker,
        text,
      });

      turns = [...turns, turn];
      if (speaker === 'party_a') inputPartyA = '';
      else inputPartyB = '';

      // Auto speak translated response
      await speakTurn(turn);
    } catch (e) {
      console.error('Failed to process conversation turn:', e);
    } finally {
      activeSpeaker = 'none';
    }
  }

  async function speakTurn(turn: ConversationTurn) {
    try {
      const req: TtsSynthesisRequest = {
        text: turn.translatedText,
        language: turn.targetLang,
        speakingRate: 1.0,
      };
      const res = await tauriInvoke<TtsSynthesisResponse>('synthesize_speech_cmd', { request: req });
      if (res && res.audioWavBase64) {
        const audio = new Audio(`data:audio/wav;base64,${res.audioWavBase64}`);
        await audio.play();
      }
    } catch (e) {
      console.error('Error speaking turn:', e);
    }
  }
</script>

<div class="conversation-container">
  <!-- Conversation Control Bar -->
  <div class="conv-controls">
    <div class="party-setup">
      <span class="party-label">Party A:</span>
      <select bind:value={partyALang} on:change={startSession} class="select-pill">
        {#each SUPPORTED_LANGUAGES as lang}
          <option value={lang.code}>{lang.displayName}</option>
        {/each}
      </select>
    </div>

    <div class="conv-badge">
      <span class="pulse-dot"></span>
      <span>Live Split-Screen Interpreter</span>
    </div>

    <div class="party-setup">
      <span class="party-label">Party B:</span>
      <select bind:value={partyBLang} on:change={startSession} class="select-pill">
        {#each SUPPORTED_LANGUAGES as lang}
          <option value={lang.code}>{lang.displayName}</option>
        {/each}
      </select>
    </div>
  </div>

  <!-- Chat stream -->
  <div class="turns-stream">
    {#if turns.length === 0}
      <div class="empty-state">
        <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path>
        </svg>
        <p>No conversation yet. Speak or type from either party below to begin real-time translation.</p>
      </div>
    {:else}
      {#each turns as turn}
        <div class="turn-bubble" class:turn-a={turn.speaker === 'party_a'} class:turn-b={turn.speaker === 'party_b'}>
          <div class="turn-meta">
            <span class="turn-speaker">{turn.speaker === 'party_a' ? 'Party A' : 'Party B'}</span>
            <span class="turn-lang">{turn.sourceLang.toUpperCase()} → {turn.targetLang.toUpperCase()}</span>
          </div>
          <div class="turn-original">{turn.originalText}</div>
          <div class="turn-translated">
            <span>{turn.translatedText}</span>
            <button class="tts-btn" on:click={() => speakTurn(turn)} title="Listen to translation">
              🔊
            </button>
          </div>
        </div>
      {/each}
    {/if}
  </div>

  <!-- Split Inputs for Party A and Party B -->
  <div class="split-inputs">
    <!-- Party A Input -->
    <div class="party-input-box box-a">
      <div class="box-header">
        <span class="box-title">Party A ({partyALang.toUpperCase()})</span>
      </div>
      <div class="input-row">
        <input
          type="text"
          bind:value={inputPartyA}
          placeholder="Speak or type English..."
          on:keydown={(e) => e.key === 'Enter' && submitTurn('party_a')}
          class="turn-input"
        />
        <button
          class="send-btn"
          disabled={!inputPartyA.trim() || activeSpeaker === 'party_a'}
          on:click={() => submitTurn('party_a')}
        >
          {activeSpeaker === 'party_a' ? '...' : 'Speak / Send'}
        </button>
      </div>
    </div>

    <!-- Party B Input -->
    <div class="party-input-box box-b">
      <div class="box-header">
        <span class="box-title">Party B ({partyBLang.toUpperCase()})</span>
      </div>
      <div class="input-row">
        <input
          type="text"
          bind:value={inputPartyB}
          placeholder="Hablar o escribir español..."
          on:keydown={(e) => e.key === 'Enter' && submitTurn('party_b')}
          class="turn-input"
        />
        <button
          class="send-btn"
          disabled={!inputPartyB.trim() || activeSpeaker === 'party_b'}
          on:click={() => submitTurn('party_b')}
        >
          {activeSpeaker === 'party_b' ? '...' : 'Speak / Send'}
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .conversation-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 1rem 1.5rem;
    overflow: hidden;
    gap: 0.75rem;
  }

  .conv-controls {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--bg-card);
    padding: 0.6rem 1.25rem;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
  }

  .party-setup {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .party-label {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-muted);
  }

  .select-pill {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--text-main);
    padding: 0.35rem 0.65rem;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    cursor: pointer;
  }

  .conv-badge {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.75rem;
    color: #a5b4fc;
    font-weight: 500;
  }

  .pulse-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent-primary);
    box-shadow: 0 0 6px var(--accent-primary);
  }

  .turns-stream {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 0.5rem;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-faint);
    gap: 0.75rem;
    text-align: center;
  }

  .turn-bubble {
    max-width: 75%;
    padding: 0.75rem 1rem;
    border-radius: var(--radius-md);
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .turn-a {
    align-self: flex-start;
    background: rgba(99, 102, 241, 0.12);
    border: 1px solid rgba(99, 102, 241, 0.25);
  }

  .turn-b {
    align-self: flex-end;
    background: rgba(6, 182, 212, 0.12);
    border: 1px solid rgba(6, 182, 212, 0.25);
  }

  .turn-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 0.7rem;
    color: var(--text-muted);
  }

  .turn-original {
    font-size: 0.85rem;
    color: var(--text-muted);
  }

  .turn-translated {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 0.95rem;
    font-weight: 500;
    color: var(--text-main);
    gap: 0.5rem;
  }

  .tts-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    font-size: 0.85rem;
    opacity: 0.8;
  }

  .tts-btn:hover {
    opacity: 1;
  }

  .split-inputs {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    padding-top: 0.5rem;
  }

  .party-input-box {
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .box-a { border-top: 3px solid var(--accent-primary); }
  .box-b { border-top: 3px solid var(--accent-cyan); }

  .box-title {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .input-row {
    display: flex;
    gap: 0.5rem;
  }

  .turn-input {
    flex: 1;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    color: var(--text-main);
    padding: 0.45rem 0.75rem;
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
    outline: none;
  }

  .turn-input:focus {
    border-color: var(--accent-primary);
  }

  .send-btn {
    background: var(--accent-primary);
    border: none;
    color: white;
    padding: 0.45rem 0.85rem;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
  }

  .send-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>

