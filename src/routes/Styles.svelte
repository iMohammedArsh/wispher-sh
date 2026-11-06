<script lang="ts">
  import { api, errorText, type AppRule, type StyleId, type StyleInfo, type StyleOverride } from "../lib/ipc";
  import { app } from "../lib/state.svelte";
  import Icon from "../components/Icon.svelte";
  import HeardWritten from "../components/HeardWritten.svelte";

  const MAX_PROMPT = 1500;

  let styles = $state<StyleInfo[]>([]);
  let recentApps = $state<string[]>([]);
  let drafts = $state<Partial<Record<StyleId, string>>>({});
  let newRule = $state<AppRule>({ app: "", style: "messaging" });
  /** The style (or the app rules) shown in the detail pane. */
  let selected = $state<StyleId | "rules">("messaging");
  let trial = $state<{ style: StyleId; text: string; polished: boolean } | { style: StyleId; error: string } | null>(null);
  let trying = $state(false);

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

  // A typical rambling sentence for each style, to try the style on.
  const samples: Record<StyleId, string> = {
    messaging: "um yeah i can do thursday uh does three work for you",
    email: "hi sam new paragraph thanks for sending the contract over i've read it and i'm happy with everything except the payment terms could we talk tomorrow new paragraph best arsh",
    code: "rename the function handle submit to commit prompt and add a test for it",
    docs: "so the onboarding has three steps first permissions then the model download and uh finally a test dictation",
    default: "best low profile mechanical keyboard for mac",
  };

  const current = $derived(selected === "rules" ? null : (styles.find((x) => x.id === selected) ?? null));
  const rulesFor = (id: StyleId) => rules.filter((r) => r.style === id);

  async function tryStyle(style: StyleInfo) {
    await commitPrompt(style);
    trying = true;
    trial = null;
    try {
      const result = await api.styleTry(style.id, samples[style.id]);
      trial = { style: style.id, text: result.text, polished: result.polished };
    } catch (error) {
      trial = { style: style.id, error: errorText(error) };
    } finally {
      trying = false;
    }
  }

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

