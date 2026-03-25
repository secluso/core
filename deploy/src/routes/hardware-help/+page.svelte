<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import { openExternalUrl } from "$lib/api";

  let backTarget = "/image";

  onMount(() => {
    const saved = sessionStorage.getItem("secluso-help-ref");
    if (saved) backTarget = saved;
  });

  async function openExternal(url: string) {
    try {
      await openExternalUrl(url);
    } catch {
      window.open(url, "_blank", "noopener,noreferrer");
    }
  }
</script>

<main class="wrap">
  <header class="topbar">
    <button class="back" type="button" on:click={() => goto(backTarget)}>← Back</button>
    <h1>Recommended Hardware</h1>
    <div class="spacer"></div>
  </header>

  <section class="card">
    <h2>What works</h2>
    <p class="muted">Secluso runs on any Raspberry Pi (Zero 2W, 3B, 4, 5 all work). We recommend the Zero 2W because it is small, efficient, and can handle one camera with motion and object detection.</p>
  </section>

  <section class="card">
    <h2>Option A — Standard Pi Camera + Light Sensor (paired)</h2>
    <ul class="list">
      <li><a href="https://www.amazon.com/Raspberry-Zero-Bluetooth-RPi-2W/dp/B09LH5SBPS" on:click|preventDefault={() => openExternal("https://www.amazon.com/Raspberry-Zero-Bluetooth-RPi-2W/dp/B09LH5SBPS")}>Raspberry Pi Zero 2W</a></li>
      <li><a href="https://www.amazon.com/Raspberry-Pi-Camera-Module-Megapixel/dp/B01ER2SKFS" on:click|preventDefault={() => openExternal("https://www.amazon.com/Raspberry-Pi-Camera-Module-Megapixel/dp/B01ER2SKFS")}>Raspberry Pi Camera Module v2</a></li>
      <li><a href="https://www.amazon.com/Raspberry-Official-Enclosure-Camera-Compatible/dp/B07QJZBXYC" on:click|preventDefault={() => openExternal("https://www.amazon.com/Raspberry-Official-Enclosure-Camera-Compatible/dp/B07QJZBXYC")}>Raspberry Pi Official Enclosure (with camera hole)</a></li>
      <li>Optional light sensor (motion-activated, paired with the Pi camera to improve night detection).
        Example: <a href="https://www.amazon.com/EverBrite-Battery-Bathroom-Batteries-Included/dp/B08Y8QCZ6M" on:click|preventDefault={() => openExternal("https://www.amazon.com/EverBrite-Battery-Bathroom-Batteries-Included/dp/B08Y8QCZ6M")}>EverBrite sensor</a></li>
    </ul>
    <p class="muted">Use this setup if you want the official Pi camera and can add a simple light sensor for low-light conditions.</p>
  </section>

  <section class="card">
    <h2>Option B — Integrated Adjustable-Focus Camera (alternative)</h2>
    <ul class="list">
      <li><a href="https://www.amazon.com/Raspberry-Zero-Bluetooth-RPi-2W/dp/B09LH5SBPS" on:click|preventDefault={() => openExternal("https://www.amazon.com/Raspberry-Zero-Bluetooth-RPi-2W/dp/B09LH5SBPS")}>Raspberry Pi Zero 2W</a></li>
    </ul>

    <details class="details">
      <summary><strong>Case and camera pairings</strong></summary>
      <div class="details-body">
        <h3>Option (easy)</h3>
        <p class="muted">Use the included acrylic mount for the camera and a separate Pi case.</p>
        <ul class="list">
          <li><a href="https://www.amazon.com/MakerFocus-Raspberry-Camera-Adjustable-Focus-Fisheye/dp/B07BK1QZ2L" on:click|preventDefault={() => openExternal("https://www.amazon.com/MakerFocus-Raspberry-Camera-Adjustable-Focus-Fisheye/dp/B07BK1QZ2L")}>Night vision adjustable focus camera + acrylic mount</a></li>
          <li><a href="https://www.amazon.com/iUniker-Raspberry-Starter-Acrylic-Clear/dp/B075FLGWJL/" on:click|preventDefault={() => openExternal("https://www.amazon.com/iUniker-Raspberry-Starter-Acrylic-Clear/dp/B075FLGWJL/")}>iUniker Raspberry Pi acrylic case</a></li>
        </ul>

        <h3>Option (advanced)</h3>
        <p class="muted">Use a 3D-printed case that holds both the Pi and the night-vision camera.</p>
        <ul class="list">
          <li><a href="https://gmail784811.autodesk360.com/g/shares/SH56a43QTfd62c1cd9681e75e6af7b61dce3" on:click|preventDefault={() => openExternal("https://gmail784811.autodesk360.com/g/shares/SH56a43QTfd62c1cd9681e75e6af7b61dce3")}>3D printed case design</a></li>
          <li><a href="https://www.amazon.com/MELIFE-Raspberry-Camera-Adjustable-Focus-Infrared/dp/B08RHZ5BJM/" on:click|preventDefault={() => openExternal("https://www.amazon.com/MELIFE-Raspberry-Camera-Adjustable-Focus-Infrared/dp/B08RHZ5BJM/")}>Night vision adjustable focus camera without mount</a></li>
        </ul>
      </div>
    </details>

    <p class="muted">If you have another night vision camera or prefer a different option, it likely will work. These are just recommendations.</p>
  </section>

  <section class="card tip">
    <h2>Tip</h2>
    <p class="muted">Stick with Option A if you want the simplest build. Pick Option B when you need adjustable focus or a tighter integrated enclosure.</p>
  </section>
</main>

<style>
.wrap { max-width: 980px; margin: 0 auto; padding: 44px 20px 60px; }
.topbar { display: grid; grid-template-columns: 120px 1fr 120px; align-items: center; gap: 12px; margin: 8px 0 18px; }
.topbar h1 { text-align: center; margin: 0; font-size: 1.6rem; }
.spacer { width: 100%; }
.back { justify-self: start; border: 1px solid #d7d7d7; background: #fff; color: #111; padding: 10px 14px; border-radius: 10px; cursor: pointer; }
.back:hover { border-color: #c6c6c6; }

.card { background: #fff; border: 1px solid #e7e7e7; border-radius: 14px; padding: 16px; margin-bottom: 14px; box-shadow: 0 6px 22px rgba(0,0,0,0.06); }
.card h2 { margin: 0 0 10px 0; font-size: 1.15rem; }
.card h3 { margin: 16px 0 6px; font-size: 1rem; }
.muted { color: #666; margin: 0 0 8px 0; }
.list { margin: 6px 0 0; padding-left: 20px; color: #444; }
.list li { margin: 6px 0; }
.list a { color: #396cd8; text-decoration: none; }
.list a:hover { text-decoration: underline; }

.details { margin-top: 10px; border: 1px dashed #dadada; border-radius: 12px; padding: 10px 12px; background: #fafafa; }
.details summary { cursor: pointer; font-size: 0.98rem; }
.details-body { margin-top: 10px; }

.tip { border-style: dashed; }

@media (prefers-color-scheme: dark) {
  .card { background: #121212; border-color: #2a2a2a; box-shadow: 0 6px 22px rgba(0,0,0,0.4); }
  .muted { color: #d3d3d3; }
  .list { color: #e5e5e5; }
  .details { background: #161616; border-color: #2a2a2a; }
  .list a { color: #7aa7ff; }
  .back { background: #1a1a1a; color: #f1f1f1; border-color: #2a2a2a; }
}
</style>
