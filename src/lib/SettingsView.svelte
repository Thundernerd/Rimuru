<script lang="ts">
  import { api, errorMessage } from "./api";
  import { clearForumPages } from "./forum";
  import { refresh, runRefresh } from "./refresh.svelte";
  import { refreshSession, session } from "./session.svelte";
  import { settings, updateSettings } from "./settings.svelte";

  const INTERVALS = [5, 10, 15, 30, 60];
  const BROWSERS = ["Safari", "Google Chrome", "Firefox", "Brave Browser", "Microsoft Edge", "Arc", "Vivaldi", "Opera"];

  let browser = $state(settings.value.link_browser ?? "");
  let cacheMessage = $state<string | null>(null);
  let signOutArmed = $state(false);
  let accountMessage = $state<string | null>(null);

  const username = $derived(session.current?.state === "logged_in" ? session.current.username : null);
  const lastCheck = $derived(refresh.lastRun ? new Date(refresh.lastRun).toLocaleTimeString() : null);

  async function clearCache() {
    try {
      const removed = await api.clearCache();
      clearForumPages();
      cacheMessage = `Removed ${removed} cached ${removed === 1 ? "item" : "items"}.`;
    } catch (e) {
      cacheMessage = `Couldn't clear the cache: ${errorMessage(e)}`;
    }
  }

  async function signOut() {
    if (!signOutArmed) {
      signOutArmed = true;
      return;
    }
    signOutArmed = false;
    try {
      await api.signOut();
      clearForumPages();
      accountMessage = "Signed out.";
      await refreshSession();
    } catch (e) {
      accountMessage = `Couldn't sign out: ${errorMessage(e)}`;
    }
  }
</script>

<div class="settings">
  <h1>Settings</h1>

  <section class="card">
    <h2>Links</h2>
    <label class="row">
      <span class="label">
        Open links with
        <small>An app name, e.g. Firefox. Leave empty for your default browser.</small>
      </span>
      <input
        list="browsers"
        placeholder="Default browser"
        bind:value={browser}
        onchange={() => updateSettings({ link_browser: browser.trim() || null })}
      />
      <datalist id="browsers">
        {#each BROWSERS as name (name)}<option value={name}></option>{/each}
      </datalist>
    </label>
  </section>

  <section class="card">
    <h2>Updates</h2>
    <label class="row">
      <span class="label">
        Check the forum every
        <small>Refreshes new replies, followed games and their releases in the background.</small>
      </span>
      <select
        value={settings.value.refresh_minutes}
        onchange={(e) => updateSettings({ refresh_minutes: Number(e.currentTarget.value) })}
      >
        {#each INTERVALS as minutes (minutes)}<option value={minutes}>{minutes} minutes</option>{/each}
      </select>
    </label>
    <label class="row">
      <span class="label">
        Notify me about new releases
        <small>A system notification when a followed game gets a new release.</small>
      </span>
      <input
        type="checkbox"
        checked={settings.value.notifications}
        onchange={(e) => updateSettings({ notifications: e.currentTarget.checked })}
      />
    </label>
    <div class="row">
      <span class="label">
        Check now
        <small>{lastCheck ? `Last checked at ${lastCheck}.` : "Not checked yet this session."}</small>
      </span>
      <button type="button" onclick={() => runRefresh()} disabled={refresh.running}>
        {refresh.running ? "Checking…" : "Check now"}
      </button>
    </div>
  </section>

  <section class="card">
    <h2>Data</h2>
    <div class="row">
      <span class="label">
        Clear cache
        <small>Forum pages and Steam details are fetched again. Followed games, visits and releases you have are kept.</small>
      </span>
      <button type="button" onclick={clearCache}>Clear cache</button>
    </div>
    {#if cacheMessage}<p class="note">{cacheMessage}</p>{/if}
  </section>

  <section class="card">
    <h2>Account</h2>
    <div class="row">
      <span class="label">
        {username ? `Signed in as ${username}` : session.current?.state === "logged_out" ? "Not signed in" : "Checking…"}
        <small>Signing out clears the forum window's cookies and session.</small>
      </span>
      <button type="button" class:armed={signOutArmed} onclick={signOut} onblur={() => (signOutArmed = false)}>
        {signOutArmed ? "Click again to sign out" : "Sign out"}
      </button>
    </div>
    {#if accountMessage}<p class="note">{accountMessage}</p>{/if}
  </section>

  <section class="card">
    <h2>Developer</h2>
    <label class="row">
      <span class="label">
        Show developer tools
        <small>"Save HTML" buttons for saving forum pages as parser test fixtures.</small>
      </span>
      <input
        type="checkbox"
        checked={settings.value.dev_tools}
        onchange={(e) => updateSettings({ dev_tools: e.currentTarget.checked })}
      />
    </label>
  </section>
</div>

<style>
  .settings {
    display: grid;
    gap: 16px;
    max-width: 760px;
  }
  h1 {
    margin: 0 0 4px;
    font-size: 24px;
  }
  .card {
    padding: 6px 18px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
  }
  h2 {
    margin: 12px 0 4px;
    font-size: 12px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    padding: 12px 0;
    border-top: 1px solid var(--border);
  }
  h2 + .row {
    border-top: none;
  }
  .label {
    display: grid;
    gap: 3px;
  }
  small {
    font-size: 12px;
    color: var(--muted);
  }
  input:not([type="checkbox"]),
  select {
    width: 220px;
    font: inherit;
    font-size: 13px;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    color: var(--text);
  }
  input[type="checkbox"] {
    width: 18px;
    height: 18px;
    accent-color: var(--accent);
  }
  .note {
    margin: 0 0 12px;
    font-size: 13px;
    color: var(--muted);
  }
  button.armed {
    border-color: var(--warn);
    color: var(--warn);
  }
</style>
