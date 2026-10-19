<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { PasswordStrength, SecretPayload, SecretType } from '../types';
  import EntropyMeter from './EntropyMeter.svelte';

  export let isOpen: boolean = false;
  export let isLoading: boolean = false;

  const dispatch = createEventDispatcher();

  let secretType: SecretType = 'password';
  let title = '';
  let tagsStr = '';
  let username = '';
  let url = '';
  let notes = '';

  // Password fields
  let passwordVal = '';

  // SSH Key fields
  let sshPrivateKey = '';
  let sshPublicKey = '';
  let sshPassphrase = '';

  // Certificate fields
  let certPrivateKey = '';
  let certPem = '';
  let certChain = '';

  // API Key fields
  let apiKeyVal = '';
  let apiSecretVal = '';
  let apiEndpoint = '';

  // Database fields
  let dbEngine = 'postgresql';
  let dbHost = 'localhost';
  let dbPort = 5432;
  let dbName = '';
  let dbUser = '';
  let dbPassword = '';
  let dbConnString = '';

  // Generator settings
  let genLength = 24;
  let genIncludeSymbols = true;
  let genIncludeNumbers = true;
  let genIncludeUppercase = true;

  $: passwordStrength = computeStrength(passwordVal);

  function computeStrength(pwd: string): PasswordStrength {
    const length = pwd.length;
    if (length === 0) {
      return {
        entropy_bits: 0,
        rating: 'very_weak',
        length: 0,
        has_lowercase: false,
        has_uppercase: false,
        has_digits: false,
        has_symbols: false,
      };
    }

    const has_lowercase = /[a-z]/.test(pwd);
    const has_uppercase = /[A-Z]/.test(pwd);
    const has_digits = /[0-9]/.test(pwd);
    const has_symbols = /[^a-zA-Z0-9]/.test(pwd);

    let pool = 0;
    if (has_lowercase) pool += 26;
    if (has_uppercase) pool += 26;
    if (has_digits) pool += 10;
    if (has_symbols) pool += 33;

    const entropy_bits = pool > 0 ? length * Math.log2(pool) : 0;
    let rating: 'very_weak' | 'weak' | 'fair' | 'strong' | 'very_strong' = 'very_weak';
    if (entropy_bits >= 80 && length >= 16) rating = 'very_strong';
    else if (entropy_bits >= 60 && length >= 12) rating = 'strong';
    else if (entropy_bits >= 45 && length >= 8) rating = 'fair';
    else if (entropy_bits >= 28) rating = 'weak';

    return {
      entropy_bits,
      rating,
      length,
      has_lowercase,
      has_uppercase,
      has_digits,
      has_symbols,
    };
  }

  function generateSecurePassword() {
    let charset = 'abcdefghijklmnopqrstuvwxyz';
    if (genIncludeUppercase) charset += 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';
    if (genIncludeNumbers) charset += '0123456789';
    if (genIncludeSymbols) charset += '!@#$%^&*()_+-=[]{}|;:,.<>?';

    const array = new Uint32Array(genLength);
    window.crypto.getRandomValues(array);

    let result = '';
    for (let i = 0; i < genLength; i++) {
      result += charset[array[i] % charset.length];
    }
    passwordVal = result;
  }

  function handleSubmit() {
    if (!title.trim()) return;

    let payload: SecretPayload;

    switch (secretType) {
      case 'password':
        payload = { type: 'password', data: { password: passwordVal } };
        break;
      case 'ssh_key':
        payload = {
          type: 'ssh_key',
          data: {
            private_key: sshPrivateKey,
            public_key: sshPublicKey || null,
            passphrase: sshPassphrase || null,
          },
        };
        break;
      case 'certificate':
        payload = {
          type: 'certificate',
          data: {
            private_key: certPrivateKey,
            certificate_pem: certPem,
            chain_pem: certChain || null,
          },
        };
        break;
      case 'api_key':
        payload = {
          type: 'api_key',
          data: {
            api_key: apiKeyVal,
            api_secret: apiSecretVal || null,
            endpoint_url: apiEndpoint || null,
          },
        };
        break;
      case 'database_credential':
        payload = {
          type: 'database_credential',
          data: {
            engine: dbEngine,
            host: dbHost,
            port: Number(dbPort) || 5432,
            database: dbName,
            username: dbUser,
            password: dbPassword,
            connection_string: dbConnString || null,
          },
        };
        break;
    }

    const tags = tagsStr
      .split(',')
      .map(t => t.trim().toLowerCase())
      .filter(t => t.length > 0);

    dispatch('createSecret', {
      title: title.trim(),
      tags,
      username: username.trim() || null,
      url: url.trim() || null,
      notes: notes.trim() || null,
      payload,
    });
  }

  function closeModal() {
    dispatch('close');
  }
