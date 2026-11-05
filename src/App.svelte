<script lang="ts">
  import { onMount } from "svelte";
  import { app, type Route } from "./lib/state.svelte";
  import Home from "./routes/Home.svelte";
  import History from "./routes/History.svelte";
  import Dictionary from "./routes/Dictionary.svelte";
  import Snippets from "./routes/Snippets.svelte";
  import Styles from "./routes/Styles.svelte";
  import Settings from "./routes/Settings.svelte";
  import Onboarding from "./routes/Onboarding.svelte";

  const nav: { id: Route; label: string; icon: string }[] = [
    { id: "home", label: "Home", icon: "M3 10.5 12 3l9 7.5V20a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z" },
    { id: "history", label: "History", icon: "M12 7v5l3 2M21 12a9 9 0 1 1-3-6.7M21 4v4h-4" },
    { id: "dictionary", label: "Dictionary", icon: "M4 19.5V5a2 2 0 0 1 2-2h13v16H6a2 2 0 0 0-2 2zm0 0A2 2 0 0 0 6 21h13" },
    { id: "snippets", label: "Snippets", icon: "M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01" },
    { id: "styles", label: "Styles", icon: "M4 20h4L19 9l-4-4L4 16zM13.5 6.5l4 4" },
    { id: "settings", label: "Settings", icon: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" },
  ];

  onMount(() => {
    app.init();
  });

  const showOnboarding = $derived(app.settings !== null && !app.settings.onboarding_done);
</script>

<div class="shell" data-platform={app.platform}>
  <aside>
    <div class="drag" data-tauri-drag-region></div>
    <div class="brand">
      <span class="logo" aria-hidden="true"></span>
      <span>Hams</span>
    </div>
    <nav>
      {#each nav as item (item.id)}
        <button class:active={app.route === item.id} onclick={() => (app.route = item.id)} aria-current={app.route === item.id ? "page" : undefined}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d={item.icon} /></svg>
          {item.label}
        </button>
      {/each}
    </nav>
    <div class="status-dot" title="Dictation status">
      <span class="dot {app.status.phase}"></span>
      <span class="small muted">{app.status.phase === "idle" ? "Ready" : app.status.phase}</span>
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
    <div class="toast {app.toast.kind}" role="status">{app.toast.text}</div>
  {/if}
</div>

<style>
  .shell { display: grid; grid-template-columns: 196px 1fr; height: 100vh; }
  aside {
    background: var(--surface-2);
    border-right: 1px solid var(--border);
    display: flex; flex-direction: column;
    padding: 12px 10px;
    position: relative;
  }
  .drag { height: 12px; }
  [data-platform="macos"] aside .drag { height: 30px; }
  main .drag.top { position: absolute; inset: 0 0 auto 196px; height: 28px; }
  .brand { display: flex; align-items: center; gap: 8px; font-weight: 700; font-size: 16px; padding: 4px 8px 16px; }
  .logo {
    width: 20px; height: 20px; border-radius: 6px; background: var(--accent);
    mask: radial-gradient(circle at 50% 50%, #000 34%, transparent 36%), linear-gradient(#000 0 0);
    mask-composite: exclude;
  }
  nav { display: flex; flex-direction: column; gap: 2px; }
  nav button {
    display: flex; align-items: center; gap: 10px;
    border: none; background: transparent; text-align: left;
    padding: 7px 10px; color: var(--muted); font-weight: 500;
  }
  nav button:hover { background: color-mix(in srgb, var(--text) 6%, transparent); color: var(--text); }
  nav button.active { background: var(--surface); color: var(--text); box-shadow: var(--shadow); }
  nav svg { width: 17px; height: 17px; fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; }
  .status-dot { margin-top: auto; display: flex; align-items: center; gap: 8px; padding: 8px 10px; text-transform: capitalize; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--accent); }
  .dot.recording { background: #e5484d; animation: pulse 1.2s infinite; }
  .dot.transcribing, .dot.polishing, .dot.delivering { background: var(--warn); }
  .dot.error { background: var(--danger); }
  @keyframes pulse { 50% { opacity: 0.35; } }
  main { overflow-y: auto; padding: 28px 32px 40px; position: relative; }
  .toast {
    position: fixed; bottom: 20px; left: 50%; transform: translateX(-50%);
    background: var(--text); color: var(--bg);
    padding: 8px 16px; border-radius: 999px; font-size: 13px; max-width: 70vw;
    box-shadow: 0 4px 16px rgb(0 0 0 / 0.2);
  }
  .toast.error { background: var(--danger); color: #fff; }
</style>