<div class="page">
  <header class="title-row">
    <div>
      <h1>Styles</h1>
      <p class="sub">Hams writes casually in chat, formally in email and leaves code alone. Pick a style to see and change how it writes.</p>
    </div>
    {#if s}
      <label class="row toggle">
        <input class="switch" type="checkbox" checked={s.app_styles} onchange={(e) => app.save({ app_styles: e.currentTarget.checked })} />
        Match the app
      </label>
    {/if}
  </header>

  {#if s && !s.app_styles}
    <div class="notice">
      <span class="grow">Matching is off, so every app gets the Default style.</span>
      <button onclick={() => app.save({ app_styles: true })}>Turn it on</button>
    </div>
  {:else if s?.ai.provider === "off"}
    <div class="notice">
      <span class="grow">These instructions shape the AI polish, which is off. Without it Hams still tidies messaging punctuation and leaves code untouched.</span>
      <button onclick={() => app.openSettings("ai")}>Set up AI</button>
    </div>
  {/if}

  <div class="split">
    <ul class="panel flush list" aria-label="Styles">
      {#each styles as style (style.id)}
        <li>
          <button class:active={selected === style.id} aria-current={selected === style.id} onclick={() => { selected = style.id; trial = null; }}>
            <span class="name">{style.label}{#if isEdited(style)}<span class="edited">Edited</span>{/if}</span>
            <span class="apps">{[...rulesFor(style.id).map((r) => r.app), examples[style.id]].join(", ")}</span>
          </button>
        </li>
      {/each}
      <li class="rules-item">
        <button class:active={selected === "rules"} aria-current={selected === "rules"} onclick={() => (selected = "rules")}>
          <span class="name">App rules <span class="count num">{rules.length}</span></span>
          <span class="apps">Pin an app to a style</span>
        </button>
      </li>
    </ul>

    {#if current}
      {@const style = current}
      <section class="panel detail" aria-labelledby="style-title">
        <div class="spread">
          <h2 id="style-title">{style.label}</h2>
          {#if isEdited(style)}
            <button class="ghost small" onclick={() => reset(style)}>Reset to default</button>
          {:else}
            <span class="small muted">Built-in</span>
          {/if}
        </div>
        <p class="muted small used">Used in {examples[style.id]}{#if rulesFor(style.id).length}, and by your rules for {rulesFor(style.id).map((r) => r.app).join(", ")}{/if}.</p>

        <label class="field">
          <span>Instructions for the AI</span>
          <textarea
            rows={Math.min(7, Math.max(3, Math.ceil(promptOf(style).length / 56)))}
            maxlength={MAX_PROMPT}
            value={promptOf(style)}
            placeholder={style.prompt}
            aria-label="{style.label} style instructions"
            oninput={(e) => (drafts[style.id] = e.currentTarget.value)}
            onblur={() => commitPrompt(style)}
          ></textarea>
        </label>
        <label class="switch-row">
          <span class="text">
            <strong>Polish with AI</strong>
            <span>{style.polish ? "Rewrites for grammar and this style." : "Off by default here: an AI rewrite can break code and commands."}</span>
          </span>
          <input class="switch" type="checkbox" checked={polishOf(style)} onchange={(e) => setOverride(style, { polish: e.currentTarget.checked })} />
        </label>

        <div class="try">
          <div class="spread">
            <h3>Try it</h3>
            <button class="small" onclick={() => tryStyle(style)} disabled={trying}>{#if trying}<span class="spinner"></span>Writing…{:else}Run{/if}</button>
          </div>
          {#if trial && trial.style === style.id && "text" in trial}
            <HeardWritten raw={samples[style.id]} text={trial.text} summary />
            {#if !trial.polished && polishOf(style)}<p class="muted small">AI polish is off in Settings, so this is the local clean-up only.</p>{/if}
          {:else if trial && trial.style === style.id && "error" in trial}
            <p class="danger-text small">{trial.error}</p>
          {:else}
            <p class="muted">Run it on something you might say here: <q>{samples[style.id]}</q></p>
          {/if}
        </div>
      </section>
    {:else}
      <section class="panel detail" aria-labelledby="rules-title">
        <h2 id="rules-title">App rules</h2>
        <p class="muted small used">Rules win over automatic detection. Use the app's name as it appears in History, or its bundle id, such as <code>com.google.Chrome</code>.</p>
        {#if rules.length}
          <ul class="rows rule-list">
            {#each rules as rule, index (rule.app)}
              <li class="rule">
                <span class="grow">{rule.app}</span>
                <select value={rule.style} onchange={(e) => setRuleStyle(index, e.currentTarget.value as StyleId)} aria-label="Style for {rule.app}">
                  {#each styles as option (option.id)}<option value={option.id}>{option.label}</option>{/each}
                </select>
                <button class="ghost small" onclick={() => removeRule(index)} aria-label="Remove rule for {rule.app}" title="Remove"><Icon name="trash" size={15} /></button>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="empty">No rules yet. Every app gets the style Hams detects for it.</p>
        {/if}

        <form class="rule" onsubmit={(e) => { e.preventDefault(); addRule(); }}>
          <input type="text" class="grow" list="recent-apps" placeholder="App name or bundle id" bind:value={newRule.app} aria-label="App" />
          <datalist id="recent-apps">
            {#each unruledApps as name (name)}<option value={name}></option>{/each}
          </datalist>
          <select bind:value={newRule.style} aria-label="Style for the new rule">
            {#each styles as option (option.id)}<option value={option.id}>{option.label}</option>{/each}
          </select>
          <button class="primary" type="submit" disabled={!newRule.app.trim()}><Icon name="plus" />Add</button>
        </form>

        {#if unruledApps.length}
          <div class="chips">
            <span class="small muted">You dictate into</span>
            {#each unruledApps.slice(0, 8) as name (name)}
              <button class="small chip" onclick={() => (newRule.app = name)}>{name}</button>
            {/each}
          </div>
        {/if}
      </section>
    {/if}
  </div>
</div>

<style>
  .toggle { gap: 8px; font-weight: 500; flex: none; margin-top: 4px; }
  .split { display: grid; grid-template-columns: 220px minmax(0, 1fr); gap: 16px; align-items: start; }
  .list { list-style: none; margin: 0; padding: 4px; }
  .list button {
    width: 100%; height: auto; padding: 8px 10px; border: none; background: transparent;
    display: flex; flex-direction: column; align-items: flex-start; gap: 2px; text-align: left; white-space: normal;
  }
  .list button:hover:not(.active) { background: var(--raised); }
  .list button.active { background: var(--active); }
  .name { font-weight: 500; display: flex; align-items: center; gap: 6px; }
  .apps { font-size: 12px; color: var(--muted); font-weight: 400; overflow: hidden; display: -webkit-box; -webkit-line-clamp: 1; line-clamp: 1; -webkit-box-orient: vertical; }
  .edited { font-size: 12px; font-weight: 400; color: var(--muted); }
  .count { font-weight: 400; color: var(--muted); }
  .rules-item { border-top: 1px solid var(--rule); margin-top: 4px; padding-top: 4px; }
  .detail { display: flex; flex-direction: column; gap: 16px; }
  .used { margin-top: -10px; }
  textarea { font-size: 13px; }
  .try { display: flex; flex-direction: column; gap: 12px; border-top: 1px solid var(--rule); padding-top: 16px; }
  .rule { display: flex; align-items: center; gap: 8px; }
  .rule-list { border: 1px solid var(--border); border-radius: var(--radius-card); }
  .rule-list .rule { padding: 8px 8px 8px 14px; }
  .rule select { width: 150px; flex: none; }
  .chips { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; }
  .chip { border-radius: 999px; }
  q { color: var(--text); }
  code { font-size: 12px; background: var(--raised); padding: 1px 4px; border-radius: 4px; }
  @media (max-width: 760px) { .split { grid-template-columns: 1fr; } }
</style>
