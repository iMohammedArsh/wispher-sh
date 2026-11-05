<script lang="ts">
  import { api, type StyleInfo } from "../lib/ipc";
  import { app } from "../lib/state.svelte";

  let styles = $state<StyleInfo[]>([]);
  $effect(() => {
    api.styles().then((s) => (styles = s)).catch(() => {});
  });

  const examples: Record<string, string> = {
    messaging: "Slack, WhatsApp, Messages, Discord, Teams",
    email: "Mail, Outlook, Spark, Superhuman",
    code: "VS Code, Cursor, Xcode, JetBrains, Terminal, iTerm, Warp",
    docs: "Notes, Notion, Obsidian, Pages, Word",
    default: "Everything else, including browsers",
  };
</script>

<div class="stack page">
  <div class="spread">
    <div>
      <h1>Styles</h1>
      <p class="muted">Hams adapts how it writes to the app you're typing in.</p>
    </div>
    {#if app.settings}
      <label class="row toggle">
        <input type="checkbox" checked={app.settings.app_styles} onchange={(e) => app.save({ app_styles: e.currentTarget.checked })} />
        App-aware styles
      </label>
    {/if}
  </div>

  {#if app.settings?.ai.provider === "off"}
    <div class="banner warn">
      <span class="grow">Styles shape the AI polish. Turn on an AI provider to get the full effect; without one Hams still tidies messaging punctuation.</span>
      <button onclick={() => (app.route = "settings")}>Set up AI</button>
    </div>
  {/if}

  <div class="grid" class:off={!app.settings?.app_styles}>
    {#each styles as s (s.id)}
      <article class="card stack">
        <div class="spread">
          <h2>{s.label}</h2>
          {#if !s.polish}<span class="pill">raw text</span>{/if}
        </div>
        <p class="small muted">{examples[s.id]}</p>
        <p class="small">{s.prompt}</p>
      </article>
    {/each}
  </div>
</div>

<style>
  .page { max-width: 900px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 10px; }
  .grid.off { opacity: 0.55; }
  article { gap: 6px; }
  .toggle { gap: 6px; font-weight: 500; }
</style>
