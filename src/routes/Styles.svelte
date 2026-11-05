<script lang="ts">
  import { api, type AppRule, type StyleId, type StyleInfo, type StyleOverride } from "../lib/ipc";
  import { app } from "../lib/state.svelte";

  const MAX_PROMPT = 1500;

  let styles = $state<StyleInfo[]>([]);
  let recentApps = $state<string[]>([]);
  let drafts = $state<Partial<Record<StyleId, string>>>({});
  let newRule = $state<AppRule>({ app: "", style: "messaging" });

  $effect(() => {
    api.styles().then((s) => (styles = s)).catch(() => {});
    void app.dataVersion;
    api.recentApps().then((a) => (recentApps = a)).catch(() => {});
  });

  const s = $derived(app.settings);
  const overrides = $derived(s?.style_overrides ?? {});
  const rules = $derived(s?.app_rules ?? []);
  const unruledApps = $derived(recentApps.filter((a) => !rules.some((r) => r.app.toLowerCase() === a.toLowerCase())));

  const examples: Record<StyleId, string> = {
    messaging: "Slack, WhatsApp, Messages, Discord, Teams",
    email: "Mail, Outlook, Spark, Superhuman",
    code: "VS Code, Cursor, Xcode, JetBrains, Terminal, iTerm, Warp",
    docs: "Notes, Notion, Obsidian, Pages, Word",
    default: "Everything else, including browsers",
  };

  function promptOf(style: StyleInfo): string {
    return drafts[style.id] ?? overrides[style.id]?.prompt ?? style.prompt;
  }

  function polishOf(style: StyleInfo): boolean {
    return overrides[style.id]?.polish ?? style.polish;
  }

  function isEdited(style: StyleInfo): boolean {
    const o = overrides[style.id];
    return (o?.prompt != null && o.prompt.trim() !== style.prompt) || (o?.polish != null && o.polish !== style.polish);
  }

  /** Store only what differs from the built-in style, so future default improvements still apply. */
  async function setOverride(style: StyleInfo, patch: Partial<StyleOverride>) {
    const current: StyleOverride = { prompt: null, polish: null, ...overrides[style.id], ...patch };
    if (current.prompt != null && current.prompt.trim() === style.prompt) current.prompt = null;
    if (current.polish === style.polish) current.polish = null;
    const next = { ...overrides };
    if (current.prompt == null && current.polish == null) delete next[style.id];
    else next[style.id] = current;
    return app.save({ style_overrides: next });
  }

  async function commitPrompt(style: StyleInfo) {
    const draft = drafts[style.id];
    if (draft === undefined) return;
    const text = draft.trim();
    if (await setOverride(style, { prompt: text === "" ? null : text })) delete drafts[style.id];
  }

  async function reset(style: StyleInfo) {
    delete drafts[style.id];
    const next = { ...overrides };
    delete next[style.id];
    await app.save({ style_overrides: next });
  }

  async function addRule(name = newRule.app) {
    const appName = name.trim();
    if (!appName) return;
    const others = rules.filter((r) => r.app.toLowerCase() !== appName.toLowerCase());
    if (await app.save({ app_rules: [...others, { app: appName, style: newRule.style }] })) newRule = { ...newRule, app: "" };
  }

  function setRuleStyle(index: number, style: StyleId) {
    app.save({ app_rules: rules.map((r, i) => (i === index ? { ...r, style } : r)) });
  }

  function removeRule(index: number) {
    app.save({ app_rules: rules.filter((_, i) => i !== index) });
  }
</script>

<div class="stack page">
  <div class="spread">
    <div>
      <h1>Styles</h1>
      <p class="muted">Hams adapts how it writes to the app you're typing in. Edit any style to make it yours.</p>
    </div>
    {#if s}
      <label class="row toggle">
        <input type="checkbox" checked={s.app_styles} onchange={(e) => app.save({ app_styles: e.currentTarget.checked })} />
        App-aware styles
      </label>
    {/if}
  </div>

  {#if s?.ai.provider === "off"}
    <div class="banner warn">
      <span class="grow">Style prompts shape the AI polish. Turn on an AI provider to use them; without one Hams still tidies messaging punctuation.</span>
      <button onclick={() => (app.route = "settings")}>Set up AI</button>
    </div>
  {/if}

  <div class="grid" class:off={!s?.app_styles}>
    {#each styles as style (style.id)}
      <article class="card stack">
        <div class="spread">
          <h2>{style.label}</h2>
          {#if isEdited(style)}
            <button class="ghost small" onclick={() => reset(style)}>Reset</button>
          {/if}
        </div>
        <p class="small muted">{examples[style.id]}</p>
        <textarea
          rows="4"
          maxlength={MAX_PROMPT}
          value={promptOf(style)}
          placeholder={style.prompt}
          aria-label="{style.label} style instructions"
          oninput={(e) => (drafts[style.id] = e.currentTarget.value)}
          onblur={() => commitPrompt(style)}
        ></textarea>
        <label class="check small">
          <input type="checkbox" checked={polishOf(style)} onchange={(e) => setOverride(style, { polish: e.currentTarget.checked })} />
          Polish with AI {#if !style.polish}<span class="muted">(off by default: AI rewrites can break commands)</span>{/if}
        </label>
      </article>
    {/each}
  </div>

  <section class="card stack" class:off={!s?.app_styles}>
    <div>
      <h2>App rules</h2>
      <p class="small muted">Pin an app to a style. Rules win over automatic detection. Use the app's name as shown in History, or its bundle id (for example <code>com.google.Chrome</code>).</p>
    </div>

    {#each rules as rule, index (rule.app)}
      <div class="rule">
        <span class="grow">{rule.app}</span>
        <select value={rule.style} onchange={(e) => setRuleStyle(index, e.currentTarget.value as StyleId)} aria-label="Style for {rule.app}">
          {#each styles as style (style.id)}<option value={style.id}>{style.label}</option>{/each}
        </select>
        <button class="ghost danger" onclick={() => removeRule(index)} aria-label="Remove rule for {rule.app}">Remove</button>
      </div>
    {/each}

    <form class="rule" onsubmit={(e) => { e.preventDefault(); addRule(); }}>
      <input type="text" class="grow" list="recent-apps" placeholder="App name or bundle id" bind:value={newRule.app} />
      <datalist id="recent-apps">
        {#each unruledApps as name (name)}<option value={name}></option>{/each}
      </datalist>
      <select bind:value={newRule.style} aria-label="Style for the new rule">
        {#each styles as style (style.id)}<option value={style.id}>{style.label}</option>{/each}
      </select>
      <button class="primary" type="submit" disabled={!newRule.app.trim()}>Add rule</button>
    </form>

    {#if unruledApps.length}
      <div class="chips">
        <span class="small muted">Recent:</span>
        {#each unruledApps.slice(0, 8) as name (name)}
          <button class="chip small" onclick={() => (newRule.app = name)}>{name}</button>
        {/each}
      </div>
    {/if}
  </section>
</div>

<style>
  .page { max-width: 900px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 10px; }
  .off { opacity: 0.55; }
  article { gap: 8px; }
  textarea { width: 100%; resize: vertical; font: inherit; font-size: 13px; line-height: 1.45; }
  .toggle { gap: 6px; font-weight: 500; }
  .rule { display: flex; align-items: center; gap: 8px; }
  .rule select { min-width: 150px; }
  .chips { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; }
  .chip { padding: 2px 10px; border-radius: 999px; }
  code { font-size: 12px; }
</style>
