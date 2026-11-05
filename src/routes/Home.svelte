<script lang="ts">
  import { api, type HistoryItem, type ModelStatus, type Permissions, type Stats } from "../lib/ipc";
  import { app } from "../lib/state.svelte";
  import { compactNumber, relativeTime, shortcutLabel } from "../lib/format";

  let stats = $state<Stats | null>(null);
  let recent = $state<HistoryItem[]>([]);
  let permissions = $state<Permissions | null>(null);
  let models = $state<ModelStatus[]>([]);

  $effect(() => {
    void app.dataVersion;
    api.stats().then((s) => (stats = s)).catch(() => {});
    api.history("", 6).then((h) => (recent = h)).catch(() => {});
  });

  $effect(() => {
    api.permissions().then((p) => (permissions = p)).catch(() => {});
    api.models().then((m) => (models = m)).catch(() => {});
  });

  const settings = $derived(app.settings);
  const modelReady = $derived(models.length === 0 || models.some((m) => m.id === settings?.model && m.installed));
  const busy = $derived(["transcribing", "polishing", "delivering"].includes(app.status.phase));
  const recording = $derived(app.status.phase === "recording");
  const key = (s: string | undefined) => (s ? shortcutLabel(s, app.platform) : "");

  async function recheckPermissions() {
    await api.requestAccessibility().catch(() => {});
    setTimeout(async () => (permissions = await api.permissions()), 1500);
  }
</script>

<div class="stack page">
  <header>
    <h1>Good to see you</h1>
    <p class="muted">Speak anywhere. Hams writes it down where your cursor is.</p>
  </header>

  {#if app.view?.hotkey_error}
    <div class="banner error">
      <span class="grow">{app.view.hotkey_error}</span>
      <button onclick={() => api.openPermissionSettings("accessibility")}>Open Settings</button>
    </div>
  {:else if permissions?.applicable && !permissions.accessibility}
    <div class="banner warn">
      <span class="grow">Hams needs <strong>Accessibility</strong> access to use the shortcut and paste for you.</span>
      <button onclick={recheckPermissions}>Grant access</button>
    </div>
  {/if}
  {#if !modelReady}
    <div class="banner warn">
      <span class="grow">The speech model isn't downloaded yet.</span>
      <button onclick={() => (app.route = "settings")}>Download</button>
    </div>
  {/if}

  <section class="card hero">
    <button
      class="mic"
      class:recording
      class:busy
      onclick={() => (recording ? api.stop() : api.toggle())}
      disabled={busy}
      aria-label={recording ? "Stop dictation" : "Start dictation"}
    >
      <svg viewBox="0 0 24 24" aria-hidden="true">
        {#if recording}
          <rect x="7" y="7" width="10" height="10" rx="2" />
        {:else}
          <path d="M12 3a3 3 0 0 0-3 3v6a3 3 0 0 0 6 0V6a3 3 0 0 0-3-3zM5 11a7 7 0 0 0 14 0M12 18v3" />
        {/if}
      </svg>
    </button>
    <div class="grow stack" style="gap: 6px">
      {#if settings}
        <p class="lead">Hold <kbd>{key(settings.dictate_shortcut)}</kbd> and speak. Release to paste.</p>
        <p class="muted small">
          {#if settings.double_tap_lock}Double-tap it to keep going hands-free. {/if}Press <kbd>Esc</kbd> to cancel.
          Select text and hold <kbd>{key(settings.command_shortcut)}</kbd> to edit it by voice.
        </p>
      {/if}
      {#if app.status.phase === "error" && app.status.message}
        <p class="error-text small">{app.status.message}</p>
      {:else if app.status.text}
        <p class="last small">“{app.status.text}”</p>
      {/if}
    </div>
  </section>

  {#if stats}
    <section class="tiles">
      <div class="card tile"><span class="value">{compactNumber(stats.words_today)}</span><span class="muted small">words today</span></div>
      <div class="card tile"><span class="value">{compactNumber(stats.total_words)}</span><span class="muted small">words total</span></div>
      <div class="card tile"><span class="value">{stats.average_wpm || "–"}</span><span class="muted small">words per minute</span></div>
      <div class="card tile"><span class="value">{stats.minutes_saved}</span><span class="muted small">minutes saved</span></div>
      <div class="card tile"><span class="value">{stats.current_streak}</span><span class="muted small">day streak</span></div>
    </section>
  {/if}

  <section class="stack">
    <div class="spread">
      <h2>Recent</h2>
      {#if recent.length}<button class="ghost small" onclick={() => (app.route = "history")}>See all</button>{/if}
    </div>
    {#if recent.length === 0}
      <div class="card empty">Your dictations will show up here.</div>
    {:else}
      <ul class="recent">
        {#each recent as item (item.id)}
          <li class="card">
            <p class="grow text">{item.text}</p>
            <div class="row meta">
              <span class="small muted">{relativeTime(item.created_at)}{item.app ? ` · ${item.app}` : ""}</span>
              <button class="ghost small" onclick={() => app.run(() => api.copy(item.text), "Copied")}>Copy</button>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</div>

<style>
  .page { max-width: 860px; gap: 20px; }
  .hero { display: flex; gap: 20px; align-items: center; padding: 20px; }
  .lead { font-size: 15px; }
  .mic {
    width: 64px; height: 64px; border-radius: 50%; flex: none;
    display: grid; place-items: center;
    background: var(--accent); border: none; color: var(--accent-ink);
  }
  .mic:hover:not(:disabled) { background: var(--accent); filter: brightness(1.08); }
  .mic svg { width: 26px; height: 26px; fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; }
  .mic.recording { background: #e5484d; color: #fff; }
  .mic.recording svg { fill: currentColor; stroke: none; }
  .mic.busy { animation: breathe 1s ease-in-out infinite; }
  @keyframes breathe { 50% { opacity: 0.6; } }
  .last { color: var(--muted); font-style: italic; overflow: hidden; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; }
  .error-text { color: var(--danger); }
  .tiles { display: grid; grid-template-columns: repeat(auto-fit, minmax(130px, 1fr)); gap: 10px; }
  .tile { display: flex; flex-direction: column; padding: 12px 14px; }
  .value { font-size: 22px; font-weight: 650; font-variant-numeric: tabular-nums; }
  .recent { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 8px; }
  .recent li { padding: 10px 14px; display: flex; flex-direction: column; gap: 4px; }
  .text { white-space: pre-wrap; overflow: hidden; display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; }
  .meta { justify-content: space-between; }
</style>
