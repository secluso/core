<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
  import { onMount } from "svelte";

  let checked = false;
  let isTauri = false;
  let browserPreview = false;

  onMount(() => {
    const w = window as any;
    isTauri = Boolean(w?.__TAURI__ || w?.__TAURI_INTERNALS__ || w?.__TAURI_IPC__);
    browserPreview = new URLSearchParams(window.location.search).get("browserPreview") === "1";
    checked = true;
  });
</script>

{#if checked && !isTauri && !browserPreview}
  <main class="blocked">
    <div class="panel">
      <h1>Desktop app only</h1>
      <p>This interface runs inside the Secluso desktop app.</p>
      <p>Launch it with the Tauri app instead of a browser.</p>
    </div>
  </main>
{:else}
  <slot />
{/if}

<style>
  .blocked {
    min-height: 100vh;
    display: grid;
    place-items: center;
    background: #0b0b0b;
    color: #f5f5f5;
    padding: 24px;
  }
  .panel {
    max-width: 460px;
    border: 1px solid #2a2a2a;
    border-radius: 14px;
    padding: 20px;
    background: #111;
    box-shadow: 0 16px 50px rgba(0, 0, 0, 0.35);
  }
  h1 {
    margin: 0 0 10px;
    font-size: 1.4rem;
  }
  p {
    margin: 0 0 8px;
    color: #d4d4d4;
  }
  p:last-child { margin-bottom: 0; }
</style>
