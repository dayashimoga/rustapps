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

  // Cloud & Advanced fields
  let awsAccessKey = '';
  let awsSecretKey = '';
  let awsSessionToken = '';
  let awsRegion = 'us-east-1';
  let awsProfile = 'default';

  let azSubId = '';
  let azTenantId = '';
  let azClientId = '';
  let azClientSecret = '';
  let azCloudEnv = 'AzureCloud';

  let tfStateKey = '';
  let tfVars = '';
  let tfToken = '';
  let tfBackend = '';

  // Quick import state for cloud credentials
  let showAwsIniPaste = false;
  let awsIniInput = '';
  let awsParseError = '';

  let showAzJsonPaste = false;
  let azJsonInput = '';
  let azParseError = '';

  let showTfvarsParser = false;
  let tfvarsParseResult: Record<string, string> | null = null;
  let parsedTfvarsCount = 0;

  async function invokeIpc<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
    const tauri = (window as unknown as { __TAURI__?: { invoke?: (c: string, a: Record<string, unknown>) => Promise<T> } }).__TAURI__;
    if (tauri?.invoke) {
      return await tauri.invoke(cmd, args);
    }
    if (typeof fetch !== 'undefined' && typeof window !== 'undefined' && window.location && window.location.protocol.startsWith('http')) {
      const res = await fetch('/__ipc/' + encodeURIComponent(cmd), {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(args || {}),
      });
      if (res.ok) {
        const payload = await res.json();
        if (payload && '__ipc_data' in payload) return payload.__ipc_data as T;
        return payload as T;
      }
    }
    throw new Error('IPC bridge not available');
  }

  async function handleParseAwsIni() {
    awsParseError = '';
    if (!awsIniInput.trim()) return;
    try {
      let creds: Array<{
        access_key_id: string;
        secret_access_key: string;
        session_token?: string | null;
        region?: string | null;
        profile_name?: string | null;
      }>;

      try {
        creds = await invokeIpc('parse_aws_ini', { content: awsIniInput });
      } catch {
        const lines = awsIniInput.split('\n');
        let ak = '';
        let sk = '';
        let reg = 'us-east-1';
        let prof = 'default';
        for (const line of lines) {
          const t = line.trim();
          if (t.startsWith('[') && t.endsWith(']')) {
            prof = t.slice(1, -1).replace(/^profile\s+/, '').trim();
          } else if (t.includes('=')) {
            const [k, v] = t.split('=').map((s) => s.trim().replace(/['"]/g, ''));
            if (/aws_access_key_id/i.test(k) || /access_key_id/i.test(k)) ak = v;
            if (/aws_secret_access_key/i.test(k) || /secret_access_key/i.test(k)) sk = v;
            if (/region/i.test(k)) reg = v;
          }
        }
        if (!ak || !sk) throw new Error('No AWS credentials found in INI text');
        creds = [{ access_key_id: ak, secret_access_key: sk, region: reg, profile_name: prof }];
      }

      if (creds && creds.length > 0) {
        const first = creds[0];
        awsAccessKey = first.access_key_id;
        awsSecretKey = first.secret_access_key;
        if (first.session_token) awsSessionToken = first.session_token;
        if (first.region) awsRegion = first.region;
        if (first.profile_name) awsProfile = first.profile_name;
        if (!title.trim()) title = `AWS Credentials (${first.profile_name || 'default'})`;
        showAwsIniPaste = false;
      }
    } catch (e: unknown) {
      const err = e as { message?: string };
      awsParseError = err.message || String(e);
    }
  }

  async function handleParseAzureJson() {
    azParseError = '';
    if (!azJsonInput.trim()) return;
    try {
      let cred: {
        subscription_id: string;
        tenant_id: string;
        client_id: string;
        client_secret: string;
      };

      try {
        cred = await invokeIpc('parse_azure_json', {
          jsonStr: azJsonInput,
          subscriptionId: azSubId.trim() || '00000000-0000-0000-0000-000000000000',
        });
      } catch {
        const obj = JSON.parse(azJsonInput);
        cred = {
          subscription_id: azSubId.trim() || '00000000-0000-0000-0000-000000000000',
          tenant_id: obj.tenant || obj.tenantId || '',
          client_id: obj.appId || obj.clientId || '',
          client_secret: obj.password || obj.clientSecret || '',
        };
      }

      if (cred.client_id) azClientId = cred.client_id;
      if (cred.client_secret) azClientSecret = cred.client_secret;
      if (cred.tenant_id) azTenantId = cred.tenant_id;
      if (cred.subscription_id && cred.subscription_id !== '00000000-0000-0000-0000-000000000000') {
        azSubId = cred.subscription_id;
      }
      if (!title.trim()) title = `Azure Service Principal (${cred.client_id.slice(0, 8)})`;
      showAzJsonPaste = false;
    } catch (e: unknown) {
      const err = e as { message?: string };
      azParseError = err.message || String(e);
    }
  }

  async function handleParseTfvars() {
    if (!tfVars.trim()) {
      tfvarsParseResult = null;
      parsedTfvarsCount = 0;
      return;
    }
    try {
      let vars: Record<string, string>;
      try {
        vars = await invokeIpc('parse_terraform_vars', { content: tfVars });
      } catch {
        vars = {};
        for (const line of tfVars.split('\n')) {
          const t = line.trim();
          if (t && !t.startsWith('#') && !t.startsWith('//') && t.includes('=')) {
            const [k, v] = t.split('=').map((s) => s.trim().replace(/^['"]|['"]$/g, ''));
            vars[k] = v;
          }
        }
      }
      tfvarsParseResult = vars;
      parsedTfvarsCount = Object.keys(vars).length;
    } catch (e: unknown) {
      console.error('TFvars parsing error:', e);
    }
  }

  let totpSecretBase32 = '';
  let totpIssuer = '';
  let totpAccount = '';
  let totpDigits = 6;
  let totpPeriod = 30;

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
      case 'aws_credential':
        payload = {
          type: 'aws_credential',
          data: {
            access_key_id: awsAccessKey,
            secret_access_key: awsSecretKey,
            session_token: awsSessionToken || null,
            region: awsRegion || null,
            profile_name: awsProfile || null,
          },
        };
        break;
      case 'azure_credential':
        payload = {
          type: 'azure_credential',
          data: {
            subscription_id: azSubId,
            tenant_id: azTenantId,
            client_id: azClientId,
            client_secret: azClientSecret,
            cloud_environment: azCloudEnv || null,
          },
        };
        break;
      case 'terraform_secret':
        payload = {
          type: 'terraform_secret',
          data: {
            state_key: tfStateKey || null,
            tf_vars: tfVars || null,
            provider_token: tfToken || null,
            backend_config: tfBackend || null,
          },
        };
        break;
      case 'totp':
        payload = {
          type: 'totp',
          data: {
            secret_base32: totpSecretBase32,
            issuer: totpIssuer || null,
            account_name: totpAccount || null,
            algorithm: 'SHA1',
            digits: Number(totpDigits) || 6,
            period_seconds: Number(totpPeriod) || 30,
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
            <button
              type="button"
              class="type-btn {secretType === 'aws_credential' ? 'active' : ''}"
              on:click={() => { secretType = 'aws_credential'; }}
            >
              AWS
            </button>
            <button
              type="button"
              class="type-btn {secretType === 'azure_credential' ? 'active' : ''}"
              on:click={() => { secretType = 'azure_credential'; }}
            >
              Azure
            </button>
            <button
              type="button"
              class="type-btn {secretType === 'terraform_secret' ? 'active' : ''}"
              on:click={() => { secretType = 'terraform_secret'; }}
            >
              Terraform
            </button>
            <button
              type="button"
              class="type-btn {secretType === 'totp' ? 'active' : ''}"
              on:click={() => { secretType = 'totp'; }}
            >
              2FA / TOTP
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

        {:else if secretType === 'aws_credential'}
          <div class="payload-section">
            <div class="quick-import-panel">
              <div class="quick-import-header">
                <span class="quick-import-title">Import from ~/.aws/credentials (INI)</span>
                <button
                  type="button"
                  class="quick-toggle-btn"
                  on:click={() => (showAwsIniPaste = !showAwsIniPaste)}
                >
                  {showAwsIniPaste ? 'Hide Paste Box' : '📋 Paste INI'}
                </button>
              </div>
              {#if showAwsIniPaste}
                <textarea
                  class="quick-import-area mono"
                  placeholder="[default]&#10;aws_access_key_id = AKIA...&#10;aws_secret_access_key = wJalr...&#10;region = us-east-1"
                  bind:value={awsIniInput}
                  rows="3"
                ></textarea>
                <div class="import-actions-row">
                  <button type="button" class="btn-import" on:click={handleParseAwsIni}>
                    Parse & Populate AWS Fields
                  </button>
                  {#if awsParseError}
                    <span class="parse-err">{awsParseError}</span>
                  {/if}
                </div>
              {/if}
            </div>

            <div class="grid-2">
              <div class="input-field">
                <label for="aws-key">Access Key ID *</label>
                <input id="aws-key" type="text" placeholder="AKIA..." bind:value={awsAccessKey} class="mono" required />
              </div>
              <div class="input-field">
                <label for="aws-secret">Secret Access Key *</label>
                <input id="aws-secret" type="password" placeholder="wJalrXUtn..." bind:value={awsSecretKey} class="mono" required />
              </div>
            </div>
            <div class="grid-2">
              <div class="input-field">
                <label for="aws-region">Region</label>
                <input id="aws-region" type="text" placeholder="us-east-1" bind:value={awsRegion} />
              </div>
              <div class="input-field">
                <label for="aws-profile">Profile Name</label>
                <input id="aws-profile" type="text" placeholder="default" bind:value={awsProfile} />
              </div>
            </div>
            <div class="input-field">
              <label for="aws-token">Session Token (Optional STS)</label>
              <textarea id="aws-token" rows="2" placeholder="AQoDYXdz..." bind:value={awsSessionToken} class="mono"></textarea>
            </div>
          </div>

        {:else if secretType === 'azure_credential'}
          <div class="payload-section">
            <div class="quick-import-panel">
              <div class="quick-import-header">
                <span class="quick-import-title">Import from az ad sp create-for-rbac (JSON)</span>
                <button
                  type="button"
                  class="quick-toggle-btn"
                  on:click={() => (showAzJsonPaste = !showAzJsonPaste)}
                >
                  {showAzJsonPaste ? 'Hide Paste Box' : '📋 Paste JSON'}
                </button>
              </div>
              {#if showAzJsonPaste}
                <textarea
                  class="quick-import-area mono"
                  placeholder="&#123;&#10;  &quot;appId&quot;: &quot;00000000-0000-0000-0000-000000000000&quot;,&#10;  &quot;password&quot;: &quot;secret...&quot;,&#10;  &quot;tenant&quot;: &quot;00000000-0000-0000-0000-000000000000&quot;&#10;&#125;"
                  bind:value={azJsonInput}
                  rows="3"
                ></textarea>
                <div class="import-actions-row">
                  <button type="button" class="btn-import" on:click={handleParseAzureJson}>
                    Parse & Populate Azure Fields
                  </button>
                  {#if azParseError}
                    <span class="parse-err">{azParseError}</span>
                  {/if}
                </div>
              {/if}
            </div>

            <div class="grid-2">
              <div class="input-field">
                <label for="az-sub">Subscription ID (UUID) *</label>
                <input id="az-sub" type="text" placeholder="00000000-0000-0000-0000-000000000000" bind:value={azSubId} class="mono" required />
              </div>
              <div class="input-field">
                <label for="az-tenant">Tenant ID (UUID) *</label>
                <input id="az-tenant" type="text" placeholder="00000000-0000-0000-0000-000000000000" bind:value={azTenantId} class="mono" required />
              </div>
            </div>
            <div class="grid-2">
              <div class="input-field">
                <label for="az-client">Client (App) ID *</label>
                <input id="az-client" type="text" placeholder="00000000-0000-0000-0000-000000000000" bind:value={azClientId} class="mono" required />
              </div>
              <div class="input-field">
                <label for="az-secret">Client Secret *</label>
                <input id="az-secret" type="password" placeholder="Client secret token" bind:value={azClientSecret} class="mono" required />
              </div>
            </div>
            <div class="input-field">
              <label for="az-env">Cloud Environment</label>
              <input id="az-env" type="text" placeholder="AzureCloud" bind:value={azCloudEnv} />
            </div>
          </div>

        {:else if secretType === 'terraform_secret'}
          <div class="payload-section">
            <div class="input-field">
              <label for="tf-token">Provider / Cloud Token</label>
              <input id="tf-token" type="password" placeholder="tfp_xxxxxxxxxxxxxxxx" bind:value={tfToken} class="mono" />
            </div>
            <div class="input-field">
              <div class="field-label-row">
                <label for="tf-vars">Terraform Variables (.tfvars)</label>
                <button type="button" class="gen-link" on:click={handleParseTfvars}>
                  Validate & Parse (.tfvars)
                </button>
              </div>
              <textarea
                id="tf-vars"
                rows="3"
                placeholder="db_password = &quot;...&quot;&#10;cluster_size = 3"
                bind:value={tfVars}
                class="mono"
                on:blur={handleParseTfvars}
              ></textarea>
              {#if tfvarsParseResult}
                <div class="parsed-chips">
                  <span class="parsed-count">{parsedTfvarsCount} variable(s) parsed:</span>
                  {#each Object.entries(tfvarsParseResult) as [k, v]}
                    <span class="var-chip"><code>{k}</code> = <code>{v.length > 18 ? v.slice(0, 15) + '...' : v}</code></span>
                  {/each}
                </div>
              {/if}
            </div>
            <div class="input-field">
              <label for="tf-state">State Encryption Key</label>
              <input id="tf-state" type="password" placeholder="State pass/key" bind:value={tfStateKey} class="mono" />
            </div>
          </div>

        {:else if secretType === 'totp'}
          <div class="payload-section">
            <div class="input-field">
              <label for="totp-key">Base32 Secret Key *</label>
              <input id="totp-key" type="text" placeholder="JBSWY3DPEHPK3PXP" bind:value={totpSecretBase32} class="mono" required />
            </div>
            <div class="grid-2">
              <div class="input-field">
                <label for="totp-issuer">Issuer</label>
                <input id="totp-issuer" type="text" placeholder="e.g. GitHub, AWS, Google" bind:value={totpIssuer} />
              </div>
              <div class="input-field">
                <label for="totp-acc">Account Name</label>
                <input id="totp-acc" type="text" placeholder="user@example.com" bind:value={totpAccount} />
              </div>
            </div>
            <div class="grid-2">
              <div class="input-field">
                <label for="totp-digits">Digits</label>
                <input id="totp-digits" type="number" bind:value={totpDigits} min="6" max="8" />
              </div>
              <div class="input-field">
                <label for="totp-period">Step (Seconds)</label>
                <input id="totp-period" type="number" bind:value={totpPeriod} min="15" max="120" />
              </div>
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

  .quick-import-panel {
    background: rgba(0, 229, 255, 0.04);
    border: 1px dashed rgba(0, 229, 255, 0.25);
    border-radius: var(--radius-sm);
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .quick-import-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .quick-import-title {
    font-size: 11px;
    font-weight: 600;
    color: var(--accent-cyan, #00e5ff);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .quick-toggle-btn {
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 4px;
    color: #cbd5e1;
    font-size: 11px;
    padding: 2px 8px;
    cursor: pointer;
  }

  .quick-toggle-btn:hover {
    color: white;
    border-color: var(--accent-cyan, #00e5ff);
  }

  .quick-import-area {
    font-size: 11px;
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #f1f5f9;
    padding: 6px 8px;
    resize: vertical;
  }

  .import-actions-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .btn-import {
    background: rgba(0, 229, 255, 0.15);
    border: 1px solid rgba(0, 229, 255, 0.4);
    color: var(--accent-cyan, #00e5ff);
    font-size: 11px;
    font-weight: 600;
    padding: 4px 10px;
    border-radius: 4px;
    cursor: pointer;
    align-self: flex-start;
  }

  .btn-import:hover {
    background: rgba(0, 229, 255, 0.25);
  }

  .parse-err {
    font-size: 11px;
    color: #ef4444;
  }

  .parsed-chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-top: 4px;
  }

  .parsed-count {
    font-size: 11px;
    color: #94a3b8;
  }

  .var-chip {
    font-size: 10px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 4px;
    padding: 2px 6px;
    color: #38bdf8;
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
