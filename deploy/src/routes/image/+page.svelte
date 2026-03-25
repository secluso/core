<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
  import { onMount } from "svelte";
  import { save } from "@tauri-apps/plugin-dialog";
  import { goto } from "$app/navigation";
  import { browser } from "$app/environment";
  import { buildImage, checkDocker, checkRequirements, openExternalUrl, type RequirementStatus } from "$lib/api";

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
        "Auto-updater enabled.",
        "Production config & indicators."
      ]
    },
    {
      value: "diy",
      title: "DIY",
      subtitle: "Simple Pi setup",
      bullets: [
        "No button, LED, or integrated night-vision controller.",
        "Auto-updater enabled.",
      ]
    }
  ];

  type DevSettings = {
    enabled: boolean;
    cache: boolean;
    wifiSsid: string;
    wifiPsk: string;
    wifiCountry: string;
    sshEnabled: boolean;
    binariesSource: "main" | "custom";
    binariesRepo: string;
    key1Name: string;
    key1User: string;
    key2Name: string;
    key2User: string;
    githubToken: string;
    showDockerHelp: boolean;
  };

  const SETTINGS_KEY = "secluso-dev-settings";
  const FIRST_TIME_KEY = "secluso-first-time";
  const imageBackIcon = "/deploy-assets/image-back-latest.svg";
  const imageHeroArt = "/deploy-assets/image-hero-latest.svg";
  const officialIcon = "/deploy-assets/image-official-icon-latest.svg";
  const diyIcon = "/deploy-assets/image-diy-icon-latest.svg";
  const selectedIcon = "/deploy-assets/image-selected-icon-latest.svg";
  const tipIcon = "/deploy-assets/image-tip-icon-latest.svg";
  const imageLocationIcon = "/deploy-assets/image-output-icon-latest.svg";
  const pickerIcon = "/deploy-assets/image-picker-icon-latest.svg";
  const qrLocationIcon = "/deploy-assets/image-qr-icon-latest.svg";
  const outputHelpIcon = "/deploy-assets/image-output-help-icon-latest.svg";
  const buildArrowIcon = "/deploy-assets/image-build-arrow-latest.svg";

  // config state
  let productVariant: VariantKey = "diy";
  let qrOutputPath = "";           // full file path from the os save dialog
  let imageOutputPath = "";        // full file path from the os save dialog
  let devSettings: DevSettings = {
    enabled: false,
    cache: false,
    wifiSsid: "",
    wifiPsk: "",
    wifiCountry: "",
    sshEnabled: true,
    binariesSource: "main",
    binariesRepo: "",
    key1Name: "",
    key1User: "",
    key2Name: "",
    key2User: "",
    githubToken: "",
    showDockerHelp: false
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
  $: showDockerHelp = dockerMissing || buildxMissing || (devSettings.enabled && devSettings.showDockerHelp);
  $: sshStatusText = !devSettings.enabled
    ? "SSH disabled."
    : devSettings.sshEnabled
    ? "SSH enabled (developer options)."
    : "SSH disabled (developer options).";
  $: imageOutputPlaceholder = effectiveSshEnabled()
    ? "Choose file (e.g., secluso-rpi-ssh-enabled.img)"
    : "Choose file (e.g., secluso-rpi.img)";

  function effectiveSshEnabled(): boolean {
    return devSettings.enabled && devSettings.sshEnabled;
  }

  function normalizeSshSuffix(path: string, sshEnabled: boolean): string {
    if (!path.endsWith(".img")) return path;
    if (sshEnabled) {
      if (path.endsWith("-ssh-enabled.img")) return path;
      return `${path.slice(0, -4)}-ssh-enabled.img`;
    }
    if (path.endsWith("-ssh-enabled.img")) {
      return `${path.slice(0, -"-ssh-enabled.img".length)}.img`;
    }
    return path;
  }

  $: if (imageOutputPath) {
    const normalized = normalizeSshSuffix(imageOutputPath, effectiveSshEnabled());
    if (normalized !== imageOutputPath) {
      imageOutputPath = normalized;
    }
  }

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
    const defaultPath = normalizeSshSuffix(`secluso-rpi-${stamp}.img`, effectiveSshEnabled());
    const path = await save({
      title: "Save Raspberry Pi image as…",
      defaultPath,
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
      const dockerStatus = await checkDocker();
      if (!dockerStatus.ok) {
        errorMsg = dockerStatus.message ?? "Docker is installed, but the Docker daemon is not reachable. Start Docker and try again.";
        return;
      }

      const sshEnabled = effectiveSshEnabled();
      const outputWithSuffix = normalizeSshSuffix(imageOutputPath, sshEnabled);
      if (outputWithSuffix !== imageOutputPath) {
        imageOutputPath = outputWithSuffix;
      }

      const devWifiEnabled =
        devSettings.enabled &&
        devSettings.wifiSsid.trim() &&
        devSettings.wifiPsk.trim() &&
        devSettings.wifiCountry.trim();

      const { run_id } = await buildImage({
        variant: productVariant,
        cache: devSettings.cache,
        qrOutputPath,
        imageOutputPath: outputWithSuffix,
        sshEnabled,
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
          cache: false,
          wifiSsid: "",
          wifiPsk: "",
          wifiCountry: "",
          sshEnabled: true,
          binariesSource: "main",
          binariesRepo: "",
          key1Name: "",
          key1User: "",
          key2Name: "",
          key2User: "",
          githubToken: "",
          showDockerHelp: false
        };
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

  async function openExternal(url: string) {
    if (!browser) return;
    try {
      await openExternalUrl(url);
    } catch (err) {
      console.warn("Failed to open external link via shell opener.", err);
      window.open(url, "_blank", "noopener,noreferrer");
    }
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

<main class="page">
  <div class="backdrop"></div>

  <header class="appbar">
    <div class="appbar-inner">
      <div class="brand">
        <img src="/deploy-assets/header-mark.jpeg" alt="" />
        <span>Secluso</span>
        <small>v0.1.0</small>
        <span class="status-pill"><i></i>Latest Version</span>
      </div>
    </div>
  </header>

  <section class="frame">
    <div class="toolbar">
      <button class="back-link" on:click={goBack}>
        <img src={imageBackIcon} alt="" />
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

    <div class="step-pill">Step 1</div>

    <div class="hero">
      <div class="hero-copy">
        <h1>Build Raspberry Pi Image</h1>
        <p>Generate a custom Pi OS image pre-configured with your encryption keys.</p>
      </div>
      <img class="hero-art" src={imageHeroArt} alt="" />
    </div>

    <section class="section-block">
      <div class="label">Hardware Type</div>

      {#each variantDefs as v}
        <label class="hardware-card {productVariant === v.value ? 'selected' : ''}">
          <input type="radio" name="variant" value={v.value} bind:group={productVariant} />
          <span class="hardware-icon {v.value}">
            <img src={v.value === "official" ? officialIcon : diyIcon} alt="" />
          </span>
          <span class="hardware-copy">
            <strong>{v.value === "official" ? "Official Hardware" : "DIY Setup"}</strong>
            <small>{v.value === "official" ? "LED, night vision, hardware buttons" : "Any Raspberry Pi with camera module"}</small>
          </span>
          <span class="hardware-check">
            {#if productVariant === v.value}
              <span class="selected-pill"><img src={selectedIcon} alt="" /></span>
            {:else}
              <span class="empty-pill"></span>
            {/if}
          </span>
        </label>
      {/each}

      {#if firstTimeOn}
        <section class="tip-banner">
          <img src={tipIcon} alt="" />
          <p>
            Choose <span>Official</span> if you bought a Secluso camera. Choose <span>DIY</span> for custom Pi builds.
          </p>
        </section>
      {/if}
    </section>

    <section class="section-block outputs">
      <div class="label">Output Locations</div>

      <div class="output-row">
        <div class="field-label">
          <img src={imageLocationIcon} alt="" />
          <span>Save Pi image (.img) to</span>
        </div>
        <div class="output-picker">
          <div class="output-input">
            <input readonly placeholder={imageOutputPlaceholder} bind:value={imageOutputPath} />
          </div>
          <button class="picker-button" on:click={pickImageOutput} aria-label="Choose image output path">
            <img src={pickerIcon} alt="" />
          </button>
        </div>
      </div>

      <div class="output-row">
        <div class="field-label">
          <img src={qrLocationIcon} alt="" />
          <span>Save camera QR code (.png) to</span>
        </div>
        <div class="output-picker">
          <div class="output-input">
            <input readonly placeholder="Choose where to save the QR code..." bind:value={qrOutputPath} />
          </div>
          <button class="picker-button" on:click={pickQrOutput} aria-label="Choose QR output path">
            <img src={pickerIcon} alt="" />
          </button>
        </div>
      </div>

      {#if firstTimeOn}
        <div class="info-banner">
          <img src={outputHelpIcon} alt="" />
          <p>The <span>.img file</span> is flashed to your SD card. The <span>QR code</span> is scanned by the mobile app to connect securely.</p>
        </div>
      {/if}
    </section>

    {#if checkingRequirements}
      <section class="status-panel">
        <p>Checking local tools…</p>
      </section>
    {:else if missingRequirements.length > 0}
      <section class="status-panel error-panel">
        <ul class="req-list">
          {#each missingRequirements as req}
            <li><strong>{req.name}:</strong> {req.hint}</li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if showDockerHelp}
      <section class="status-panel">
        <p>Docker is required for image builds.</p>
        <div class="help-links">
          <a href="https://docs.docker.com/desktop/install/windows-install/" on:click|preventDefault={() => openExternal("https://docs.docker.com/desktop/install/windows-install/")}>Windows</a>
          <a href="https://docs.docker.com/desktop/install/mac-install/" on:click|preventDefault={() => openExternal("https://docs.docker.com/desktop/install/mac-install/")}>macOS</a>
          <a href="https://docs.docker.com/engine/install/" on:click|preventDefault={() => openExternal("https://docs.docker.com/engine/install/")}>Linux</a>
        </div>
      </section>
    {/if}

    {#if errorMsg}
      <div class="alert error">{errorMsg}</div>
    {/if}

    <button class="primary" disabled={building || checkingRequirements || missingRequirements.length > 0} on:click={startBuild}>
      <span>{building ? "Building…" : "Build Image"}</span>
      <img src={buildArrowIcon} alt="" />
    </button>
    <p class="caption">This generates a downloadable .img file for Raspberry Pi Imager</p>
  </section>
</main>

<style>
  :global(body) {
    margin: 0;
    background: #030303;
    color: #fff;
    font-family: Inter, "Segoe UI", sans-serif;
  }

  :global(*) {
    box-sizing: border-box;
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
      radial-gradient(780px 420px at 50% 132px, rgba(255, 255, 255, 0.016), transparent 68%),
      linear-gradient(180deg, rgba(3, 3, 3, 0.98), #030303 46%);
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

  .appbar-inner {
    max-width: 672px;
    width: 100%;
    height: 100%;
    margin: 0 auto;
    padding: 0 24px;
    display: flex;
    align-items: center;
    justify-content: flex-start;
    box-sizing: border-box;
  }

  .brand {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: #fff;
    font-size: 14px;
    font-weight: 500;
    line-height: 21px;
  }

  .brand img {
    width: 28px;
    height: 28px;
    border-radius: 16px;
    box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.06);
  }

  .brand small {
    color: rgba(255, 255, 255, 0.25);
    font-size: 11px;
    font-weight: 500;
    line-height: 16.5px;
  }

  .status-pill {
    height: 25px;
    padding: 0 9px 0 8px;
    border-radius: 999px;
    background: rgba(0, 188, 125, 0.08);
    border: 1px solid rgba(0, 188, 125, 0.1);
    color: rgba(0, 212, 146, 0.92);
    font-size: 10px;
    font-weight: 500;
    line-height: 15px;
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .status-pill i {
    width: 6px;
    height: 6px;
    border-radius: 999px;
    background: #00d492;
    display: block;
    font-style: normal;
  }

  .frame {
    position: relative;
    z-index: 1;
    width: min(calc(100% - 48px), 528px);
    margin: 0 auto;
  }

  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .tips-toggle {
    display: inline-flex;
    align-items: center;
    gap: 12px;
    color: rgba(255, 255, 255, 0.3);
    font-size: 11px;
    line-height: 16.5px;
  }

  .back-link {
    border: 0;
    background: transparent;
    padding: 0;
    color: rgba(255, 255, 255, 0.4);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font: inherit;
    font-size: 13px;
    line-height: 19.5px;
  }

  .back-link img {
    width: 14px;
    height: 14px;
    display: block;
  }

  .tips-switch {
    position: relative;
    width: 24px;
    height: 13.8px;
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
    background: #2b7fff;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
  }

  .tips-track::after {
    content: "";
    position: absolute;
    top: 0.9px;
    left: 10.25px;
    width: 12px;
    height: 12px;
    border-radius: 999px;
    background: #030303;
  }

  .step-pill {
    display: inline-flex;
    align-items: center;
    height: 19px;
    margin-top: 39px;
    padding: 0 8px;
    border-radius: 14px;
    background: rgba(43, 127, 255, 0.1);
    color: #51a2ff;
    text-transform: uppercase;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.5px;
    line-height: 15px;
  }

  .hero {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 18px;
    margin-top: 12px;
  }

  h1 {
    margin: 0;
    font-size: 24px;
    line-height: 32px;
    font-weight: 600;
  }

  .hero p {
    margin: 12px 0 0;
    max-width: 492px;
    color: rgba(255, 255, 255, 0.4);
    font-size: 14px;
    line-height: 22.75px;
  }

  .hero-art {
    width: 160px;
    height: 160px;
    margin-top: -31px;
    margin-right: -16px;
    flex: 0 0 auto;
  }

  .label {
    color: rgba(255, 255, 255, 0.36);
    text-transform: uppercase;
    letter-spacing: 0.55px;
    font-size: 11px;
    font-weight: 500;
    line-height: 16.5px;
    margin-bottom: 12px;
  }

  .section-block {
    margin-top: 24px;
  }

  .hardware-card {
    position: relative;
    display: grid;
    grid-template-columns: 40px 1fr auto;
    align-items: center;
    gap: 16px;
    min-height: 83px;
    padding: 0 20px;
    border-radius: 20px;
    border: 1px solid rgba(255, 255, 255, 0.04);
    background: rgba(255, 255, 255, 0.02);
    cursor: pointer;
  }

  .hardware-card + .hardware-card {
    margin-top: 16px;
  }

  .hardware-card.selected {
    background: rgba(59, 130, 246, 0.06);
    border-color: rgba(59, 130, 246, 0.25);
  }

  .hardware-card input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .hardware-icon {
    width: 40px;
    height: 40px;
    border-radius: 12px;
    display: grid;
    place-items: center;
    background: rgba(255, 255, 255, 0.03);
  }

  .hardware-icon img {
    width: 20px;
    height: 20px;
    display: block;
  }

  .hardware-icon.diy {
    background: rgba(43, 127, 255, 0.15);
  }

  .hardware-copy {
    display: grid;
    gap: 5px;
  }

  .hardware-copy strong {
    font-size: 14px;
    line-height: 21px;
    font-weight: 500;
  }

  .hardware-copy small {
    color: rgba(255, 255, 255, 0.4);
    font-size: 12px;
    line-height: 18px;
  }

  .selected-pill,
  .empty-pill {
    width: 20px;
    height: 20px;
    border-radius: 999px;
    display: grid;
    place-items: center;
  }

  .selected-pill {
    background: #2b7fff;
  }

  .selected-pill img {
    width: 12px;
    height: 12px;
    display: block;
  }

  .empty-pill {
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .tip-banner {
    margin-top: 16px;
    min-height: 45.5px;
    padding: 0 12px;
    border-radius: 16px;
    border: 1px solid rgba(43, 127, 255, 0.1);
    background: rgba(43, 127, 255, 0.05);
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
  }

  .tip-banner img {
    width: 16px;
    height: 16px;
    display: block;
    flex: 0 0 auto;
  }

  .tip-banner p {
    margin: 0;
    color: rgba(255, 255, 255, 0.5);
    font-size: 12px;
    line-height: 19.5px;
  }

  .tip-banner span {
    color: rgba(255, 255, 255, 0.7);
  }

  .outputs {
    margin-top: 42px;
  }

  .output-row + .output-row {
    margin-top: 20px;
  }

  .field-label {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 10px;
    color: rgba(255, 255, 255, 0.5);
    font-size: 12px;
    line-height: 18px;
  }

  .field-label img {
    width: 14px;
    height: 14px;
    display: block;
  }

  .output-picker {
    display: grid;
    grid-template-columns: 1fr 50px;
    gap: 8px;
  }

  .output-input {
    height: 41.5px;
    border-radius: 16px;
    border: 1px solid rgba(255, 255, 255, 0.06);
    background: rgba(255, 255, 255, 0.03);
    overflow: hidden;
    display: flex;
    align-items: center;
    padding: 0 12px;
  }

  .output-input input {
    width: 100%;
    height: 15.5px;
    padding: 0;
    border: 0;
    background: transparent;
    color: rgba(255, 255, 255, 0.88);
    font: inherit;
    font-size: 13px;
    line-height: 15.5px;
  }

  .output-input input::placeholder {
    color: rgba(255, 255, 255, 0.25);
  }

  .picker-button {
    width: 50px;
    height: 41.5px;
    border-radius: 16px;
    border: 1px solid rgba(255, 255, 255, 0.06);
    background: rgba(255, 255, 255, 0.04);
    display: grid;
    place-items: center;
    cursor: pointer;
  }

  .picker-button img {
    width: 16px;
    height: 16px;
    display: block;
  }

  .info-banner {
    margin-top: 20px;
    min-height: 65px;
    padding: 12px;
    border-radius: 16px;
    border: 1px solid rgba(255, 255, 255, 0.04);
    background: rgba(255, 255, 255, 0.02);
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }

  .info-banner img {
    width: 16px;
    height: 16px;
    display: block;
    margin-top: 2px;
    flex: 0 0 auto;
  }

  .info-banner p {
    margin: 0;
    color: rgba(255, 255, 255, 0.4);
    font-size: 12px;
    line-height: 19.5px;
  }

  .info-banner span {
    color: rgba(255, 255, 255, 0.6);
  }

  .status-panel {
    margin-top: 18px;
    padding: 14px 16px;
    border-radius: 16px;
    border: 1px solid rgba(255, 255, 255, 0.04);
    background: rgba(255, 255, 255, 0.02);
  }

  .status-panel p {
    margin: 0;
    color: rgba(255, 255, 255, 0.55);
    font-size: 13px;
    line-height: 19.5px;
  }

  .req-list {
    margin: 0;
    padding-left: 18px;
    color: rgba(255, 255, 255, 0.65);
    font-size: 13px;
    line-height: 19.5px;
  }

  .req-list li + li {
    margin-top: 4px;
  }

  .help-links {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    margin-top: 10px;
  }

  .help-links a {
    color: #51a2ff;
    text-decoration: none;
    font-size: 12px;
    line-height: 18px;
  }

  .error-panel {
    border-color: rgba(248, 113, 113, 0.2);
    background: rgba(127, 29, 29, 0.16);
  }

  .primary {
    width: 100%;
    height: 49px;
    margin-top: 28px;
    border-radius: 20px;
    border: none;
    background: #3b82f6;
    color: #fff;
    font: inherit;
    font-size: 14px;
    line-height: 21px;
    font-weight: 500;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    cursor: pointer;
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

  .caption {
    margin: 12px 0 0;
    text-align: center;
    color: rgba(255, 255, 255, 0.25);
    font-size: 11px;
    line-height: 16.5px;
  }

  .alert {
    margin-top: 18px;
    padding: 12px 14px;
    border-radius: 16px;
    border: 1px solid rgba(248, 113, 113, 0.24);
    background: rgba(127, 29, 29, 0.25);
    color: #fecaca;
    font-size: 13px;
    line-height: 19.5px;
  }

  @media (max-width: 640px) {
    .appbar-inner {
      padding-inline: 14px;
    }

    .frame {
      width: calc(100% - 28px);
    }

    .hero {
      gap: 10px;
    }

    .hero-art {
      width: 128px;
      height: 128px;
      margin-top: -18px;
      margin-right: -10px;
    }
  }
</style>
