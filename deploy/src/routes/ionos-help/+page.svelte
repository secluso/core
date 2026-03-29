<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";

  let backTarget = "/server-ssh";

  onMount(() => {
    const saved = sessionStorage.getItem("secluso-help-ref");
    if (saved) backTarget = saved;
  });
</script>

<main class="wrap">
  <header class="topbar">
    <button class="back" type="button" on:click={() => goto(backTarget)}>← Back</button>
    <h1>Ionos VPS Setup</h1>
    <div class="spacer"></div>
  </header>

  <section class="card">
    <h2>What you need</h2>
    <ul class="list">
      <li>An Ionos account with a VPS plan (1 vCPU, 1 GB RAM is enough to start).</li>
      <li>A Linux image such as Debian 12 or Ubuntu 22.04.</li>
      <li>Root password or an SSH key (both work with Secluso).</li>
    </ul>
    <p class="muted">Screens and labels change over time, so use this as a checklist if the UI looks different.</p>
  </section>

  <section class="card">
    <h2>Step-by-step</h2>
    <ol class="steps">
      <li>Pick a VPS plan (VPS XS is enough to start) and add it to your cart.</li>
      <li>Configure the term and choose your server location.</li>
      <li>Pick an operating system like Ubuntu or Debian.</li>
      <li>Complete checkout and place the order.</li>
      <li>After the order is complete, log in to the Ionos control panel.</li>
      <li>Open Servers &amp; Cloud and select your VPS.</li>
      <li>Copy the IP address and root password from the VPS details.</li>
    </ol>
  </section>

  <section class="card">
    <h2>Copy these into Secluso</h2>
    <ul class="list">
      <li><strong>Host / IP</strong>: the IPv4 address shown in your VPS details.</li>
      <li><strong>User</strong>: usually <code>root</code> unless you created a different user.</li>
      <li><strong>Password or key</strong>: the root password shown in Login Data or your SSH private key.</li>
    </ul>
  </section>

  <section class="card">
    <h2>Visual guide</h2>
    <div class="gallery">
      <figure class="shot">
        <img src="/help/ionos/ionos-step-1-plan.png" alt="Ionos VPS hosting plan comparison with VPS XS highlighted" />
        <figcaption>Start on the VPS plan page and choose a basic plan.</figcaption>
      </figure>
      <figure class="shot">
        <img src="/help/ionos/ionos-step-2-configure.png" alt="Configure VPS Linux XS with term and server location" />
        <figcaption>Set the term and server location.</figcaption>
      </figure>
      <figure class="shot">
        <img src="/help/ionos/ionos-step-3-os.png" alt="Operating system selection with Ubuntu and Debian options" />
        <figcaption>Select Ubuntu or Debian in the operating system section.</figcaption>
      </figure>
      <figure class="shot">
        <img src="/help/ionos/ionos-step-4-order-complete.png" alt="Order complete screen with thank you message" />
        <figcaption>Finish checkout and confirm your order.</figcaption>
      </figure>
      <figure class="shot">
        <img src="/help/ionos/ionos-step-5-login.png" alt="Ionos login screen with email field" />
        <figcaption>After the order completes, log in to your Ionos account.</figcaption>
      </figure>
      <figure class="shot">
        <img src="/help/ionos/ionos-step-6-dashboard.png" alt="Ionos product dashboard with Servers and Cloud tile" />
        <figcaption>In the dashboard, open Servers &amp; Cloud.</figcaption>
      </figure>
      <figure class="shot">
        <img src="/help/ionos/ionos-step-7-vps-details.png" alt="VPS details page with login data and IP address" />
        <figcaption>Copy the IP address and root password from Login Data.</figcaption>
      </figure>
    </div>
  </section>

  <section class="card tip">
    <h2>Tip</h2>
    <p class="muted">If you use an SSH key, store the private key somewhere easy to find for the deploy tool.</p>
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
.muted { color: #666; margin: 0 0 8px 0; }
.list { margin: 6px 0 0; padding-left: 20px; color: #444; }
.list li { margin: 6px 0; }
.steps { margin: 6px 0 0; padding-left: 20px; color: #444; }
.steps li { margin: 6px 0; }

.gallery { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 12px; }
.shot { margin: 0; border: 1px solid #e7e7e7; border-radius: 12px; padding: 10px; background: #fafafa; }
.shot img { width: 100%; display: block; border-radius: 8px; }
.shot figcaption { color: #555; font-size: 0.92rem; margin-top: 8px; }

.tip { border-style: dashed; }

@media (prefers-color-scheme: dark) {
  .card { background: #121212; border-color: #2a2a2a; box-shadow: 0 6px 22px rgba(0,0,0,0.4); }
  .muted { color: #d3d3d3; }
  .list, .steps { color: #e5e5e5; }
  .shot { background: #161616; border-color: #2a2a2a; }
  .shot figcaption { color: #cbd5f5; }
  .back { background: #1a1a1a; color: #f1f1f1; border-color: #2a2a2a; }
}
</style>
