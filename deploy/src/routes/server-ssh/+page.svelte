<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import {
    testServerSsh,
    provisionServer,
    checkRequirements,
    type RequirementStatus,
    type SshTarget,
    type ServerPlan
  } from "$lib/api";

  // ssh target state
  let host = "";
  let port = 22;
  let user = "root";

  type AuthMode = "password" | "keyfile" | "keypaste";
  let authMode: AuthMode = "password";
  let password = "";
  let keyPath = "";
  let keyText = "";

  let useSameForSudo = true;
  let sudoPassword = "";

  let enableAutoUpdater = true;
  let overwriteInstall = false;
  let serviceAccountKeyPath = "";
  let userCredentialsQrPath = "";
  let credentialsServerUrl = "";
  type DevSettings = {
    enabled: boolean;
    binariesSource: "main" | "custom";
    binariesRepo: string;
    key1Name: string;
    key1User: string;
    key2Name: string;
    key2User: string;
    githubToken: string;
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
    githubToken: ""
  };
  let devSettings: DevSettings | null = null;
  let firstTimeOn = false;

  // ui state
  let testing = false;
  let provisioning = false;
  let errorMsg = "";
  let testResult: "ok" | "error" | null = null;
  let testMessage = "";
  let requirements: RequirementStatus[] = [];
  let missingRequirements: RequirementStatus[] = [];
  let checkingRequirements = true;
  $: dockerMissing = missingRequirements.some((req) => req.name === "Docker");
  $: buildxMissing = missingRequirements.some((req) => req.name === "Docker Buildx");

  function goBack() {
    goto("/");
  }

  async function pickKeyFile() {
    try {
      const path = await open({
        title: "Choose private key file",
        multiple: false,
        directory: false,
        filters: [{ name: "SSH Key", extensions: ["pem", "key", "ppk"] }]
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
    return null;
  }

  async function onTest() {
    errorMsg = "";
    testResult = null;
    testMessage = "";
    if (checkingRequirements) {
      errorMsg = "Checking required tools. Try again in a moment.";
      return;
    }
    if (missingRequirements.length > 0) {
      errorMsg = `Missing required tools: ${missingRequirements.map((req) => req.name).join(", ")}.`;
      return;
    }
    const err = validateTarget();
    if (err) { errorMsg = err; return; }

    testing = true;
    try {
      const target: SshTarget = {
        host,
        port,
        user,
        auth:
          authMode === "password"
            ? { kind: "password", password }
            : authMode === "keyfile"
            ? { kind: "keyfile", path: keyPath }
            : { kind: "keytext", text: keyText },
        sudo: {
          mode: useSameForSudo ? "same" : "password",
          password: useSameForSudo ? undefined : sudoPassword
        }
      };

      await testServerSsh(target);
      testResult = "ok";
      testMessage = "SSH OK. Server reachable and command execution succeeded.";
    } catch (e: any) {
      testResult = "error";
      testMessage = e?.toString() ?? "SSH test failed.";
    } finally {
      testing = false;
    }
  }

  async function onProvision() {
    errorMsg = "";
    if (checkingRequirements) {
      errorMsg = "Checking required tools. Try again in a moment.";
      return;
    }
    if (missingRequirements.length > 0) {
      errorMsg = `Missing required tools: ${missingRequirements.map((req) => req.name).join(", ")}.`;
      return;
    }
    const tErr = validateTarget();
    if (tErr) { errorMsg = tErr; return; }
    if (!serviceAccountKeyPath.trim()) { errorMsg = "Service account key is required."; return; }
    if (!userCredentialsQrPath.trim()) {
      await pickUserCredentialsQrSave();
    }
    if (!userCredentialsQrPath.trim()) { errorMsg = "Choose where to save the QR code."; return; }

    const rawServerUrl = credentialsServerUrl.trim();
    const serverUrl = rawServerUrl
      ? rawServerUrl.startsWith("http://") || rawServerUrl.startsWith("https://")
        ? rawServerUrl
        : `https://${rawServerUrl}`
      : host.trim()
      ? `https://${host.trim()}`
      : "";
    if (!serverUrl) { errorMsg = "Server URL is required to generate credentials."; return; }
    if (serverUrl.toLowerCase().startsWith("https://")) {
      errorMsg = "HTTPS is not supported yet for automatic setups. Use http:// for now.";
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
      auth:
        authMode === "password"
          ? { kind: "password", password }
          : authMode === "keyfile"
          ? { kind: "keyfile", path: keyPath }
          : { kind: "keytext", text: keyText },
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
      useDocker: false,
      protectPackages: false,
      autoUpdater: { enable: enableAutoUpdater },
      secrets: {
        serviceAccountKeyPath,
        serverUrl,
        userCredentialsQrPath
      },
      sigKeys: sigKeys.length ? sigKeys : undefined,
      binariesRepo: useDevRepo ? devSettings?.binariesRepo.trim() : undefined,
      githubToken: devSettings?.githubToken.trim() ? devSettings?.githubToken.trim() : undefined,
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
        title: "Save user credentials QR code as…",
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
    if (raw === null) return;
    firstTimeOn = raw === "true";
  });

  function toggleFirstTime() {
    firstTimeOn = !firstTimeOn;
    localStorage.setItem(FIRST_TIME_KEY, String(firstTimeOn));
  }

  onMount(async () => {
    try {
      requirements = await checkRequirements();
      missingRequirements = requirements.filter((req) => !req.ok);
    } catch {
      requirements = [];
      missingRequirements = [];
    } finally {
      checkingRequirements = false;
    }
  });
</script>

<main class="wrap">
  {#if testResult}
    <div class="overlay" role="status" aria-live="polite">
      <div class="modal {testResult}">
        <div class="modal-title">{testResult === "ok" ? "SSH OK" : "SSH test failed"}</div>
        <div class="modal-body">{testMessage}</div>
        <button class="modal-btn" type="button" on:click={() => (testResult = null)}>Dismiss</button>
      </div>
    </div>
  {/if}
  <header class="topbar">
    <button class="back" type="button" on:click={goBack}>← Back</button>
    <h1>Provision Server (SSH)</h1>
    <div class="spacer"></div>
  </header>

  {#if checkingRequirements}
    <section class="card requirements">
      <h2>Setup checks</h2>
      <p class="muted">Checking local tools…</p>
    </section>
  {:else if missingRequirements.length > 0}
    <section class="card requirements">
      <h2>Missing tools</h2>
      <ul class="req-list">
        {#each missingRequirements as req}
          <li class="req-item">
            <span class="req-name">{req.name}</span>
            <span class="req-status missing">Missing</span>
            <span class="req-detail">{req.hint}</span>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if dockerMissing || buildxMissing}
    <section class="card requirements">
      <h2>Install Docker</h2>
      <p class="muted">Docker is required to continue.</p>
      <ul class="req-steps">
        <li>Windows: install Docker Desktop and enable the WSL 2 backend.</li>
        <li>macOS: install Docker Desktop for Mac.</li>
        <li>Linux: install Docker Engine and the Buildx plugin.</li>
      </ul>
      <div class="req-links">
        <a href="https://docs.docker.com/desktop/install/windows-install/">Windows install guide</a>
        <a href="https://docs.docker.com/desktop/install/mac-install/">macOS install guide</a>
        <a href="https://docs.docker.com/engine/install/">Linux install guide</a>
      </div>
    </section>
  {/if}

  {#if firstTimeOn}
    <section class="card">
      <div class="cardhead">
        <h2>Need help?</h2>
        <label class="toggle">
          <input type="checkbox" checked={firstTimeOn} on:change={toggleFirstTime} />
          <span>On</span>
        </label>
      </div>
      <ol class="quick-steps">
        <li>Enter the server login details you get from your provider and test the connection.</li>
        <li>Set your server address and choose your service account key file.</li>
        <li>Choose where to save the server QR code.</li>
        <li>Click Provision Server, then scan this QR code in the app.</li>
        <li>When you are done, open the app and scan the server QR code, then the camera QR code.</li>
      </ol>
      <p class="muted">Need a server? A low cost option is Ionos VPS for around $2 per month. Just copy the login details from your provider and the app does the rest. We are not affiliated with Ionos.</p>
    </section>
  {:else}
    <section class="card">
      <div class="cardhead">
        <h2>Need help?</h2>
        <label class="toggle">
          <input type="checkbox" checked={firstTimeOn} on:change={toggleFirstTime} />
          <span>Off</span>
        </label>
      </div>
      <p class="muted">Turn this on for quick guidance.</p>
    </section>
  {/if}

  <section class="card">
    <h2>SSH Target</h2>
    <div class="grid-3">
      <label class="field"><span>Host / IP</span><input placeholder="server.example.com or 203.0.113.45" bind:value={host} /></label>
      <label class="field"><span>Port</span><input type="number" min="1" max="65535" bind:value={port} /></label>
      <label class="field"><span>User</span><input placeholder="root" bind:value={user} /></label>
    </div>

    <div class="auth">
      <div class="inline-options">
        <label class="radio"><input type="radio" name="auth" value="password" bind:group={authMode} /><span>Password</span></label>
        <label class="radio"><input type="radio" name="auth" value="keyfile" bind:group={authMode} /><span>Key file</span></label>
        <label class="radio"><input type="radio" name="auth" value="keypaste" bind:group={authMode} /><span>Paste key</span></label>
      </div>

      {#if authMode === "password"}
        <label class="field"><span>Password</span><input type="password" bind:value={password} /></label>
      {:else if authMode === "keyfile"}
        <div class="row">
          <label class="field grow"><span>Private key path</span><input readonly placeholder="Choose private key" bind:value={keyPath} /></label>
          <button class="ghost" type="button" on:click={pickKeyFile}>Choose File</button>
        </div>
      {:else}
        <label class="field"><span>Private key (PEM/OpenSSH)</span><textarea rows="5" bind:value={keyText} placeholder="-----BEGIN OPENSSH PRIVATE KEY----- …"></textarea></label>
      {/if}

      <div class="inline-options" style="margin-top:8px;">
        <label class="toggle"><input type="checkbox" bind:checked={useSameForSudo} /><span>Use same login credentials for sudo</span></label>
        {#if !useSameForSudo}
          <label class="field" style="min-width:260px;">
            <span>Sudo password</span><input type="password" bind:value={sudoPassword} />
          </label>
        {/if}
      </div>
    </div>

    <div class="actions">
      <button class="secondary" type="button" on:click={onTest} disabled={testing || provisioning || checkingRequirements || missingRequirements.length > 0}>
        {testing ? "Testing…" : "Test Connection"}
      </button>
    </div>
  </section>

  <section class="card">
    <h2>Server Secrets</h2>
    <label class="field">
      <span>Server address for credentials</span>
      <input placeholder="http:// your server IP is most common" bind:value={credentialsServerUrl} />
      <span class="hint">Most people use http with an IP address. https or a domain is optional.</span>
    </label>
    <div class="row spaced">
      <label class="field grow">
        <span class="label-row">
          <span>Service account key (JSON)</span>
          <a class="help-link" href="/service-account-help">Where to get this?</a>
        </span>
        <input readonly placeholder="Choose service_account_key.json" bind:value={serviceAccountKeyPath} />
      </label>
      <button class="ghost" type="button" on:click={pickServiceAccountKey}>Choose File</button>
    </div>
    <div class="row spaced">
      <label class="field grow">
        <span>User credentials QR code</span>
        <input readonly placeholder="Choose user_credentials_qr.png" bind:value={userCredentialsQrPath} />
      </label>
      <button class="ghost" type="button" on:click={pickUserCredentialsQrSave}>Choose Save Path</button>
    </div>
    <label class="toggle" style="margin-top:12px;"><input type="checkbox" bind:checked={enableAutoUpdater} /><span>Enable auto-updater service</span></label>
    <label class="toggle" style="margin-top:12px;">
      <input type="checkbox" bind:checked={overwriteInstall} />
      <span>Overwrite existing install (removes /opt/secluso first)</span>
    </label>
  </section>

  {#if errorMsg}<div class="alert error">{errorMsg}</div>{/if}

  <div class="actions bottom">
    <button class="primary" type="button" on:click={onProvision} disabled={provisioning || testing || checkingRequirements || missingRequirements.length > 0}>
      {provisioning ? "Provisioning…" : "Provision Server"}
    </button>
  </div>
</main>

<style>
/* reuse the same styles as other pages */
.wrap { max-width: 980px; margin: 0 auto; padding: 20px 20px 60px; }
.topbar { display: grid; grid-template-columns: 120px 1fr 120px; align-items: center; gap: 12px; margin: 8px 0 18px; }
.topbar h1 { text-align: center; margin: 0; font-size: 1.6rem; }
.spacer { width: 100%; }
.cardhead { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
.card { background: #fff; border: 1px solid #e7e7e7; border-radius: 14px; padding: 16px; margin-bottom: 14px; box-shadow: 0 6px 22px rgba(0,0,0,0.06); }
.card h2 { margin: 0 0 10px 0; font-size: 1.15rem; }
.field { display: flex; flex-direction: column; gap: 6px; }
.field span { color: #333; font-weight: 600; }
.field input, .field textarea { padding: 10px 12px; border-radius: 10px; border: 1px solid #ddd; background: #fff; font-size: 0.98rem; }
.inline-options { display: flex; gap: 16px; flex-wrap: wrap; }
.toggle, .radio { display: inline-flex; gap: 8px; align-items: center; cursor: pointer; }
.toggle input, .radio input { transform: translateY(1px); }
.row { display: flex; gap: 10px; align-items: end; }
.row.spaced { margin-top: 14px; }
.grow { flex: 1; }
.label-row { display: flex; align-items: center; gap: 10px; }
.help-link { font-size: 0.9rem; color: #396cd8; text-decoration: none; }
.help-link:hover { text-decoration: underline; }
.requirements h2 { margin: 0 0 10px 0; }
.req-list { list-style: none; padding: 0; margin: 0; display: grid; gap: 10px; }
.req-item { display: grid; grid-template-columns: 1fr auto; gap: 4px 12px; align-items: center; }
.req-name { font-weight: 600; }
.req-status.missing { color: #b91c1c; font-weight: 700; font-size: 0.92rem; }
.req-detail { grid-column: 1 / -1; color: #666; font-size: 0.9rem; }
.req-steps { margin: 6px 0 10px; padding-left: 18px; color: #555; }
.req-steps li { margin: 4px 0; }
.req-links { display: flex; flex-wrap: wrap; gap: 10px; }
.req-links a { color: #396cd8; text-decoration: none; font-size: 0.95rem; }
.req-links a:hover { text-decoration: underline; }
button { border: 1px solid #d7d7d7; background: #fff; color: #111; padding: 10px 14px; border-radius: 10px; cursor: pointer; }
button:hover { border-color: #c6c6c6; }
button.primary { background: #396cd8; color: #fff; border-color: #396cd8; }
button.primary:hover { filter: brightness(1.05); }
button.secondary { background: #f6f6f6; }
button.ghost { background: #f6f6f6; }
button:disabled { opacity: .6; cursor: not-allowed; }
.back { justify-self: start; }
.actions { margin-top: 10px; display: flex; gap: 12px; align-items: center; }
.actions.bottom { margin-top: 16px; }
.status { color: #444; }
.hint { margin-top: 8px; color: #0f172a; font-size: 0.9rem; font-weight: 700; }
.alert { padding: 10px 12px; border-radius: 10px; border: 1px solid; }
.alert.error { background: #fff4f4; border-color: #ffd6d6; color: #9a1b1b; }
.toggle {
  display: inline-flex;
  gap: 8px;
  align-items: center;
  padding: 8px 10px;
  border: 1px solid #e6e6e6;
  border-radius: 10px;
  background: #fff;
  font-size: 0.9rem;
  color: #111;
}
.toggle input { transform: translateY(1px); }
.quick-steps { margin: 6px 0 0; padding-left: 20px; color: #555; }
.quick-steps li { margin: 4px 0; }
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(2, 6, 23, 0.55);
  display: grid;
  place-items: center;
  z-index: 30;
}
.modal {
  width: min(420px, 90vw);
  background: #fff;
  border: 1px solid #e7e7e7;
  border-radius: 16px;
  padding: 18px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.2);
}
.modal.ok { border-color: #b7f5d6; }
.modal.error { border-color: #ffd6d6; }
.modal-title { font-size: 1.1rem; font-weight: 700; margin-bottom: 6px; color: #0f172a; }
.modal-body { color: #475569; margin-bottom: 14px; }
.modal-btn {
  appearance: none;
  border: 1px solid #d7d7d7;
  background: #fff;
  color: #111;
  border-radius: 10px;
  padding: 10px 14px;
  cursor: pointer;
}
.modal-btn:hover { border-color: #c6c6c6; }
@media (max-width: 860px) { .grid-3 { grid-template-columns: 1fr; } .topbar { grid-template-columns: 1fr auto; } .topbar h1 { text-align:left; } }
@media (prefers-color-scheme: dark) {
  .card { background: #121212; border-color: #2a2a2a; box-shadow: 0 6px 22px rgba(0,0,0,0.4); }
  .field input, .field textarea { background: #0f0f0f; border-color: #2a2a2a; color: #f1f1f1; }
  .toggle input, .radio input { color: #f1f1f1; }
  button { background: #1a1a1a; color: #f1f1f1; border-color: #2a2a2a; }
  button.primary { background: #396cd8; border-color: #396cd8; color: #fff; }
  button.secondary { background: #1a1a1a; }
  button.ghost { background: #141414; }
  .status { color: #dedede; }
  .alert.error { background: #2b1414; border-color: #5a2a2a; color: #ffbdbd; }
  .toggle { background: #111; border-color: #2a2a2a; color: #f1f1f1; }
  .quick-steps { color: #d3d3d3; }
  .modal { background: #121212; border-color: #2a2a2a; box-shadow: 0 20px 60px rgba(0,0,0,0.45); }
  .modal-title { color: #f8fafc; }
  .modal-body { color: #cbd5f5; }
  .modal-btn { background: #1a1a1a; color: #f1f1f1; border-color: #2a2a2a; }
}
</style>
