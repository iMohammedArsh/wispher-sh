<script lang="ts">
  import { onMount } from "svelte";
  import { app, type Route, type SettingsArea } from "./lib/state.svelte";
  import { duration, shortcutLabel } from "./lib/format";
  import { inTauri } from "./lib/ipc";
  import Icon, { type IconName } from "./components/Icon.svelte";
  import Mark from "./components/Mark.svelte";
  import Home from "./routes/Home.svelte";
  import History from "./routes/History.svelte";
  import Notes from "./routes/Notes.svelte";
  import Dictionary from "./routes/Dictionary.svelte";
  import Snippets from "./routes/Snippets.svelte";
  import Styles from "./routes/Styles.svelte";
  import Settings from "./routes/Settings.svelte";
  import Onboarding from "./routes/Onboarding.svelte";

  type Item = { id: Route; label: string; icon: IconName };
  const groups: { label: string | null; items: Item[] }[] = [
    {
      label: null,
      items: [
        { id: "home", label: "Home", icon: "home" },
        { id: "history", label: "History", icon: "history" },
        { id: "notes", label: "Notes", icon: "notes" },
      ],
    },
    {
      label: "Writing",
      items: [
        { id: "dictionary", label: "Dictionary", icon: "dictionary" },
        { id: "snippets", label: "Snippets", icon: "snippets" },
        { id: "styles", label: "Styles", icon: "styles" },
      ],
    },
  ];

  onMount(() => {
    app.init();
    // Browser preview (vite dev without Tauri): open a screen from the URL, e.g. main.html#notes.
    // A Settings view opens as main.html#settings/speech.
    const [route, area] = location.hash.slice(1).split("/") as [Route, SettingsArea | undefined];
    if (!inTauri && route) app.route = route;
    if (!inTauri && area) app.settingsArea = area;
  });

  // A ticking clock, only while recording, for the status line's timer.
  let now = $state(Date.now());
  $effect(() => {
    if (app.status.phase !== "recording") return;
    const timer = setInterval(() => (now = Date.now()), 500);
    return () => clearInterval(timer);
  });

  const showOnboarding = $derived(app.settings !== null && !app.settings.onboarding_done);
  const phase = $derived(app.status.phase);
  const statusText = $derived(
    phase === "recording"
      ? app.status.kind === "note" ? "Recording a note" : "Listening"
      : ["transcribing", "polishing", "delivering"].includes(phase)
        ? "Writing…"
        : phase === "error"
          ? "Something went wrong"
          : "Ready",
  );
</script>

