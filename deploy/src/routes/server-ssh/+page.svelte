<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import {
    testServerSsh,
    provisionServer,
    type ServerRuntimePlan,
    type SshTarget,
    type ServerPlan
  } from "$lib/api";

  let host = "";
  let port = 22;
  let user = "root";

  type AuthMode = "password" | "keyfile" | "keypaste";
  let authMode: AuthMode = "password";
  let password = "";
  let keyPath = "";
  let keyText = "";
  let keyPassphrase = "";

  let useSameForSudo = true;
  let sudoPassword = "";

  let enableAutoUpdater = true;
  let overwriteInstall = false;
  let serviceAccountKeyPath = "";
  let userCredentialsQrPath = "";
  let advancedNetworkMode = false;
  type AccessMode = "direct" | "proxy";
  let accessMode: AccessMode = "direct";
  let directPublicAddress = "";
  let directListenPort = 8000;
  let proxyPublicUrl = "";
  let proxyListenPort = 18000;

  type DevSettings = {
    enabled: boolean;
    binariesSource: "main" | "custom";
    binariesRepo: string;
    key1Name: string;
    key1User: string;
    key2Name: string;
    key2User: string;
    githubToken: string;
    manifestVersionOverride: string;
    showDockerHelp: boolean;
  };

  const SETTINGS_KEY = "secluso-dev-settings";
  const FIRST_TIME_KEY = "secluso-first-time";
  const defaultDevSettings: DevSettings = {
    enabled: false,
    binariesSource: "main",
    binariesRepo: "",
    key1Name: "",
    key1User: "",
    key2Name: "",
    key2User: "",
    githubToken: "",
    manifestVersionOverride: "",
    showDockerHelp: false
  };
  let devSettings: DevSettings | null = null;
  let firstTimeOn = false;

  let testing = false;
  let provisioning = false;
  let errorMsg = "";
  let testResult: "ok" | "error" | null = null;
  let testMessage = "";

  function goBack() {
    goto("/");
  }

  async function pickKeyFile() {
    try {
      const path = await open({
        title: "Choose private key file",
        multiple: false,
        directory: false
      });
      if (typeof path === "string") keyPath = path;
      if (Array.isArray(path) && path.length) keyPath = path[0];
    } catch (e: any) {
      errorMsg = e?.toString() ?? "Failed to open file picker.";
    }
  }

  async function pickServiceAccountKey() {
    try {
      const path = await open({
        title: "Choose service account key (JSON)",
        multiple: false,
        directory: false,
        filters: [{ name: "JSON", extensions: ["json"] }]
      });
      if (typeof path === "string") serviceAccountKeyPath = path;
      if (Array.isArray(path) && path.length) serviceAccountKeyPath = path[0];
    } catch (e: any) {
      errorMsg = e?.toString() ?? "Failed to open file picker.";
    }
  }

  function validateTarget(): string | null {
    if (!host.trim()) return "Server host/IP is required.";
    if (port < 1 || port > 65535) return "Port must be between 1 and 65535.";
    if (!user.trim()) return "Username is required.";
    if (authMode === "password" && !password) return "Password is required.";
    if (authMode === "keyfile" && !keyPath) return "Select a private key file.";
    if (authMode === "keypaste" && !keyText.trim()) return "Paste a private key.";
    if (!useSameForSudo && !sudoPassword) return "Enter sudo password or toggle same-as-login.";
    const runtime = buildRuntimePlan();
    if (runtime.listenPort < 1 || runtime.listenPort > 65535) return "Secluso listen port must be between 1 and 65535.";
    if (effectiveAccessMode() === "proxy" && !proxyPublicUrl.trim()) return "Enter the public URL already served by your reverse proxy.";
    return null;
  }

  function effectiveAccessMode(): AccessMode {
    return advancedNetworkMode ? accessMode : "direct";
  }

  function buildAuth(): SshTarget["auth"] {
    if (authMode === "password") {
      return { kind: "password", password };
    }
    if (authMode === "keyfile") {
      return {
        kind: "keyfile",
        path: keyPath,
        passphrase: keyPassphrase.trim() ? keyPassphrase : undefined
      };
    }
    return {
      kind: "keytext",
      text: keyText,
      passphrase: keyPassphrase.trim() ? keyPassphrase : undefined
    };
  }

  function buildRuntimePlan(): ServerRuntimePlan {
    if (effectiveAccessMode() === "proxy") {
      return {
        exposureMode: "proxy",
        bindAddress: "127.0.0.1",
        listenPort: proxyListenPort
      };
    }
    return {
      exposureMode: "direct",
      bindAddress: "0.0.0.0",
      listenPort: directListenPort
    };
  }

  function buildCredentialsServerUrl(): string {
    const runtime = buildRuntimePlan();
    const mode = effectiveAccessMode();
    const candidate =
      mode === "proxy"
        ? proxyPublicUrl.trim()
        : (advancedNetworkMode ? directPublicAddress.trim() : "") || host.trim();
    if (!candidate) return "";

    const withScheme =
      candidate.startsWith("http://") || candidate.startsWith("https://")
        ? candidate
        : mode === "proxy"
        ? `https://${candidate}`
        : `http://${candidate}`;

    try {
      const url = new URL(withScheme);
      if (mode === "direct" && !url.port) {
        url.port = String(runtime.listenPort);
      }
      return url.toString().replace(/\/$/, "");
    } catch {
      return withScheme.replace(/\/$/, "");
    }
  }

  function credentialsUrlWarning(urlValue: string): string | null {
    if (!urlValue) return null;
    try {
      const url = new URL(urlValue);
      const runtime = buildRuntimePlan();
      if (effectiveAccessMode() === "direct") {
        if (url.protocol === "https:") {
          return "Direct mode serves Secluso over plain HTTP on the server port.";
        }
        const effectivePort = Number(url.port || (url.protocol === "https:" ? 443 : 80));
        if (effectivePort !== runtime.listenPort) {
          return "This public URL port does not match the configured Secluso listen port.";
        }
      }
      const hostValue = url.hostname;
      if (hostValue === "localhost" || hostValue === "127.0.0.1" || hostValue === "::1") {
        return "localhost only works on the same machine. Use your public server IP or domain for phone access.";
      }
      const ipv4Match = hostValue.match(/^(\d+)\.(\d+)\.(\d+)\.(\d+)$/);
      if (ipv4Match) {
        const [a, b] = ipv4Match.slice(1).map(Number);
        if (a === 10 || (a === 172 && b >= 16 && b <= 31) || (a === 192 && b === 168) || a === 127) {
          return "This is a private/local IP. Remote access only works if the phone can reach that network or VPN.";
        }
      }
    } catch {
      return null;
    }
    return null;
  }

  async function onTest() {
    errorMsg = "";
    testResult = null;
    testMessage = "";
    const err = validateTarget();
    if (err) { errorMsg = err; return; }

    testing = true;
    try {
      const target: SshTarget = {
        host,
        port,
        user,
        auth: buildAuth(),
        sudo: {
          mode: useSameForSudo ? "same" : "password",
          password: useSameForSudo ? undefined : sudoPassword
        }
      };

      await testServerSsh(target, buildRuntimePlan(), buildCredentialsServerUrl() || undefined);
      testResult = "ok";
      testMessage = "Preflight OK. SSH, sudo, OS, port, network, and compatibility checks passed.";
    } catch (e: any) {
      testResult = "error";
      testMessage = e?.toString() ?? "SSH test failed.";
    } finally {
      testing = false;
    }
  }

  async function onProvision() {
    errorMsg = "";
    const tErr = validateTarget();
    if (tErr) { errorMsg = tErr; return; }
    if (!serviceAccountKeyPath.trim()) { errorMsg = "Service account key is required."; return; }
    if (!userCredentialsQrPath.trim()) {
      await pickUserCredentialsQrSave();
    }
    if (!userCredentialsQrPath.trim()) { errorMsg = "Choose where to save the QR code."; return; }

    const serverUrl = buildCredentialsServerUrl();
    if (!serverUrl) { errorMsg = "Server URL is required to generate credentials."; return; }
    if (effectiveAccessMode() === "direct" && serverUrl.toLowerCase().startsWith("https://")) {
      errorMsg = "Direct mode serves Secluso over plain HTTP on the server port. Use Advanced network setup and Existing reverse proxy if your public URL should be HTTPS.";
      return;
    }
    const useDevRepo = devSettings?.enabled && devSettings.binariesSource === "custom";
    if (useDevRepo && !devSettings?.binariesRepo.trim()) {
      errorMsg = "Dev settings repo is required for a custom updater.";
      return;
    }
    if (useDevRepo) {
      const hasKey1 = !!(devSettings?.key1Name.trim() && devSettings?.key1User.trim());
      const hasKey2 = !!(devSettings?.key2Name.trim() && devSettings?.key2User.trim());
      if (!hasKey1 || !hasKey2) {
        errorMsg = "Dev settings keys are required for a custom updater.";
        return;
      }
    }
    provisioning = true;

    const target: SshTarget = {
      host, port, user,
      auth: buildAuth(),
      sudo: {
        mode: useSameForSudo ? "same" : "password",
        password: useSameForSudo ? undefined : sudoPassword
      }
    };

    const sigKeys =
      useDevRepo
        ? [
            {
              name: devSettings?.key1Name.trim() ?? "",
              githubUser: devSettings?.key1User.trim() ?? ""
            },
            {
              name: devSettings?.key2Name.trim() ?? "",
              githubUser: devSettings?.key2User.trim() ?? ""
            }
          ].filter((k) => k.name && k.githubUser)
        : [];

    const plan: ServerPlan = {
      autoUpdater: { enable: enableAutoUpdater },
      runtime: buildRuntimePlan(),
      secrets: {
        serviceAccountKeyPath,
        serverUrl,
        userCredentialsQrPath
      },
      sigKeys: sigKeys.length ? sigKeys : undefined,
      binariesRepo: useDevRepo ? devSettings?.binariesRepo.trim() : undefined,
      githubToken: devSettings?.githubToken.trim() ? devSettings?.githubToken.trim() : undefined,
      manifestVersionOverride:
        devSettings?.enabled && devSettings?.manifestVersionOverride.trim()
          ? devSettings.manifestVersionOverride.trim()
          : undefined,
      overwrite: overwriteInstall
    };

    try {
      const { run_id } = await provisionServer(target, plan);
      goto(`/status?mode=server&runId=${encodeURIComponent(run_id)}`);
    } catch (e: any) {
      errorMsg = e?.toString() ?? "Server provisioning failed.";
    } finally {
      provisioning = false;
    }
  }

  async function pickUserCredentialsQrSave() {
    try {
      const path = await save({
        title: "Choose where to save the user credentials QR code…",
        defaultPath: "user_credentials_qr.png",
        filters: [{ name: "PNG image", extensions: ["png"] }]
      });
      if (typeof path === "string" && path.length) userCredentialsQrPath = path;
    } catch (e: any) {
      errorMsg = e?.toString() ?? "Failed to open file picker.";
    }
  }

  onMount(() => {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (!raw) {
      devSettings = { ...defaultDevSettings };
      return;
    }
    try {
      const parsed = JSON.parse(raw) as Partial<DevSettings>;
      devSettings = { ...defaultDevSettings, ...parsed };
    } catch {
      devSettings = { ...defaultDevSettings };
    }
  });

  onMount(() => {
    const raw = localStorage.getItem(FIRST_TIME_KEY);
    if (raw === null) {
      firstTimeOn = true;
      return;
    }
    firstTimeOn = raw === "true";
  });

  function toggleFirstTime() {
    firstTimeOn = !firstTimeOn;
    localStorage.setItem(FIRST_TIME_KEY, String(firstTimeOn));
  }

  function setHelpRef() {
    try {
      sessionStorage.setItem("secluso-help-ref", window.location.pathname);
    } catch {
      // best effort only
    }
  }

  function isInteractiveTarget(target: EventTarget | null): boolean {
    return target instanceof Element && !!target.closest("a, button, input, label, textarea, select");
  }

  function onToggleCardClick(event: MouseEvent) {
    if (isInteractiveTarget(event.target)) return;
    toggleFirstTime();
  }

  function onToggleKey(event: KeyboardEvent) {
    if (event.key === "Enter" || event.key === " " || event.key === "Spacebar") {
      event.preventDefault();
      toggleFirstTime();
    }
  }
