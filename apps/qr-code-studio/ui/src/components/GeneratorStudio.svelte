<script lang="ts">
  import { invokeIpc } from '../lib/ipc';

  export let onGenerated: () => void = () => {};

  // Payload Type State
  type PayloadTab = 'text' | 'url' | 'wifi' | 'vcard' | 'email' | 'sms' | 'geo' | 'totp' | 'crypto';
  let activeTab: PayloadTab = 'url';

  // Inputs
  let plainText = 'https://antigravity.internal';
  let urlScheme = 'https://';
  let urlHost = 'antigravity.internal';

  let wifiSsid = 'HomeWiFi';
  let wifiAuth = 'WPA';
  let wifiPass = 'SuperSecret123';
  let wifiHidden = false;

  let vcardFirst = 'Alan';
  let vcardLast = 'Turing';
  let vcardOrg = 'Bletchley Park';
  let vcardPhone = '+44 1908 640136';
  let vcardEmail = 'alan@bletchley.org';

  let emailTo = 'security@antigravity.internal';
  let emailSubject = 'Incident Report';
  let emailBody = 'Encrypted offline telemetry report.';

  let smsNumber = '+15550199';
  let smsMessage = 'Secure activation code.';

  let geoLat = '37.7749';
  let geoLng = '-122.4194';

  let totpSecret = 'JBSWY3DPEHPK3PXP';
  let totpIssuer = 'Antigravity Studio';
  let totpAccount = 'developer';

  let cryptoCoin = 'BTC';
  let cryptoAddr = 'bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq';
  let cryptoAmount = '0.05';

  // Style Config
  let moduleShape: 'Square' | 'Rounded' | 'Dots' | 'Diamond' = 'Rounded';
  let finderStyle: 'Square' | 'Rounded' | 'Circle' = 'Rounded';
  let foregroundColor = '#06b6d4';
  let enableGradient = true;
  let secondaryColor = '#8b5cf6';
  let gradientAngle = 45;
  let backgroundColor = '#0a0e17';
  let eccLevel: 'L' | 'M' | 'Q' | 'H' = 'M';
  let quietZone = 4;
  let centerLogoText = '';
  let customLabel = '';

  // Security options
  let signEd25519 = false;
  let encryptAes = false;
  let passphrase = '';

  // Generation Output
  let isGenerating = false;
  let errorMessage = '';
  let generatedResult: {
    id: string;
    payload_type: string;
    raw_payload: string;
    version: number;
    dimension: number;
    svg_markup: string;
    png_base64: string;
    contrast_ratio: number;
    contrast_warning: string | null;
    is_encrypted: boolean;
    has_signature: boolean;
  } | null = null;

  // Build formatted raw payload from active tab
  function buildPayloadString(): string {
    switch (activeTab) {
      case 'url': {
        const full = (urlHost.startsWith('http://') || urlHost.startsWith('https://'))
          ? urlHost
          : `${urlScheme}${urlHost}`;
        return full;
      }
      case 'wifi':
        return `WIFI:S:${wifiSsid};T:${wifiAuth};P:${wifiPass};H:${wifiHidden ? 'true' : 'false'};;`;
      case 'vcard':
        return `BEGIN:VCARD\nVERSION:3.0\nN:${vcardLast};${vcardFirst};;;\nFN:${vcardFirst} ${vcardLast}\nORG:${vcardOrg}\nTEL:${vcardPhone}\nEMAIL:${vcardEmail}\nEND:VCARD`;
      case 'email':
        return `mailto:${emailTo}?subject=${encodeURIComponent(emailSubject)}&body=${encodeURIComponent(emailBody)}`;
      case 'sms':
        return `smsto:${smsNumber}:${smsMessage}`;
      case 'geo':
        return `geo:${geoLat},${geoLng}`;
      case 'totp':
        return `otpauth://totp/${encodeURIComponent(totpIssuer)}:${encodeURIComponent(totpAccount)}?secret=${totpSecret}&issuer=${encodeURIComponent(totpIssuer)}`;
      case 'crypto':
        return `${cryptoCoin.toLowerCase()}:${cryptoAddr}?amount=${cryptoAmount}`;
      case 'text':
      default:
        return plainText;
    }
  }

  async function generate() {
    isGenerating = true;
    errorMessage = '';
    try {
      const rawText = buildPayloadString();
      const style = {
        ecc_level: eccLevel,
        module_shape: moduleShape,
        finder_style: finderStyle,
        foreground_color: foregroundColor,
        secondary_color: enableGradient ? secondaryColor : null,
        gradient_angle: enableGradient ? gradientAngle : 0,
        background_color: backgroundColor,
        quiet_zone: quietZone,
        center_logo_text: centerLogoText.trim() ? centerLogoText.trim() : null,
        target_size: 360,
      };

      const result = await invokeIpc<any>('generate_qr', {
        raw_text: rawText,
        label: customLabel.trim() ? customLabel.trim() : null,
        style,
        passphrase: encryptAes && passphrase.trim() ? passphrase.trim() : null,
        sign_ed25519: signEd25519,
      });

      generatedResult = result;
      onGenerated();
    } catch (e: any) {
      errorMessage = e?.message || String(e);
    } finally {
      isGenerating = false;
    }
  }

  function downloadSvg() {
    if (!generatedResult) return;
    const blob = new Blob([generatedResult.svg_markup], { type: 'image/svg+xml' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `qrcode_${Date.now()}.svg`;
    a.click();
    URL.revokeObjectURL(url);
  }

  function downloadPng() {
    if (!generatedResult) return;
    const a = document.createElement('a');
    a.href = generatedResult.png_base64;
    a.download = `qrcode_${Date.now()}.png`;
    a.click();
  }

  function copyBase64() {
    if (!generatedResult) return;
    navigator.clipboard.writeText(generatedResult.png_base64);
  }

  function selectTab(id: string) {
    activeTab = id as PayloadTab;
    generate();
  }

  // Initial generation
  generate();
</script>

<div class="generator-container">
  <!-- Left Side: Controls & Input Tabs -->
  <div class="controls-panel">
    <div class="header-section">
      <h2>Studio QR Generator</h2>
      <p class="subtitle">ISO/IEC 18004 Vector & Raster Engine with Air-Gapped Security</p>
    </div>

    <!-- Payload Type Selector Tabs -->
    <div class="tabs-scroll">
      <div class="tabs-bar">
        {#each [
          { id: 'url', label: '🔗 URL' },
          { id: 'wifi', label: '📶 Wi-Fi' },
          { id: 'vcard', label: '👤 vCard' },
          { id: 'text', label: '📝 Text' },
          { id: 'email', label: '✉️ Email' },
          { id: 'sms', label: '💬 SMS' },
          { id: 'geo', label: '📍 Geo' },
          { id: 'totp', label: '🔑 TOTP' },
          { id: 'crypto', label: '₿ Crypto' }
        ] as tab}
          <button
            class="tab-btn"
            class:active={activeTab === tab.id}
            on:click={() => selectTab(tab.id)}
          >
            {tab.label}
          </button>
        {/each}
      </div>
    </div>

    <!-- Form Fields by Tab -->
    <div class="form-body">
      {#if activeTab === 'url'}
        <div class="input-group">
          <label for="url-in">Destination Web Address</label>
          <div class="input-row">
            <select bind:value={urlScheme} on:change={generate} class="select-scheme">
              <option value="https://">https://</option>
              <option value="http://">http://</option>
            </select>
            <input
              id="url-in"
              type="text"
              bind:value={urlHost}
              on:input={generate}
              placeholder="e.g. antigravity.dev/docs"
              class="text-input"
            />
          </div>
        </div>
      {:else if activeTab === 'wifi'}
        <div class="grid-2">
          <div class="input-group">
            <label for="wifi-ssid">Network SSID</label>
            <input id="wifi-ssid" type="text" bind:value={wifiSsid} on:input={generate} class="text-input" />
          </div>
          <div class="input-group">
            <label for="wifi-auth">Security Type</label>
            <select id="wifi-auth" bind:value={wifiAuth} on:change={generate} class="text-input">
              <option value="WPA">WPA/WPA2/WPA3</option>
              <option value="WEP">WEP</option>
              <option value="nopass">None (Open)</option>
            </select>
          </div>
        </div>
        <div class="input-group">
          <label for="wifi-pass">Network Password</label>
          <input id="wifi-pass" type="text" bind:value={wifiPass} on:input={generate} class="text-input" />
        </div>
        <label class="checkbox-row">
          <input type="checkbox" bind:checked={wifiHidden} on:change={generate} />
          <span>Hidden Network SSID</span>
        </label>
      {:else if activeTab === 'vcard'}
        <div class="grid-2">
          <div class="input-group">
            <label for="vc-first">First Name</label>
            <input id="vc-first" type="text" bind:value={vcardFirst} on:input={generate} class="text-input" />
          </div>
          <div class="input-group">
            <label for="vc-last">Last Name</label>
            <input id="vc-last" type="text" bind:value={vcardLast} on:input={generate} class="text-input" />
          </div>
        </div>
        <div class="grid-2">
          <div class="input-group">
            <label for="vc-org">Organization</label>
            <input id="vc-org" type="text" bind:value={vcardOrg} on:input={generate} class="text-input" />
          </div>
          <div class="input-group">
            <label for="vc-phone">Phone</label>
            <input id="vc-phone" type="text" bind:value={vcardPhone} on:input={generate} class="text-input" />
          </div>
        </div>
        <div class="input-group">
          <label for="vc-email">Email</label>
          <input id="vc-email" type="email" bind:value={vcardEmail} on:input={generate} class="text-input" />
        </div>
      {:else if activeTab === 'text'}
        <div class="input-group">
          <label for="plain-text">Plaintext Payload</label>
          <textarea id="plain-text" rows="3" bind:value={plainText} on:input={generate} class="text-input textarea"></textarea>
        </div>
      {:else if activeTab === 'email'}
        <div class="input-group">
          <label for="em-to">Recipient Email</label>
          <input id="em-to" type="email" bind:value={emailTo} on:input={generate} class="text-input" />
        </div>
        <div class="input-group">
          <label for="em-sub">Subject</label>
          <input id="em-sub" type="text" bind:value={emailSubject} on:input={generate} class="text-input" />
        </div>
        <div class="input-group">
          <label for="em-body">Body</label>
          <textarea id="em-body" rows="2" bind:value={emailBody} on:input={generate} class="text-input textarea"></textarea>
        </div>
      {:else if activeTab === 'sms'}
        <div class="input-group">
          <label for="sms-num">Phone Number</label>
          <input id="sms-num" type="text" bind:value={smsNumber} on:input={generate} class="text-input" />
        </div>
        <div class="input-group">
          <label for="sms-msg">SMS Message</label>
          <input id="sms-msg" type="text" bind:value={smsMessage} on:input={generate} class="text-input" />
        </div>
      {:else if activeTab === 'geo'}
        <div class="grid-2">
          <div class="input-group">
            <label for="geo-lat">Latitude</label>
            <input id="geo-lat" type="text" bind:value={geoLat} on:input={generate} class="text-input" />
          </div>
          <div class="input-group">
            <label for="geo-lng">Longitude</label>
            <input id="geo-lng" type="text" bind:value={geoLng} on:input={generate} class="text-input" />
          </div>
        </div>
      {:else if activeTab === 'totp'}
        <div class="grid-2">
          <div class="input-group">
            <label for="totp-iss">Issuer</label>
            <input id="totp-iss" type="text" bind:value={totpIssuer} on:input={generate} class="text-input" />
          </div>
          <div class="input-group">
            <label for="totp-acc">Account / Username</label>
            <input id="totp-acc" type="text" bind:value={totpAccount} on:input={generate} class="text-input" />
          </div>
        </div>
        <div class="input-group">
          <label for="totp-sec">Base32 Secret Key</label>
          <input id="totp-sec" type="text" bind:value={totpSecret} on:input={generate} class="text-input" />
        </div>
      {:else if activeTab === 'crypto'}
        <div class="grid-2">
          <div class="input-group">
            <label for="cry-coin">Coin / Network</label>
            <select id="cry-coin" bind:value={cryptoCoin} on:change={generate} class="text-input">
              <option value="BTC">Bitcoin (BTC)</option>
              <option value="ETH">Ethereum (ETH)</option>
              <option value="SOL">Solana (SOL)</option>
              <option value="USDT">Tether (USDT)</option>
            </select>
          </div>
          <div class="input-group">
            <label for="cry-amt">Amount</label>
            <input id="cry-amt" type="text" bind:value={cryptoAmount} on:input={generate} class="text-input" />
          </div>
        </div>
        <div class="input-group">
          <label for="cry-addr">Wallet Address</label>
          <input id="cry-addr" type="text" bind:value={cryptoAddr} on:input={generate} class="text-input" />
        </div>
      {/if}

      <!-- Custom Styling Controls Accordion -->
      <div class="styling-section">
        <h4 class="section-title">Visual Styling & Customization</h4>
        
        <div class="grid-3">
          <div class="input-group">
            <label for="mod-shape">Module Shape</label>
            <select id="mod-shape" bind:value={moduleShape} on:change={generate} class="text-input">
              <option value="Rounded">Rounded</option>
              <option value="Square">Square</option>
              <option value="Dots">Dots</option>
              <option value="Diamond">Diamond</option>
            </select>
          </div>

          <div class="input-group">
            <label for="find-style">Finder Pattern</label>
            <select id="find-style" bind:value={finderStyle} on:change={generate} class="text-input">
              <option value="Rounded">Rounded Eyes</option>
              <option value="Square">Square Eyes</option>
              <option value="Circle">Circle Eyes</option>
            </select>
          </div>

          <div class="input-group">
            <label for="ecc-lvl">Error Correction</label>
            <select id="ecc-lvl" bind:value={eccLevel} on:change={generate} class="text-input">
              <option value="L">L (7% Recovery)</option>
              <option value="M">M (15% Recovery)</option>
              <option value="Q">Q (25% Recovery)</option>
              <option value="H">H (30% Recovery)</option>
            </select>
          </div>
        </div>

        <div class="grid-3">
          <div class="input-group">
            <label for="fg-col">Foreground</label>
            <div class="color-picker-wrap">
              <input id="fg-col" type="color" bind:value={foregroundColor} on:change={generate} />
              <span>{foregroundColor}</span>
            </div>
          </div>

          <div class="input-group">
            <label for="bg-col">Background</label>
            <div class="color-picker-wrap">
              <input id="bg-col" type="color" bind:value={backgroundColor} on:change={generate} />
              <span>{backgroundColor}</span>
            </div>
          </div>

          <div class="input-group">
            <label for="badge-txt">Center Logo Badge</label>
            <input
              id="badge-txt"
              type="text"
              maxlength="4"
              bind:value={centerLogoText}
              on:input={generate}
              placeholder="e.g. AG"
              class="text-input"
            />
          </div>
        </div>

        <div class="gradient-row">
          <label class="checkbox-row">
            <input type="checkbox" bind:checked={enableGradient} on:change={generate} />
            <span>Enable Linear Gradient</span>
          </label>
          {#if enableGradient}
            <div class="color-picker-wrap inline-picker">
              <input type="color" bind:value={secondaryColor} on:change={generate} />
              <span>{secondaryColor}</span>
            </div>
            <select bind:value={gradientAngle} on:change={generate} class="select-small">
              <option value={0}>Horizontal (0°)</option>
              <option value={45}>Diagonal (45°)</option>
              <option value={90}>Vertical (90°)</option>
              <option value={135}>Reverse Diagonal (135°)</option>
            </select>
          {/if}
        </div>
      </div>

      <!-- Cryptography & Protection Accordion -->
      <div class="crypto-section">
        <h4 class="section-title">Cryptographic Extension</h4>
        <div class="grid-2">
          <label class="checkbox-card" class:checked={signEd25519}>
            <input type="checkbox" bind:checked={signEd25519} on:change={generate} />
            <div>
              <strong>Ed25519 Digital Signature</strong>
              <p>Signs payload with offline private key for zero-trust authenticity.</p>
            </div>
          </label>

          <label class="checkbox-card" class:checked={encryptAes}>
            <input type="checkbox" bind:checked={encryptAes} on:change={generate} />
            <div>
              <strong>AES-256-GCM Encryption</strong>
              <p>Protects payload behind PBKDF2 passphrase encryption.</p>
            </div>
          </label>
        </div>

        {#if encryptAes}
          <div class="input-group passphrase-box">
            <label for="enc-pass">Encryption Passphrase</label>
            <input
              id="enc-pass"
              type="password"
              bind:value={passphrase}
              on:input={generate}
              placeholder="Enter strong passphrase to seal QR payload"
              class="text-input"
            />
          </div>
        {/if}
      </div>

      {#if errorMessage}
        <div class="error-banner">
          ⚠️ {errorMessage}
        </div>
      {/if}
    </div>
  </div>

  <!-- Right Side: Real-Time Live Preview & Actions -->
  <div class="preview-panel">
    <div class="preview-card">
      <div class="preview-header">
        <span class="preview-title">Real-Time QR Matrix</span>
        {#if generatedResult}
          <span class="version-badge">V{generatedResult.version} ({generatedResult.dimension}x{generatedResult.dimension})</span>
        {/if}
      </div>

      <div class="matrix-stage">
        {#if generatedResult}
          <div class="svg-wrapper">
            {@html generatedResult.svg_markup}
          </div>
        {:else}
          <div class="matrix-placeholder">Generating matrix...</div>
        {/if}
      </div>

      {#if generatedResult}
        <!-- WCAG Contrast & Scannability Indicator -->
        <div class="contrast-bar" class:warning={Boolean(generatedResult.contrast_warning)}>
          <div class="contrast-info">
            <span class="ratio-val">Contrast: {generatedResult.contrast_ratio.toFixed(2)}:1</span>
            {#if generatedResult.contrast_warning}
              <span class="warning-text">⚠️ {generatedResult.contrast_warning}</span>
            {:else}
              <span class="pass-text">✓ High contrast (ISO scannable)</span>
            {/if}
          </div>
          <div class="tags-row">
            {#if generatedResult.has_signature}
              <span class="tag-badge signed">Ed25519 Signed</span>
            {/if}
            {#if generatedResult.is_encrypted}
              <span class="tag-badge encrypted">AES-GCM Encrypted</span>
            {/if}
          </div>
        </div>

        <!-- Export Actions -->
        <div class="actions-grid">
          <button class="btn-primary" on:click={downloadSvg}>
            Download SVG (Vector)
          </button>
          <button class="btn-secondary" on:click={downloadPng}>
            Download PNG (Raster)
          </button>
          <button class="btn-outline" on:click={copyBase64}>
            Copy Data URI
          </button>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .generator-container {
    display: grid;
    grid-template-columns: 1fr 440px;
    height: 100%;
    overflow: hidden;
    gap: 1.5rem;
    padding: 1.5rem;
  }

  .controls-panel {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .header-section {
    padding: 1.25rem 1.5rem;
    border-bottom: 1px solid var(--border-color);
  }

  .header-section h2 {
    font-size: 1.25rem;
    font-weight: 700;
    color: var(--text-primary);
  }

  .subtitle {
    font-size: 0.8rem;
    color: var(--text-muted);
    margin-top: 0.25rem;
  }

  .tabs-scroll {
    overflow-x: auto;
    background: rgba(15, 23, 42, 0.6);
    border-bottom: 1px solid var(--border-color);
  }

  .tabs-bar {
    display: flex;
    padding: 0.5rem 1rem;
    gap: 0.5rem;
    white-space: nowrap;
  }

  .tab-btn {
    padding: 0.4rem 0.8rem;
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted);
    border: 1px solid transparent;
    cursor: pointer;
    font-size: 0.85rem;
    transition: all 0.15s ease;
  }

  .tab-btn:hover {
    color: var(--text-primary);
    background: var(--bg-elevated);
  }

  .tab-btn.active {
    color: var(--accent-cyan);
    background: rgba(6, 182, 212, 0.12);
    border-color: rgba(6, 182, 212, 0.4);
    font-weight: 600;
  }

  .form-body {
    padding: 1.5rem;
    overflow-y: auto;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .input-group label {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .input-row {
    display: flex;
    gap: 0.5rem;
  }

  .select-scheme {
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    color: var(--accent-cyan);
    border-radius: 6px;
    padding: 0.5rem 0.75rem;
    font-size: 0.9rem;
    font-weight: 600;
  }

  .text-input {
    width: 100%;
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    border-radius: 6px;
    padding: 0.55rem 0.75rem;
    font-size: 0.9rem;
    transition: border-color 0.15s ease;
  }

  .text-input:focus {
    outline: none;
    border-color: var(--accent-cyan);
  }

  .textarea {
    resize: vertical;
    font-family: var(--font-mono);
    font-size: 0.85rem;
  }

  .grid-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  .grid-3 {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 1rem;
  }

  .checkbox-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .styling-section, .crypto-section {
    background: rgba(15, 23, 42, 0.4);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
  }

  .section-title {
    font-size: 0.85rem;
    font-weight: 700;
    color: var(--accent-indigo);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .color-picker-wrap {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    padding: 0.35rem 0.5rem;
  }

  .color-picker-wrap input[type="color"] {
    border: none;
    background: transparent;
    width: 24px;
    height: 24px;
    cursor: pointer;
  }

  .color-picker-wrap span {
    font-size: 0.8rem;
    font-family: var(--font-mono);
    color: var(--text-secondary);
  }

  .gradient-row {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .inline-picker {
    padding: 0.2rem 0.5rem;
  }

  .select-small {
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    border-radius: 6px;
    padding: 0.35rem 0.6rem;
    font-size: 0.8rem;
  }

  .checkbox-card {
    display: flex;
    gap: 0.75rem;
    padding: 0.75rem;
    border-radius: 6px;
    border: 1px solid var(--border-color);
    background: var(--bg-elevated);
    cursor: pointer;
    align-items: flex-start;
  }

  .checkbox-card.checked {
    border-color: var(--accent-indigo);
    background: rgba(99, 102, 241, 0.1);
  }

  .checkbox-card strong {
    font-size: 0.85rem;
    color: var(--text-primary);
    display: block;
  }

  .checkbox-card p {
    font-size: 0.75rem;
    color: var(--text-muted);
    margin-top: 0.2rem;
  }

  .passphrase-box {
    margin-top: 0.5rem;
  }

  .error-banner {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.4);
    color: #fca5a5;
    padding: 0.75rem;
    border-radius: 6px;
    font-size: 0.85rem;
  }

  .preview-panel {
    display: flex;
    flex-direction: column;
  }

  .preview-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.25rem;
    height: 100%;
  }

  .preview-header {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .preview-title {
    font-size: 1rem;
    font-weight: 700;
    color: var(--text-primary);
  }

  .version-badge {
    background: var(--bg-elevated);
    padding: 0.25rem 0.6rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    color: var(--accent-cyan);
    font-family: var(--font-mono);
  }

  .matrix-stage {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    background: #000000;
    border-radius: 12px;
    padding: 1.5rem;
    border: 1px solid var(--border-color);
  }

  .svg-wrapper {
    display: flex;
    align-items: center;
    justify-content: center;
    max-width: 100%;
    max-height: 340px;
  }

  .svg-wrapper :global(svg) {
    width: 100%;
    height: 100%;
    max-height: 320px;
  }

  .contrast-bar {
    width: 100%;
    background: var(--bg-elevated);
    border-radius: 8px;
    padding: 0.75rem 1rem;
    border: 1px solid var(--border-color);
  }

  .contrast-bar.warning {
    border-color: var(--accent-amber);
    background: rgba(245, 158, 11, 0.1);
  }

  .contrast-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.8rem;
  }

  .ratio-val {
    font-family: var(--font-mono);
    font-weight: 700;
    color: var(--text-primary);
  }

  .pass-text {
    color: var(--accent-emerald);
    font-weight: 600;
  }

  .warning-text {
    color: var(--accent-amber);
    font-weight: 600;
  }

  .tags-row {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }

  .tag-badge {
    font-size: 0.7rem;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-weight: 600;
    text-transform: uppercase;
  }

  .tag-badge.signed {
    background: rgba(99, 102, 241, 0.2);
    color: #a5b4fc;
    border: 1px solid rgba(99, 102, 241, 0.4);
  }

  .tag-badge.encrypted {
    background: rgba(16, 185, 129, 0.2);
    color: #6ee7b7;
    border: 1px solid rgba(16, 185, 129, 0.4);
  }

  .actions-grid {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .btn-primary, .btn-secondary, .btn-outline {
    width: 100%;
    padding: 0.65rem;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .btn-primary {
    background: var(--accent-cyan);
    color: #0b0f19;
    border: none;
  }

  .btn-primary:hover {
    background: #22d3ee;
  }

  .btn-secondary {
    background: var(--bg-elevated);
    color: var(--text-primary);
    border: 1px solid var(--border-color);
  }

  .btn-secondary:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .btn-outline {
    background: transparent;
    color: var(--text-muted);
    border: 1px dashed var(--border-color);
  }

  .btn-outline:hover {
    color: var(--text-primary);
    border-color: var(--text-muted);
  }
</style>