<div class="shell" data-platform={app.platform}>
  <aside>
    <div class="drag" data-tauri-drag-region></div>
    <div class="brand" data-tauri-drag-region>
      <Mark size={18} />
      <span>Hams</span>
    </div>
    <nav aria-label="Main">
      {#each groups as group}
        {#if group.label}<p class="group">{group.label}</p>{/if}
        {#each group.items as item (item.id)}
          <button class:active={app.route === item.id} onclick={() => (app.route = item.id)} aria-current={app.route === item.id ? "page" : undefined}>
            <Icon name={item.icon} />
            {item.label}
            {#if item.id === "notes" && phase === "recording" && app.status.kind === "note"}<span class="live-dot" title="Recording"></span>{/if}
            {#if item.id === "dictionary" && app.pendingSuggestions}<span class="count num" title="{app.pendingSuggestions} suggestions to review">{app.pendingSuggestions}</span>{/if}
          </button>
        {/each}
      {/each}
    </nav>

    <div class="foot">
      {#if app.update}
        <div class="update" role="status">
          <span><strong>Update ready</strong><span class="small muted">Hams {app.update.version}</span></span>
          {#if app.updateProgress !== undefined}
            <progress max="1" value={app.updateProgress ?? undefined} aria-label="Installing update"></progress>
          {:else}
            <button class="small primary" onclick={() => app.installUpdate()}>Install</button>
          {/if}
        </div>
      {/if}
      <button class:active={app.route === "settings"} onclick={() => (app.route = "settings")} aria-current={app.route === "settings" ? "page" : undefined}>
        <Icon name="settings" />
        Settings
      </button>
      <div class="state" title="Dictation status" aria-live="polite">
        <span class="dot {phase}"></span>
        <span class="grow">{statusText}</span>
        {#if phase === "recording" && app.status.started_at}<span class="num">{duration(Math.max(0, now - app.status.started_at))}</span>{/if}
        {#if phase === "idle" && app.settings}<kbd>{shortcutLabel(app.settings.dictate_shortcut, app.platform)}</kbd>{/if}
      </div>
    </div>
  </aside>

  <main>
    <div class="drag top" data-tauri-drag-region></div>
    {#if showOnboarding}
      <Onboarding />
    {:else if app.route === "home"}
      <Home />
    {:else if app.route === "history"}
      <History />
    {:else if app.route === "notes"}
      <Notes />
    {:else if app.route === "dictionary"}
      <Dictionary />
    {:else if app.route === "snippets"}
      <Snippets />
    {:else if app.route === "styles"}
      <Styles />
    {:else}
      <Settings />
    {/if}
  </main>

  {#if app.toast}
    <div class="toast-lane"><div class="toast {app.toast.kind}" role="status">{app.toast.text}</div></div>
  {/if}
</div>

<style>
  .shell { --sidebar: 212px; display: grid; grid-template-columns: var(--sidebar) 1fr; height: 100vh; }
  aside { display: flex; flex-direction: column; padding: 0 10px 12px; min-height: 0; }
  .drag { height: 14px; flex: none; }
  [data-platform="macos"] aside .drag { height: 38px; }
  main .drag.top { position: sticky; top: 0; height: 0; }
  .brand { display: flex; align-items: center; gap: 8px; font-weight: 600; font-size: 16px; letter-spacing: -0.01em; padding: 2px 10px 18px; }
  nav { display: flex; flex-direction: column; gap: 2px; }
  .group { font-size: 12px; color: var(--muted); padding: 18px 10px 6px; }
  nav button, .foot > button {
    height: 32px; justify-content: flex-start; gap: 10px;
    border: none; background: transparent; padding: 0 10px;
    font-size: 14px; font-weight: 400; color: var(--text);
  }
  nav button :global(.icon), .foot > button :global(.icon) { color: var(--muted); }
  nav button:hover:not(.active), .foot > button:hover:not(.active) { background: color-mix(in srgb, var(--active) 60%, transparent); }
  nav button.active, .foot > button.active { background: var(--active); font-weight: 500; }
  nav button.active :global(.icon), .foot > button.active :global(.icon) { color: var(--text); }
  nav .live-dot { margin-left: auto; }
  nav .count { margin-left: auto; font-size: 12px; color: var(--muted); min-width: 20px; height: 18px; padding: 0 6px; border-radius: 999px; background: var(--pill-neutral-bg); display: inline-flex; align-items: center; justify-content: center; }
  .foot { margin-top: auto; display: flex; flex-direction: column; gap: 6px; }
  .update { display: flex; align-items: center; justify-content: space-between; gap: 8px; margin: 0 2px 4px; padding: 10px 10px 10px 12px; border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--surface); font-size: 13px; }
  .update strong { display: block; font-weight: 600; }
  .update .small { display: block; }
  .update progress { width: 56px; }
  .state { display: flex; align-items: center; gap: 8px; padding: 10px 10px 2px; font-size: 12px; color: var(--muted); border-top: 1px solid var(--border); }
  .state kbd { height: 20px; font-size: 12px; }
  .dot { width: 7px; height: 7px; border-radius: 50%; background: var(--data); flex: none; }
  .dot.recording { background: var(--live); animation: pulse 1.2s ease-in-out infinite; }
  .dot.transcribing, .dot.polishing, .dot.delivering { background: var(--faint); animation: pulse 1s ease-in-out infinite; }
  .dot.error { background: var(--negative); }
  main {
    overflow-y: auto; padding: 34px 36px 48px;
    background: var(--surface);
    border-left: 1px solid var(--border);
  }
  .toast-lane { position: fixed; bottom: 20px; left: var(--sidebar); right: 0; display: flex; justify-content: center; pointer-events: none; }
  .toast {
    pointer-events: auto;
    background: var(--primary); color: var(--primary-ink);
    padding: 9px 16px; border-radius: 8px; font-size: 13px; max-width: 60vw;
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.18);
    animation: rise 160ms ease-out;
  }
  .toast.error { background: var(--negative); color: #fff; }
  @keyframes rise { from { transform: translateY(6px); opacity: 0; } }
</style>