</script>

<main class="page">
  <div class="backdrop"></div>

  {#if testResult}
    <div class="overlay" role="status" aria-live="polite">
      <div class="modal {testResult}">
        <div class="modal-title">{testResult === "ok" ? "Preflight OK" : "Preflight failed"}</div>
        <div class="modal-body">{testMessage}</div>
        <button class="modal-btn" type="button" on:click={() => (testResult = null)}>Dismiss</button>
      </div>
    </div>
  {/if}

  <section class="frame">
    <div class="toolbar">
      <button class="back" type="button" on:click={goBack}>
        <img src="/deploy-assets/server-back.svg" alt="" />
        <span>Back</span>
      </button>
      <label class="tips-toggle">
        <span>Show tips</span>
        <span class="tips-switch">
          <input type="checkbox" checked={firstTimeOn} on:change={toggleFirstTime} />
          <span class="tips-track"></span>
        </span>
      </label>
    </div>

    <div class="step-pill">Step 2</div>

    <div class="hero">
      <div>
        <h1>Provision Server</h1>
        <p>Install Secluso on your Linux server via SSH. Sets up services, packages, and the auto-updater.</p>
      </div>
      <img class="hero-art" src="/deploy-assets/server-hero-exact.svg" alt="" />
    </div>

    <section class="panel">
      <h2>SSH Target</h2>
      <div class="grid">
        <label class="field field-wide">
          <span>Host / IP</span>
          <input placeholder="server.example.com or 203.0.113.45" bind:value={host} autocorrect="off" autocapitalize="off" spellcheck="false" />
        </label>
        <label class="field">
          <span>Port</span>
          <input type="number" min="1" max="65535" bind:value={port} />
        </label>
        <label class="field">
          <span>User</span>
          <input placeholder="root" bind:value={user} autocorrect="off" autocapitalize="off" spellcheck="false" />
        </label>
      </div>

      <div class="label">Authentication</div>
      <div class="pill-row">
        <label class="choice {authMode === 'password' ? 'selected' : ''}">
          <input type="radio" name="auth" value="password" bind:group={authMode} />
          <img src="/deploy-assets/server-auth-password.svg" alt="" />
          <span>Password</span>
        </label>
        <label class="choice {authMode === 'keyfile' ? 'selected' : ''}">
          <input type="radio" name="auth" value="keyfile" bind:group={authMode} />
          <img src="/deploy-assets/server-auth-keyfile.svg" alt="" />
          <span>Key file</span>
        </label>
        <label class="choice {authMode === 'keypaste' ? 'selected' : ''}">
          <input type="radio" name="auth" value="keypaste" bind:group={authMode} />
          <img src="/deploy-assets/server-auth-paste.svg" alt="" />
          <span>Paste key</span>
        </label>
      </div>

      {#if authMode === "password"}
        <label class="field">
          <span>Password</span>
          <input type="password" bind:value={password} autocorrect="off" autocapitalize="off" spellcheck="false" />
        </label>
      {:else if authMode === "keyfile"}
        <label class="field">
          <span>Private key path</span>
          <div class="field-row">
            <input readonly placeholder="Choose private key" bind:value={keyPath} />
            <button class="ghost" type="button" on:click={pickKeyFile}>Choose File</button>
          </div>
        </label>
        <label class="field">
          <span>Key passphrase (optional)</span>
          <input type="password" bind:value={keyPassphrase} placeholder="Only needed for encrypted private keys" autocorrect="off" autocapitalize="off" spellcheck="false" />
        </label>
      {:else}
        <label class="field">
          <span>Private key (PEM/OpenSSH)</span>
          <textarea rows="5" bind:value={keyText} placeholder="-----BEGIN OPENSSH PRIVATE KEY----- …" autocorrect="off" autocapitalize="off" spellcheck="false"></textarea>
        </label>
        <label class="field">
          <span>Key passphrase (optional)</span>
          <input type="password" bind:value={keyPassphrase} placeholder="Only needed for encrypted private keys" autocorrect="off" autocapitalize="off" spellcheck="false" />
        </label>
      {/if}

      <div class="switch-divider">
        <label class="switch-row">
          <input type="checkbox" bind:checked={useSameForSudo} />
          <span>Use same credentials for sudo</span>
        </label>
      </div>

      {#if !useSameForSudo}
        <label class="field">
          <span>Sudo password</span>
          <input type="password" bind:value={sudoPassword} autocorrect="off" autocapitalize="off" spellcheck="false" />
        </label>
      {/if}

      <button class="secondary" type="button" on:click={onTest} disabled={testing || provisioning}>
        {testing ? "Running Preflight…" : "Run Preflight"}
      </button>
    </section>

    <section class="panel">
      <h2>App Access</h2>
      <p class="muted">Simple mode exposes Secluso directly on <code>http://{host || "your-server"}:{directListenPort}</code>. Use Advanced only if you have a reverse proxy or domain.</p>

      <label class="switch-row">
        <input type="checkbox" bind:checked={advancedNetworkMode} />
        <span>Advanced network setup</span>
      </label>

      {#if advancedNetworkMode}
        <div class="pill-row">
          <label class="choice {accessMode === 'direct' ? 'selected' : ''}"><input type="radio" name="access_mode" value="direct" bind:group={accessMode} /><span>Direct IP / Port</span></label>
          <label class="choice {accessMode === 'proxy' ? 'selected' : ''}"><input type="radio" name="access_mode" value="proxy" bind:group={accessMode} /><span>Existing reverse proxy</span></label>
        </div>

        {#if accessMode === "direct"}
          <div class="grid">
            <label class="field">
              <span>Public server address override</span>
              <input placeholder="203.0.113.10 or http://203.0.113.10:9000" bind:value={directPublicAddress} autocorrect="off" autocapitalize="off" spellcheck="false" />
            </label>
            <label class="field">
              <span>Secluso listen port</span>
              <input type="number" min="1" max="65535" bind:value={directListenPort} />
            </label>
          </div>
        {:else}
          <div class="grid">
            <label class="field">
              <span>Public URL from your reverse proxy</span>
              <input placeholder="https://cam.example.com or https://example.com/secluso" bind:value={proxyPublicUrl} autocorrect="off" autocapitalize="off" spellcheck="false" />
            </label>
            <label class="field">
              <span>Local Secluso listen port</span>
              <input type="number" min="1" max="65535" bind:value={proxyListenPort} />
            </label>
          </div>
        {/if}
      {/if}

      {#if firstTimeOn}
        <div class="hint-card">
          <img src="/deploy-assets/server-info.svg" alt="" />
          <span>Most users should leave this off. Simple mode works great on a spare VPS or home server.</span>
        </div>
      {/if}

      {#if buildCredentialsServerUrl()}
        <label class="field">
          <span>Final app URL</span>
          <input readonly value={buildCredentialsServerUrl()} />
          {#if credentialsUrlWarning(buildCredentialsServerUrl())}
            <small class="warn-text">{credentialsUrlWarning(buildCredentialsServerUrl())}</small>
          {/if}
        </label>
      {/if}
    </section>

    <section class="panel">
      <h2>Files & Secrets</h2>
      <label class="field">
        <span class="field-label">
          Service account key (JSON)
          <a class="help-link" href="/service-account-help">
            <span>Where to get this?</span>
            <img src="/deploy-assets/server-external-link.svg" alt="" />
          </a>
        </span>
        <div class="field-row">
          <input readonly placeholder="Choose service_account_key.json" bind:value={serviceAccountKeyPath} />
          <button class="ghost" type="button" on:click={pickServiceAccountKey}>Choose File</button>
        </div>
      </label>

      <label class="field">
        <span>Save user credentials QR code to</span>
        <div class="field-row">
          <input readonly placeholder="Choose where to save user_credentials_qr.png" bind:value={userCredentialsQrPath} />
          <button class="ghost" type="button" on:click={pickUserCredentialsQrSave}>Choose Path</button>
        </div>
      </label>

      <label class="switch-row">
        <input type="checkbox" bind:checked={enableAutoUpdater} />
        <span>Enable auto-updater service</span>
      </label>

      <label class="switch-row">
        <input type="checkbox" bind:checked={overwriteInstall} />
        <span>Overwrite existing install</span>
      </label>
    </section>

    {#if errorMsg}
      <div class="alert error">{errorMsg}</div>
    {/if}

    <button class="primary" type="button" on:click={onProvision} disabled={provisioning || testing}>
      {provisioning ? "Provisioning…" : "Provision Server"}
      <img src="/deploy-assets/server-button-arrow.svg" alt="" />
    </button>
  </section>
</main>

<style>
  :global(body) {
    margin: 0;
    background: #050608;
    color: #f5f7fb;
    font-family: Inter, "Segoe UI", sans-serif;
  }

  .page {
    min-height: 100vh;
    position: relative;
    overflow: hidden;
    padding-bottom: 72px;
  }

  .backdrop {
    position: fixed;
    inset: 0;
    pointer-events: none;
    background:
      radial-gradient(780px 420px at 50% 132px, rgba(255, 255, 255, 0.028), transparent 68%),
      linear-gradient(180deg, rgba(5, 6, 8, 0.98), #050608 46%);
  }

  .appbar {
    height: 57px;
    margin-bottom: 32px;
    position: sticky;
    top: 0;
    z-index: 20;
    background: rgba(3, 3, 3, 0.9);
    backdrop-filter: blur(12px);
    border-bottom: 1px solid rgba(255, 255, 255, 0.03);
  }

  .frame {
    position: relative;
    z-index: 1;
    max-width: 528px;
    margin: 0 auto;
    padding: 24px 24px 0;
    box-sizing: border-box;
  }

  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 18px;
  }

  .back {
    border: none;
    padding: 0;
    background: transparent;
    color: rgba(255, 255, 255, 0.4);
    cursor: pointer;
    font-size: 13px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    line-height: 19.5px;
  }

  .back img {
    width: 14px;
    height: 14px;
    display: block;
  }

  .tips-toggle {
    display: inline-flex;
    align-items: center;
    gap: 12px;
    color: rgba(255, 255, 255, 0.3);
    font-size: 11px;
    line-height: 16.5px;
  }

  .tips-switch {
    position: relative;
    width: 24px;
    height: 13.8px;
    flex: 0 0 auto;
  }

  .tips-switch input {
    position: absolute;
    inset: 0;
    margin: 0;
    opacity: 0;
    cursor: pointer;
  }

  .tips-track {
    position: absolute;
    inset: 0;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.05);
    box-sizing: border-box;
    transition:
      background-color 140ms ease,
      border-color 140ms ease;
  }

  .tips-track::after {
    content: "";
    position: absolute;
    top: 0.9px;
    left: 0.9px;
    width: 12px;
    height: 12px;
    border-radius: 999px;
    background: #030303;
    transition: transform 140ms ease;
  }

  .tips-switch input:checked + .tips-track {
    background: #2b7fff;
    border-color: transparent;
  }

  .tips-switch input:checked + .tips-track::after {
    transform: translateX(10.25px);
  }

  .switch-row input {
    appearance: none;
    width: 28.8px;
    height: 16.56px;
    margin: 0;
    border-radius: 999px;
    border: 1px solid transparent;
    background: rgba(255, 255, 255, 0.05);
    position: relative;
    flex: 0 0 auto;
  }

  .switch-row input::after {
    content: "";
    position: absolute;
    top: 1.08px;
    left: 0;
    width: 14.4px;
    height: 14.4px;
    border-radius: 999px;
    background: #030303;
    transition: transform 120ms ease;
  }

  .switch-row input:checked {
    background: #00bc7d;
  }

  .switch-row input:checked::after {
    transform: translateX(12.5px);
  }

  .step-pill {
    display: inline-flex;
    align-items: center;
    height: 19px;
    padding: 0 8px;
    border-radius: 14px;
    background: rgba(0, 188, 125, 0.1);
    color: #00d492;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 10px;
    font-weight: 600;
    line-height: 15px;
  }

  .hero {
    position: relative;
    min-height: 121px;
    margin-top: 14px;
  }

  h1 {
    margin: 0;
    font-size: 24px;
    line-height: 32px;
    font-weight: 600;
  }

  .hero p {
    margin: 10px 0 0;
    max-width: 512.75px;
    color: rgba(255, 255, 255, 0.4);
    font-size: 14px;
    line-height: 22.75px;
  }

  .hero-art {
    position: absolute;
    top: -12px;
    right: -16px;
    width: 160px;
    height: 160px;
  }

  .panel {
    margin-top: 32px;
    padding: 16px;
    border-radius: 20px;
    border: 1px solid rgba(255, 255, 255, 0.04);
    background: rgba(255, 255, 255, 0.02);
  }

  .help-link {
    color: #4f90ff;
    text-decoration: none;
    font-size: 11px;
    display: inline-flex;
    align-items: center;
    gap: 3px;
    white-space: nowrap;
  }

  .help-link img {
    width: 12px;
    height: 12px;
    display: block;
  }

  h2 {
    margin: 0 0 20px;
    font-size: 13px;
    line-height: 19.5px;
    font-weight: 600;
  }

  .muted {
    margin: 0;
    color: rgba(255, 255, 255, 0.4);
    font-size: 12px;
    line-height: 19.5px;
  }

  code {
    padding: 1px 5px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.05);
    color: rgba(255, 255, 255, 0.82);
  }

  .label {
    margin: 24px 0 10px;
    color: rgba(255, 255, 255, 0.4);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 11px;
    line-height: 16.5px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 16px;
  }

  .field-wide { grid-column: 1 / -1; }

  .field {
    display: flex;
    flex-direction: column;
    gap: 9px;
    margin-top: 18px;
  }

  .field > span,
  .field-label {
    color: rgba(255, 255, 255, 0.4);
    font-size: 11px;
    line-height: 16.5px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .field-row {
    display: flex;
    gap: 9px;
  }

  input,
  textarea {
    width: 100%;
    min-width: 0;
    padding: 12px;
    border-radius: 16px;
    border: 1px solid rgba(255, 255, 255, 0.06);
    background: rgba(255, 255, 255, 0.03);
    color: rgba(255, 255, 255, 0.9);
    font: inherit;
    box-sizing: border-box;
  }

  input {
    height: 41.5px;
    font-size: 13px;
    line-height: 19.5px;
  }

  textarea {
    min-height: 110px;
    resize: vertical;
  }

  input::placeholder,
  textarea::placeholder {
    color: rgba(255, 255, 255, 0.28);
  }

  .pill-row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 8px;
  }

  .choice {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 12px;
    border-radius: 16px;
    border: 1px solid rgba(255, 255, 255, 0.06);
    background: rgba(255, 255, 255, 0.03);
    color: rgba(255, 255, 255, 0.5);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    box-sizing: border-box;
  }

  .choice img {
    width: 14px;
    height: 14px;
    display: block;
  }

  .choice.selected {
    border-color: rgba(0, 188, 125, 0.3);
    background: rgba(0, 188, 125, 0.15);
    color: #00d492;
  }

  .choice input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .switch-divider {
    margin-top: 16px;
    padding-top: 12px;
    border-top: 1px solid rgba(255, 255, 255, 0.04);
  }

  .switch-row {
    display: flex;
    width: 100%;
    min-height: 28px;
    padding-block: 2px;
    align-items: center;
    gap: 12px;
    color: rgba(255, 255, 255, 0.62);
    font-size: 12px;
    line-height: 18px;
    box-sizing: border-box;
  }

  .switch-row span {
    flex: 1 1 auto;
    min-width: 0;
  }

  .panel > .switch-row {
    margin-top: 14px;
  }

  .switch-row + .switch-row {
    margin-top: 14px;
  }

  .hint-card {
    margin-top: 16px;
    min-height: 65px;
    padding: 14px 12px 12px 38px;
    border-radius: 16px;
    border: 1px solid rgba(0, 188, 125, 0.1);
    background: rgba(0, 188, 125, 0.05);
    color: rgba(255, 255, 255, 0.5);
    font-size: 12px;
    line-height: 19.5px;
    position: relative;
    box-sizing: border-box;
  }

  .hint-card img {
    position: absolute;
    left: 12px;
    top: 14px;
    width: 16px;
    height: 16px;
    display: block;
  }

  .warn-text {
    margin-top: 2px;
    color: #fbbf24;
    font-size: 12px;
  }

  button {
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.04);
    color: rgba(255, 255, 255, 0.84);
    cursor: pointer;
    font: inherit;
  }

  .ghost,
  .secondary {
    padding: 0 16px;
    height: 36px;
    border-radius: 16px;
    white-space: nowrap;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.6);
  }

  .field-row .ghost {
    min-width: 100px;
  }

  .secondary {
    margin-top: 16px;
    width: fit-content;
    min-width: 106.9px;
    max-width: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .primary {
    width: 100%;
    margin-top: 40px;
    height: 45px;
    border-radius: 20px;
    border: none;
    background: #00bc7d;
    color: #fff;
    font-size: 14px;
    font-weight: 500;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
  }

  .primary img {
    width: 16px;
    height: 16px;
    display: block;
  }

  button:disabled {
    opacity: 0.56;
    cursor: not-allowed;
  }

  .alert {
    margin-top: 18px;
    padding: 12px 14px;
    border-radius: 14px;
    border: 1px solid rgba(248, 113, 113, 0.24);
    background: rgba(127, 29, 29, 0.25);
    color: #fecaca;
    font-size: 14px;
  }

  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(2, 6, 23, 0.62);
    display: grid;
    place-items: center;
    z-index: 30;
  }

  .modal {
    width: min(420px, 90vw);
    background: #0a0c11;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 16px;
    padding: 18px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.45);
  }

  .modal.ok { border-color: rgba(18, 216, 159, 0.24); }
  .modal.error { border-color: rgba(248, 113, 113, 0.24); }
  .modal-title { font-size: 18px; font-weight: 700; margin-bottom: 6px; }
  .modal-body { color: rgba(255, 255, 255, 0.64); margin-bottom: 14px; }
  .modal-btn { padding: 10px 14px; border-radius: 10px; }

  @media (max-width: 640px) {
    .appbar-inner,
    .frame { padding-inline: 14px; }
    .hero {
      min-height: 0;
      padding-right: 112px;
    }
    .hero-art { width: 112px; height: 112px; top: -2px; right: -6px; }
    .grid { grid-template-columns: 1fr; }
    .field-row { flex-direction: column; }
    .secondary,
    .field-row .ghost { width: 100%; }
  }
</style>
