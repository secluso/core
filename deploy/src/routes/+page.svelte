<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
  import { onMount } from "svelte";
  import { openExternalUrl } from "$lib/api";

  const FIRST_TIME_KEY = "secluso-first-time";
  const homeBackdrop = "/deploy-assets/home-backdrop-latest.svg";
  const homeMark = "/deploy-assets/home-logo.jpeg";
  const homeSettings = "/deploy-assets/home-settings-latest.svg";
  const homeAppStore = "/deploy-assets/home-app-store-latest.svg";
  const homeGooglePlay = "/deploy-assets/home-google-play-latest.svg";
  const homeSignal = "/deploy-assets/home-signal-latest.svg";
  const homeShield = "/deploy-assets/home-chip-shield-latest.svg";
  const homeZap = "/deploy-assets/home-chip-zap-latest.svg";
  const homeStepOneBg = "/deploy-assets/home-step-1-bg-latest.svg";
  const homeStepTwoBg = "/deploy-assets/home-step-2-bg-latest.svg";
  const homeStepArrow = "/deploy-assets/home-step-arrow-latest.svg";
  const homeStepOneIcon = "/deploy-assets/home-step-1-icon-latest.svg";
  const homeStepTwoIcon = "/deploy-assets/home-step-2-icon-latest.svg";

  let firstTimeOn = false;

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

  async function openExternal(url: string) {
    try {
      await openExternalUrl(url);
    } catch {
      window.open(url, "_blank", "noopener,noreferrer");
    }
  }
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
      <a class="settings-btn" href="/settings" aria-label="Settings">
        <img src={homeSettings} alt="" />
      </a>
    </div>
  </header>

  <section class="hero">
    <div class="hero-inner">
      <div class="logo-stage">
        <img class="logo-glow" src={homeBackdrop} alt="" />
        <div class="mark-shell">
          <img class="mark" src={homeMark} alt="" />
        </div>
        <span class="signal"><img src={homeSignal} alt="" /></span>
      </div>

      <h1>Secluso Deploy</h1>
      <p class="lead">Get your encrypted camera system online in<br />just two easy steps</p>

      <div class="chips">
        <span class="chip chip-encrypted"><img src={homeShield} alt="" />End-to-End Encrypted</span>
        <span class="chip chip-fast"><img src={homeZap} alt="" />2 Minute Setup</span>
      </div>

      <div class="store-links">
        <a class="store-btn app-store" href="https://apps.apple.com/app/id0000000000" on:click|preventDefault={() => openExternal("https://apps.apple.com/app/id0000000000")}>
          <span class="store-badge"><img src={homeAppStore} alt="" /></span>
          <span class="store-copy"><small>Download on</small><strong>App Store</strong></span>
        </a>
        <a class="store-btn play-store" href="https://play.google.com/store/apps/details?id=com.secluso.mobile" on:click|preventDefault={() => openExternal("https://play.google.com/store/apps/details?id=com.secluso.mobile")}>
          <span class="store-badge"><img src={homeGooglePlay} alt="" /></span>
          <span class="store-copy"><small>Get it on</small><strong>Google Play</strong></span>
        </a>
      </div>
    </div>
  </section>

  <section class="toggle-strip" role="button" tabindex="0" aria-pressed={firstTimeOn} on:click={onToggleCardClick} on:keydown={onToggleKey}>
    <div>
      <div class="toggle-title">First time setting up?</div>
      <p class="toggle-copy">Enable step-by-step guidance</p>
    </div>
    <label class="toggle" aria-label="Enable step-by-step guidance">
      <input type="checkbox" checked={firstTimeOn} on:change={toggleFirstTime} />
    </label>
  </section>

  {#if firstTimeOn}
    <section class="help-panel">
      <ol class="quick-steps">
        <li>Install the Secluso app on your phone.</li>
        <li>Build the Raspberry Pi image and keep the camera QR code.</li>
        <li>Provision your Linux server and save the user credentials QR code.</li>
        <li>Scan the server QR code in the app, then scan the camera QR code.</li>
      </ol>
      <div class="help-links">
        <a class="help-link" href="/hardware-help" on:click={setHelpRef}>Recommended hardware guide</a>
        <a class="help-link" href="/ionos-help" on:click={setHelpRef}>Ionos VPS setup guide</a>
      </div>
    </section>
  {/if}

  <section class="steps-shell">
    <div class="section-heading">Setup Steps</div>

    <a class="step-card" href="/image">
      <img class="step-bg" src={homeStepOneBg} alt="" />
      <div class="step-icon-wrap step-one">
        <div class="step-icon"><img src={homeStepOneIcon} alt="" /></div>
        <span class="step-badge">1</span>
      </div>
      <div class="step-body">
        <div class="step-title-row">
          <h2>Raspberry Pi</h2>
          <span>Build image</span>
        </div>
        <p>Generate a Pi OS image with encryption keys pre-configured.</p>
      </div>
      <span class="step-arrow"><img src={homeStepArrow} alt="" /></span>
    </a>

    <a class="step-card" href="/server-ssh">
      <img class="step-bg" src={homeStepTwoBg} alt="" />
      <div class="step-icon-wrap step-two">
        <div class="step-icon"><img src={homeStepTwoIcon} alt="" /></div>
        <span class="step-badge">2</span>
      </div>
      <div class="step-body">
        <div class="step-title-row">
          <h2>Server</h2>
          <span>Deploy via SSH</span>
        </div>
        <p>Install Secluso on any Linux machine via SSH.</p>
      </div>
      <span class="step-arrow"><img src={homeStepArrow} alt="" /></span>
    </a>

    <p class="footnote">New device? Start with Step 1. Pi already running? Go to Step 2.</p>
  </section>
</main>

<style>
  :global(body) {
    margin: 0;
    background: #030303;
    color: #fff;
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
      radial-gradient(780px 420px at 50% 132px, rgba(255, 255, 255, 0.016), transparent 68%),
      linear-gradient(180deg, rgba(3, 3, 3, 0.98), #030303 46%);
  }

  .appbar {
    height: 57px;
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
    justify-content: space-between;
    position: relative;
    box-sizing: border-box;
  }

  .hero {
    position: relative;
    z-index: 1;
    padding: 32px 24px 0;
  }

  .hero-inner {
    position: relative;
    width: min(100%, 576px);
    margin: 0 auto;
    text-align: center;
  }

  .toggle-strip,
  .help-panel,
  .steps-shell {
    position: relative;
    z-index: 1;
    max-width: 528px;
    margin: 0 auto;
    padding: 0 24px;
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

  .settings-btn {
    flex: 0 0 auto;
    width: 16px;
    height: 16px;
    padding: 0;
    border: 0;
    background: transparent;
    display: grid;
    place-items: center;
    opacity: 0.7;
  }

  .settings-btn img {
    width: 16px;
    height: 16px;
    display: block;
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

  .logo-stage {
    position: relative;
    width: 576px;
    max-width: 100%;
    height: 347.25px;
    margin: 0 auto -146px;
  }

  .logo-glow {
    position: absolute;
    left: 50%;
    top: -26.37px;
    width: 400px;
    height: 400px;
    transform: translateX(-50%);
    object-fit: contain;
    opacity: 1;
  }

  .mark-shell {
    position: absolute;
    left: 50%;
    top: 80px;
    width: 80px;
    height: 80px;
    transform: translateX(-50%);
    border-radius: 24px;
    box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.1), 0 25px 50px -12px rgba(43, 127, 255, 0.1);
    overflow: hidden;
  }

  .mark {
    width: 80px;
    height: 80px;
    border-radius: 24px;
    display: block;
  }

  .signal {
    position: absolute;
    left: calc(50% + 20px);
    top: 140px;
    width: 24px;
    height: 24px;
    border-radius: 999px;
    background: #00bc7d;
    box-shadow: 0 0 0 4px #030303;
    display: grid;
    place-items: center;
  }

  .signal img {
    width: 12px;
    height: 12px;
    display: block;
  }

  h1 {
    margin: 0;
    font-size: 48px;
    line-height: 48px;
    font-weight: 700;
    letter-spacing: -1.2px;
    background: linear-gradient(180deg, #fff 0%, #fff 50%, rgba(255, 255, 255, 0.4) 100%);
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
  }

  .lead {
    margin: 10px 0 0;
    color: rgba(255, 255, 255, 0.4);
    font-size: 15px;
    line-height: 24.38px;
    width: min(100%, 314px);
    margin-inline: auto;
  }

  .chips {
    display: flex;
    justify-content: center;
    gap: 12px;
    margin-top: 38px;
  }

  .chip {
    height: 30.5px;
    border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.05);
    background: rgba(255, 255, 255, 0.03);
    color: rgba(255, 255, 255, 0.5);
    font-size: 11px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
  }

  .chip-encrypted { width: 160.26px; }
  .chip-fast { width: 122.72px; }

  .chip img {
    width: 12px;
    height: 12px;
    display: block;
  }

  .store-links {
    display: flex;
    justify-content: center;
    gap: 12px;
    margin-top: 59px;
  }

  .store-btn {
    height: 51.25px;
    border-radius: 20px;
    display: inline-flex;
    align-items: center;
    gap: 12px;
    padding: 0 20px;
    text-decoration: none;
    border: 1px solid rgba(255, 255, 255, 0.06);
    background: rgba(255, 255, 255, 0.03);
    color: #fff;
  }

  .app-store { width: 143.81px; }
  .play-store { width: 148.75px; }

  .app-store {
    background: #fff;
    color: #000;
  }

  .app-store .store-badge {
    color: #000;
    background: transparent;
  }

  .store-badge {
    width: 20px;
    height: 20px;
    display: grid;
    place-items: center;
    flex: 0 0 auto;
  }

  .store-badge img {
    width: 20px;
    height: 20px;
    display: block;
  }

  .store-copy {
    display: grid;
    text-align: left;
  }

  .store-copy small {
    font-size: 9px;
    letter-spacing: 0.225px;
    text-transform: uppercase;
    opacity: 0.5;
    line-height: 9px;
    white-space: nowrap;
  }

  .store-copy strong {
    font-size: 13px;
    line-height: 16.25px;
    font-weight: 600;
    white-space: nowrap;
  }

  .play-store .store-copy {
    transform: translateY(1px);
  }

  .toggle-strip,
  .help-panel,
  .steps-shell {
    margin-top: 40px;
  }

  .toggle-strip,
  .help-panel,
  .step-card {
    border: 1px solid rgba(255, 255, 255, 0.04);
    background: rgba(255, 255, 255, 0.02);
  }

  .toggle-strip {
    height: 78px;
    border-radius: 20px;
    padding: 0 17px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    cursor: pointer;
  }

  .toggle-title {
    color: rgba(255, 255, 255, 0.7);
    font-size: 13px;
    line-height: 19.5px;
    font-weight: 500;
  }

  .toggle-copy {
    margin: 4px 0 0;
    color: rgba(255, 255, 255, 0.3);
    font-size: 12px;
    line-height: 18px;
  }

  .toggle input {
    appearance: none;
    width: 32px;
    height: 18.4px;
    margin: 0;
    border-radius: 999px;
    border: 1px solid transparent;
    background: rgba(255, 255, 255, 0.05);
    position: relative;
  }

  .toggle input::after {
    content: "";
    position: absolute;
    top: 1.2px;
    left: 0;
    width: 16px;
    height: 16px;
    border-radius: 999px;
    background: #030303;
    transition: transform 120ms ease;
  }

  .toggle input:checked {
    background: #2b7fff;
  }

  .toggle input:checked::after {
    transform: translateX(15px);
  }

  .help-panel {
    border-radius: 20px;
    padding: 18px 18px 16px;
  }

  .quick-steps {
    margin: 0;
    padding-left: 20px;
    color: rgba(255, 255, 255, 0.55);
    font-size: 12px;
    line-height: 19.5px;
  }

  .help-links {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
    margin-top: 16px;
  }

  .help-link {
    color: #51a2ff;
    text-decoration: none;
    font-size: 11px;
    line-height: 16.5px;
  }

  .section-heading {
    color: rgba(255, 255, 255, 0.3);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.55px;
    text-transform: uppercase;
    margin-bottom: 18px;
  }

  .step-card {
    height: 78px;
    border-radius: 20px;
    padding: 0 16px;
    display: flex;
    align-items: center;
    gap: 16px;
    text-decoration: none;
    color: inherit;
    margin-bottom: 12px;
    overflow: hidden;
    position: relative;
  }

  .step-bg {
    position: absolute;
    right: -24px;
    bottom: -24px;
    width: 128px;
    height: 128px;
    pointer-events: none;
  }

  .step-icon-wrap {
    width: 44px;
    height: 44px;
    position: relative;
    flex: 0 0 auto;
  }

  .step-icon {
    width: 44px;
    height: 44px;
    border-radius: 20px;
    display: grid;
    place-items: center;
  }

  .step-one {
    background: rgba(43, 127, 255, 0.1);
  }

  .step-two {
    background: rgba(0, 188, 125, 0.1);
  }

  .step-icon img {
    width: 20px;
    height: 20px;
    display: block;
  }

  .step-badge {
    position: absolute;
    top: -4px;
    right: -4px;
    width: 16px;
    height: 16px;
    border-radius: 14px;
    display: grid;
    place-items: center;
    color: #fff;
    font-size: 9px;
    font-weight: 700;
    line-height: 1;
  }

  .step-one .step-badge { background: #2b7fff; }
  .step-two .step-badge { background: #00bc7d; }

  .step-arrow {
    color: rgba(255, 255, 255, 0.18);
    font-size: 16px;
    flex: 0 0 auto;
    display: grid;
    place-items: center;
  }

  .step-arrow img {
    width: 16px;
    height: 16px;
    display: block;
  }

  .step-body {
    flex: 1;
    min-width: 0;
  }

  .step-title-row {
    display: flex;
    align-items: baseline;
    gap: 8px;
    flex-wrap: wrap;
  }

  .step-title-row h2 {
    margin: 0;
    font-size: 14px;
    line-height: 21px;
    font-weight: 500;
  }

  .step-title-row span {
    color: rgba(255, 255, 255, 0.3);
    font-size: 11px;
    line-height: 16.5px;
  }

  .step-body p {
    margin: 3px 0 0;
    color: rgba(255, 255, 255, 0.4);
    font-size: 12px;
    line-height: 18px;
  }

  .footnote {
    margin: 28px 0 0;
    color: rgba(255, 255, 255, 0.2);
    text-align: center;
    font-size: 11px;
    line-height: 16.5px;
  }

  @media (max-width: 520px) {
    .hero,
    .toggle-strip,
    .help-panel,
    .steps-shell,
    .appbar-inner {
      padding-inline: 14px;
    }

    .logo-stage {
      width: 100%;
      height: 280px;
      margin-bottom: -110px;
    }

    .logo-glow {
      left: 50%;
      transform: translateX(-50%);
      width: 240px;
      height: 240px;
    }

    .mark-shell {
      left: 50%;
      transform: translateX(-50%);
      top: 48px;
    }

    .signal {
      left: calc(50% + 28px);
      top: 108px;
    }

    h1 {
      font-size: 40px;
      line-height: 42px;
    }

    .store-links {
      flex-direction: column;
      align-items: center;
    }

    .chip-encrypted,
    .chip-fast,
    .app-store,
    .play-store {
      width: auto;
    }

    .toggle-strip {
      padding: 0 14px;
    }
  }
</style>
