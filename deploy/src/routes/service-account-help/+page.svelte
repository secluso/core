<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
  import { goto } from "$app/navigation";
</script>

<main class="wrap">
  <header class="topbar">
    <button class="back" type="button" on:click={() => goto("/server-ssh")}>← Back</button>
    <h1>Service account key</h1>
    <div class="spacer"></div>
  </header>

  <section class="card">
    <h2>Where to get this</h2>
    <p class="muted">You will download a JSON file from Firebase and select it in the app.</p>
    <ol class="steps">
      <li>Go to https://console.firebase.google.com and sign in with a Google account.</li>
      <li>Click Create a new Firebase project, then name it Secluso.</li>
      <li>Click Add app and choose Android or iOS.</li>
      <li>Use the package name com.secluso.mobile, then click Register app.</li>
      <li>Open Project settings, then Service accounts, then Generate new private key.</li>
      <li>Download the JSON file and keep it private. This is the service account key.</li>
    </ol>
    <p class="muted">Use the JSON file name it downloads, for example secluso-44059-firebase-adminsdk-fbsvc-42d74661ff.json. No renaming needed.</p>
  </section>

  <section class="card">
    <h2>Visual guide</h2>
    <div class="gallery">
      <figure class="shot">
        <img src="/help/firebase/firebase-landing.png" alt="Firebase home screen with Create a new Firebase project" />
        <figcaption>Click Create a new Firebase project on the home screen.</figcaption>
      </figure>
      <figure class="shot">
        <img src="/help/firebase/firebase-project-name.png" alt="Create a project screen with project name" />
        <figcaption>Name your project and continue.</figcaption>
      </figure>
      <figure class="shot">
        <img src="/help/firebase/firebase-project-dashboard-add-app.png" alt="Project dashboard with Add app" />
        <figcaption>On the project dashboard, click Add app.</figcaption>
      </figure>
      <figure class="shot">
        <img src="/help/firebase/firebase-register-app.png" alt="Register app screen with package name" />
        <figcaption>Enter com.secluso.mobile and click Register app.</figcaption>
      </figure>
      <figure class="shot">
        <img src="/help/firebase/firebase-app-card.png" alt="App card with com.secluso.mobile" />
        <figcaption>Your app appears here. Click the gear icon to open Project settings.</figcaption>
      </figure>
      <figure class="shot">
        <img src="/help/firebase/firebase-service-account-key.png" alt="Service accounts tab with Generate new private key" />
        <figcaption>Open the Service accounts tab, then generate the private key and download the JSON file.</figcaption>
      </figure>
    </div>
  </section>

  <section class="card tip">
    <h2>Tip</h2>
    <p class="muted">If this is your first time, create a fresh Google account just for Secluso.</p>
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
  .steps { color: #e5e5e5; }
  .shot { background: #161616; border-color: #2a2a2a; }
  .shot figcaption { color: #cbd5f5; }
  .back { background: #1a1a1a; color: #f1f1f1; border-color: #2a2a2a; }
}
</style>