</script>

{#if isOpen}
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions a11y-no-noninteractive-element-interactions -->
  <div class="modal-backdrop" role="dialog" aria-modal="true" tabindex="-1" on:click|self={closeModal} on:keydown={e => { if (e.key === 'Escape') closeModal(); }}>
    <div class="modal-card glass-panel">
      <div class="modal-header">
        <div class="header-left">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" width="20" height="20" class="header-icon">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
          </svg>
          <h2 class="modal-title">Create Encrypted Secret</h2>
        </div>
        <button class="close-btn" on:click={closeModal}>✕</button>
      </div>

      <form on:submit|preventDefault={handleSubmit} class="modal-body">
        <!-- Secret Type Selector -->
        <div class="input-row">
          <label for="secret-type">Secret Type</label>
          <div class="type-selector">
            <button
              type="button"
              class="type-btn {secretType === 'password' ? 'active' : ''}"
              on:click={() => { secretType = 'password'; }}
            >
              Password
            </button>
            <button
              type="button"
              class="type-btn {secretType === 'ssh_key' ? 'active' : ''}"
              on:click={() => { secretType = 'ssh_key'; }}
            >
              SSH Key
            </button>
            <button
              type="button"
              class="type-btn {secretType === 'certificate' ? 'active' : ''}"
              on:click={() => { secretType = 'certificate'; }}
            >
              Certificate
            </button>
            <button
              type="button"
              class="type-btn {secretType === 'api_key' ? 'active' : ''}"
              on:click={() => { secretType = 'api_key'; }}
            >
              API Key
            </button>
            <button
              type="button"
              class="type-btn {secretType === 'database_credential' ? 'active' : ''}"
              on:click={() => { secretType = 'database_credential'; }}
            >
              Database
            </button>
          </div>
        </div>

        <!-- Common Metadata -->
        <div class="grid-2">
          <div class="input-field">
            <label for="sec-title">Title *</label>
            <input
              id="sec-title"
              type="text"
              placeholder="e.g. Production AWS Access"
              bind:value={title}
              required
            />
          </div>

          <div class="input-field">
            <label for="sec-tags">Tags (comma separated)</label>
            <input
              id="sec-tags"
              type="text"
              placeholder="e.g. prod, aws, cloud"
              bind:value={tagsStr}
            />
          </div>
        </div>

        <div class="grid-2">
          <div class="input-field">
            <label for="sec-user">Username / Account</label>
            <input
              id="sec-user"
              type="text"
              placeholder="admin@corp.internal"
              bind:value={username}
            />
          </div>

          <div class="input-field">
            <label for="sec-url">URL / Endpoint</label>
            <input
              id="sec-url"
              type="text"
              placeholder="https://console.aws.amazon.com"
              bind:value={url}
            />
          </div>
        </div>

        <!-- Dynamic Payload Inputs -->
        {#if secretType === 'password'}
          <div class="payload-section">
            <div class="input-field">
              <div class="field-label-row">
                <label for="pwd-val">Password *</label>
                <button type="button" class="gen-link" on:click={generateSecurePassword}>
                  ⚡ Generate Secure (24 chars)
                </button>
              </div>
              <input
                id="pwd-val"
                type="text"
                placeholder="Enter or generate password"
                bind:value={passwordVal}
                class="mono"
                required
              />
            </div>
            <EntropyMeter strength={passwordStrength} />
          </div>

        {:else if secretType === 'ssh_key'}
          <div class="payload-section">
            <div class="input-field">
              <label for="ssh-priv">Private Key *</label>
              <textarea
                id="ssh-priv"
                rows="4"
                placeholder="-----BEGIN OPENSSH PRIVATE KEY-----&#10;...&#10;-----END OPENSSH PRIVATE KEY-----"
                bind:value={sshPrivateKey}
                class="mono"
                required
              ></textarea>
            </div>
            <div class="input-field">
              <label for="ssh-pub">Public Key (Optional)</label>
              <input
                id="ssh-pub"
                type="text"
                placeholder="ssh-ed25519 AAAAC3..."
                bind:value={sshPublicKey}
                class="mono"
              />
            </div>
          </div>

        {:else if secretType === 'certificate'}
          <div class="payload-section">
            <div class="input-field">
              <label for="cert-priv">Private Key PEM *</label>
              <textarea
                id="cert-priv"
                rows="3"
                placeholder="-----BEGIN RSA PRIVATE KEY-----"
                bind:value={certPrivateKey}
                class="mono"
                required
              ></textarea>
            </div>
            <div class="input-field">
              <label for="cert-pem">Certificate PEM *</label>
              <textarea
                id="cert-pem"
                rows="3"
                placeholder="-----BEGIN CERTIFICATE-----"
                bind:value={certPem}
                class="mono"
                required
              ></textarea>
            </div>
          </div>

        {:else if secretType === 'api_key'}
          <div class="payload-section">
            <div class="input-field">
              <label for="api-key">API Key Value *</label>
              <input
                id="api-key"
                type="text"
                placeholder="api_key_xxxxxxxxxxxxx"
                bind:value={apiKeyVal}
                class="mono"
                required
              />
            </div>
            <div class="input-field">
              <label for="api-secret">API Secret (Optional)</label>
              <input
                id="api-secret"
                type="text"
                placeholder="secret_xxxxxxxxxxxx"
                bind:value={apiSecretVal}
                class="mono"
              />
            </div>
          </div>

        {:else if secretType === 'database_credential'}
          <div class="payload-section">
            <div class="grid-3">
              <div class="input-field">
                <label for="db-engine">Engine</label>
                <select id="db-engine" bind:value={dbEngine}>
                  <option value="postgresql">PostgreSQL</option>
                  <option value="mysql">MySQL</option>
                  <option value="sqlite">SQLite</option>
                  <option value="redis">Redis</option>
                  <option value="mongodb">MongoDB</option>
                </select>
              </div>
              <div class="input-field">
                <label for="db-host">Host</label>
                <input id="db-host" type="text" bind:value={dbHost} required />
              </div>
              <div class="input-field">
                <label for="db-port">Port</label>
                <input id="db-port" type="number" bind:value={dbPort} required />
              </div>
            </div>

            <div class="grid-2">
              <div class="input-field">
                <label for="db-name">Database Name</label>
                <input id="db-name" type="text" bind:value={dbName} required />
              </div>
              <div class="input-field">
                <label for="db-user">Database User</label>
                <input id="db-user" type="text" bind:value={dbUser} required />
              </div>
            </div>

            <div class="input-field">
              <label for="db-pwd">Database Password</label>
              <input id="db-pwd" type="password" bind:value={dbPassword} class="mono" required />
            </div>
          </div>
        {/if}

        <div class="input-field">
          <label for="sec-notes">Notes</label>
          <textarea
            id="sec-notes"
            rows="2"
            placeholder="Usage documentation, access instructions..."
            bind:value={notes}
          ></textarea>
        </div>

        <div class="modal-footer">
          <button type="button" class="btn" on:click={closeModal}>Cancel</button>
          <button type="submit" class="btn btn-primary" disabled={isLoading}>
            {#if isLoading}Encrypting...{:else}Encrypt & Save Secret{/if}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(4, 7, 12, 0.75);
    backdrop-filter: blur(16px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
  }

  .modal-card {
    width: 600px;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .header-icon {
    color: var(--accent-emerald);
  }

  .modal-title {
    font-size: 16px;
    font-weight: 700;
    color: #ffffff;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-faint);
    cursor: pointer;
    font-size: 14px;
  }

  .close-btn:hover {
    color: var(--text-main);
  }

  .modal-body {
    padding: 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .type-selector {
    display: flex;
    gap: 6px;
    background: rgba(10, 14, 23, 0.6);
    padding: 4px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
    overflow-x: auto;
  }

  .type-btn {
    flex: 1;
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 500;
    padding: 6px 10px;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.18s ease;
    white-space: nowrap;
  }

  .type-btn:hover {
    color: var(--text-main);
  }

  .type-btn.active {
    background: rgba(255, 255, 255, 0.12);
    color: #ffffff;
    font-weight: 600;
  }

  .grid-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .grid-3 {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 10px;
  }

  .input-field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .input-field label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .field-label-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .gen-link {
    background: transparent;
    border: none;
    color: var(--accent-cyan);
    font-size: 11px;
    cursor: pointer;
    font-weight: 600;
  }

  .gen-link:hover {
    text-decoration: underline;
  }

  .payload-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
  }

  .mono {
    font-family: var(--font-mono);
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 8px;
    padding-top: 14px;
    border-top: 1px solid var(--border-subtle);
  }
</style>
