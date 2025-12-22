<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
  import { onMount } from "svelte";
  import { save } from "@tauri-apps/plugin-dialog";
  import { goto } from "$app/navigation";
  import { buildImage, checkRequirements, type RequirementStatus } from "$lib/api";

  // variants data model
  type VariantKey = "official" | "diy";
  interface VariantDef { value: VariantKey; title: string; subtitle?: string; bullets: string[] }

  const variantDefs: VariantDef[] = [
    {
      value: "official",
      title: "Official",
      subtitle: "Production camera",
      bullets: [
        "LED and button hardware supported.",
        "Night-vision IR auto-toggle service.",
        "SSH disabled; auto-updater enabled.",
        "Production config & indicators."
      ]
    },
    {
      value: "diy",
      title: "DIY",
      subtitle: "Simple Pi setup",
      bullets: [
        "No button, LED, or integrated night-vision controller.",
        "SSH disabled; auto-updater enabled.",
      ]
    }
  ];

  type DevSettings = {
    enabled: boolean;
    wifiSsid: string;
    wifiPsk: string;
    wifiCountry: string;
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

  // config state
  let productVariant: VariantKey = "diy";
  let qrOutputPath = "";           // full file path from the os save dialog
  let imageOutputPath = "";        // full file path from the os save dialog
  let devSettings: DevSettings = {
    enabled: false,
    wifiSsid: "",
    wifiPsk: "",
    wifiCountry: "",
    binariesSource: "main",
    binariesRepo: "",
    key1Name: "",
    key1User: "",
    key2Name: "",
    key2User: "",
    githubToken: ""
  };

  // progress state
  let building = false;
  let errorMsg = "";
  let firstTimeOn = false;
  let requirements: RequirementStatus[] = [];
  let missingRequirements: RequirementStatus[] = [];
  let checkingRequirements = true;
  $: dockerMissing = missingRequirements.some((req) => req.name === "Docker");
  $: buildxMissing = missingRequirements.some((req) => req.name === "Docker Buildx");

  async function pickQrOutput() {
    const path = await save({
      title: "Save pairing QR code as…",
      defaultPath: "camera-qr.png",
      filters: [ { name: "PNG image", extensions: ["png"] } ]
    });
    if (typeof path === "string" && path.length) qrOutputPath = path;
  }

  async function pickImageOutput() {
    const now = new Date();
    const stamp = [
      now.getFullYear(),
      String(now.getMonth() + 1).padStart(2, "0"),
      String(now.getDate()).padStart(2, "0"),
      "-",
      String(now.getHours()).padStart(2, "0"),
      String(now.getMinutes()).padStart(2, "0")
    ].join("");
    const path = await save({
      title: "Save Raspberry Pi image as…",
      defaultPath: `secluso-rpi-${stamp}.img`,
      filters: [ { name: "Disk image", extensions: ["img"] } ]
    });
    if (typeof path === "string" && path.length) imageOutputPath = path;
  }

  function validate(): string | null {
    if (!qrOutputPath) return "Please choose where to save the QR code.";
    if (!imageOutputPath) return "Please choose where to save the image (.img).";
    if (!imageOutputPath.endsWith(".img")) return "Output image must end with .img";
    if (!qrOutputPath.endsWith(".png")) return "QR code must end with .png";
    if (devSettings.enabled) {
      const hasAny = !!(devSettings.wifiSsid || devSettings.wifiPsk || devSettings.wifiCountry);
      const hasAll = !!(devSettings.wifiSsid && devSettings.wifiPsk && devSettings.wifiCountry);
      if (hasAny && !hasAll) {
        return "Developer Wi-Fi needs SSID, password, and country.";
      }
    }
    if (devSettings.enabled && devSettings.binariesSource === "custom") {
      if (!devSettings.binariesRepo.trim()) return "Custom repo URL is required.";
      if (!devSettings.key1Name.trim() || !devSettings.key1User.trim()) {
        return "Key 1 name and GitHub username are required.";
      }
      if (!devSettings.key2Name.trim() || !devSettings.key2User.trim()) {
        return "Key 2 name and GitHub username are required.";
      }
    }
    return null;
  }

  async function startBuild() {
    errorMsg = "";
    if (checkingRequirements) {
      errorMsg = "Checking required tools. Try again in a moment.";
      return;
    }
    if (missingRequirements.length > 0) {
      errorMsg = `Missing required tools: ${missingRequirements.map((req) => req.name).join(", ")}.`;
      return;
    }
    const err = validate();
    if (err) { errorMsg = err; return; }

    building = true;

    try {
      const devWifiEnabled =
        devSettings.enabled &&
        devSettings.wifiSsid.trim() &&
        devSettings.wifiPsk.trim() &&
        devSettings.wifiCountry.trim();

      const { run_id } = await buildImage({
        variant: productVariant,
        qrOutputPath,
        imageOutputPath,
        binariesRepo: devSettings.binariesSource === "custom" ? devSettings.binariesRepo.trim() : undefined,
        githubToken: devSettings.enabled && devSettings.githubToken.trim() ? devSettings.githubToken.trim() : undefined,
        sigKeys:
          devSettings.binariesSource === "custom"
            ? [
                { name: devSettings.key1Name.trim(), githubUser: devSettings.key1User.trim() },
                { name: devSettings.key2Name.trim(), githubUser: devSettings.key2User.trim() }
              ]
            : undefined,
        wifi: devWifiEnabled
          ? {
              ssid: devSettings.wifiSsid.trim(),
              psk: devSettings.wifiPsk.trim(),
              country: devSettings.wifiCountry.trim()
            }
          : undefined
      });
      goto(`/status?mode=image&runId=${encodeURIComponent(run_id)}`);
    } catch (e: any) {
      errorMsg = e?.toString() ?? "Build failed.";
    } finally {
      building = false;
    }
  }

  function goBack() { goto("/"); }

  onMount(() => {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (!raw) return;
    try {
      const parsed = JSON.parse(raw) as Partial<DevSettings>;
      devSettings = { ...devSettings, ...parsed };
    } catch {
        devSettings = {
          enabled: false,
          wifiSsid: "",
          wifiPsk: "",
          wifiCountry: "",
          binariesSource: "main",
          binariesRepo: "",
          key1Name: "",
          key1User: "",
          key2Name: "",
          key2User: "",
          githubToken: ""
        };
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
  <header class="topbar">
    <button class="back" on:click={goBack}>← Back</button>
    <h1>Build Raspberry Pi Image</h1>
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
      <p class="muted">Docker is required for image builds.</p>
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
        <li>Pick a device option. If you are unsure, choose DIY.</li>
        <li>Choose where to save the camera QR code.</li>
        <li>Choose where to save the Raspberry Pi image.</li>
        <li>Click Build Image, then use your normal SD card tool to flash it.</li>
        <li>Start the Raspberry Pi and keep the camera QR code for the app.</li>
        <li>When you are done, go back and set up your server.</li>
      </ol>
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

  <!-- variant selector -->
  <section class="card">
    <h2>Device Variant</h2>
    <div class="variants">
      {#each variantDefs as v}
        <label class="variant {productVariant === v.value ? 'selected' : ''}">
          <input type="radio" name="variant" value={v.value} bind:group={productVariant} />
          <div class="head">
            <div class="title">{v.title}</div>
            {#if v.subtitle}<div class="subtitle">{v.subtitle}</div>{/if}
          </div>
          <ul class="bullets">
            {#each v.bullets as b}
              <li>{b}</li>
            {/each}
          </ul>
        </label>
      {/each}
    </div>
  </section>

  <!-- server url -->
  <!-- qr code output -->
  <section class="card">
    <h2>Camera Secret QR Code</h2>
    <div class="row">
      <label class="field grow">
        <span>Save to</span>
        <input readonly placeholder="Choose file (e.g., camera-qr.png)" bind:value={qrOutputPath} />
      </label>
      <button class="ghost" on:click={pickQrOutput}>Choose File</button>
    </div>
  </section>

  <!-- image output -->
  <section class="card">
    <h2>Output Image</h2>
    <div class="row">
      <label class="field grow">
        <span>Save to</span>
        <input readonly placeholder="Choose file (e.g., secluso-rpi.img)" bind:value={imageOutputPath} />
      </label>
      <button class="ghost" on:click={pickImageOutput}>Choose File</button>
    </div>
  </section>

  {#if errorMsg}
    <div class="alert error">{errorMsg}</div>
  {/if}

  <div class="actions">
    <button class="primary" disabled={building || checkingRequirements || missingRequirements.length > 0} on:click={startBuild}>
      {building ? "Building…" : "Build Image"}
    </button>
  </div>
</main>

<style>
/* layout */
.wrap { max-width: 980px; margin: 0 auto; padding: 20px 20px 60px; }
.topbar { display: grid; grid-template-columns: 120px 1fr 120px; align-items: center; gap: 12px; margin: 8px 0 18px; }
.topbar h1 { text-align: center; margin: 0; font-size: 1.6rem; }
.spacer { width: 100%; }
.cardhead { display: flex; align-items: center; justify-content: space-between; gap: 12px; }

/* cards */
.card { background: #fff; border: 1px solid #e7e7e7; border-radius: 14px; padding: 16px; margin-bottom: 14px; box-shadow: 0 6px 22px rgba(0,0,0,0.06); }
.card h2 { margin: 0 0 10px 0; font-size: 1.15rem; }

/* variant selector */
.variants { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 12px; }
.variant { display: grid; gap: 8px; border: 1px solid #e2e2e2; border-radius: 12px; padding: 12px; cursor: pointer; background: #fafafa; transition: border-color .15s ease, box-shadow .15s ease, background .15s ease; }
.variant:hover { border-color: #cfd6ff; box-shadow: 0 2px 12px rgba(57,108,216,0.15); }
.variant.selected { border-color: #396cd8; background: #f3f7ff; box-shadow: 0 3px 14px rgba(57,108,216,0.2); }
.variant input { position: absolute; opacity: 0; pointer-events: none; }
.variant .head { display: flex; flex-direction: column; gap: 2px; }
.variant .title { font-weight: 700; }
.variant .subtitle { color: #666; font-size: .92rem; }
/* lighter bullets to match landing page */
.variant .bullets { margin: 0; padding-left: 18px; color: #777; line-height: 1.5; font-size: .96rem; }
.variant .bullets li { margin: 2px 0; }

/* fields */
.field { display: flex; flex-direction: column; gap: 6px; }
.field span { color: #333; font-weight: 600; }
.field input { padding: 10px 12px; border-radius: 10px; border: 1px solid #ddd; background: #fff; font-size: 0.98rem; }

/* layout helpers */
.row { display: flex; gap: 10px; align-items: end; }
.grow { flex: 1; }

/* requirements */
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

/* buttons */
button { border: 1px solid #d7d7d7; background: #fff; color: #111; padding: 10px 14px; border-radius: 10px; cursor: pointer; }
button:hover { border-color: #c6c6c6; }
button:disabled { opacity: 0.6; cursor: not-allowed; }
button.primary { background: #396cd8; color: #fff; border-color: #396cd8; }
button.primary:hover { filter: brightness(1.05); }
button.ghost { background: #f6f6f6; }

/* back */
.back { justify-self: start; }

/* actions */
.actions { margin-top: 14px; display: flex; gap: 12px; align-items: center; }

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

@media (max-width: 720px) {
  .row { flex-direction: column; align-items: stretch; }
}

/* alerts */
.alert { padding: 10px 12px; border-radius: 10px; border: 1px solid; }
.alert.error { background: #fff4f4; border-color: #ffd6d6; color: #9a1b1b; }

@media (prefers-color-scheme: dark) {
  .card { background: #121212; border-color: #2a2a2a; box-shadow: 0 6px 22px rgba(0,0,0,0.4); }
  .variant { background: #161616; border-color: #2a2a2a; }
  .variant:hover { border-color: #405596; box-shadow: 0 2px 12px rgba(57,108,216,0.25); }
  .variant.selected { background: #0f1526; border-color: #5f82ff; }
  /* lighter bullets in dark mode like the landing page */
  .variant .bullets { color: #b9c2d6; }
  .field input { background: #0f0f0f; border-color: #2a2a2a; color: #f1f1f1; }
  button { background: #1a1a1a; color: #f1f1f1; border-color: #2a2a2a; }
  button.ghost { background: #141414; }
  .toggle { background: #111; border-color: #2a2a2a; color: #f1f1f1; }
  .quick-steps { color: #d3d3d3; }

  .alert.error { background: #2b1414; border-color: #5a2a2a; color: #ffbdbd; }
}
</style>
