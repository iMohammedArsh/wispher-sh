<script lang="ts">
  import { api, type AppRule, type StyleId, type StyleInfo, type StyleOverride } from "../lib/ipc";
  import { app } from "../lib/state.svelte";
  import Icon from "../components/Icon.svelte";

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

<div class="page">
  <header class="title-row">
    <div>
      <h1>Styles</h1>
      <p class="sub">Hams writes casually in chat, formally in email and leaves code alone. Edit any style to make it yours.</p>
    </div>
    {#if s}
      <label class="row toggle">
        <input class="switch" type="checkbox" checked={s.app_styles} onchange={(e) => app.save({ app_styles: e.currentTarget.checked })} />
        Match the app
      </label>
    {/if}
  </header>

  {#if s?.ai.provider === "off"}
    <div class="notice">
      <span class="grow">These instructions shape the AI polish, which is off. Without it Hams still tidies messaging punctuation and leaves code untouched.</span>
      <button onclick={() => (app.route = "settings")}>Set up AI</button>
    </div>
  {/if}

  <div class:off={!s?.app_styles}>
    {#each styles as style (style.id)}
      <section class="setting" aria-labelledby="style-{style.id}">
        <header>
          <h2 id="style-{style.id}">{style.label}</h2>
          <p>{examples[style.id]}</p>
        </header>
        <div class="stack">
          <textarea
            rows={Math.min(6, Math.max(3, Math.ceil(promptOf(style).length / 64)))}
            maxlength={MAX_PROMPT}
            value={promptOf(style)}
            placeholder={style.prompt}
            aria-label="{style.label} style instructions"
            oninput={(e) => (drafts[style.id] = e.currentTarget.value)}
            onblur={() => commitPrompt(style)}
          ></textarea>
          <div class="spread">
            <label class="row small">
              <input class="switch" type="checkbox" checked={polishOf(style)} onchange={(e) => setOverride(style, { polish: e.currentTarget.checked })} />
              <span>Polish with AI{#if !style.polish}<span class="muted">&nbsp;· off by default, AI rewrites can break commands</span>{/if}</span>
            </label>
            {#if isEdited(style)}
              <button class="ghost small" onclick={() => reset(style)}>Reset to default</button>
            {:else}
              <span class="small muted">Default</span>
            {/if}
          </div>
        </div>
      </section>
    {/each}

    <section class="setting" aria-labelledby="rules-title">
      <header>
        <h2 id="rules-title">App rules</h2>
        <p>Pin an app to a style. Rules win over automatic detection. Use the app's name as it appears in History, or its bundle id, such as <code>com.google.Chrome</code>.</p>
      </header>
      <div class="stack">
        {#if rules.length}
          <ul class="panel flush rows">
            {#each rules as rule, index (rule.app)}
              <li class="rule">
                <span class="grow">{rule.app}</span>
                <select value={rule.style} onchange={(e) => setRuleStyle(index, e.currentTarget.value as StyleId)} aria-label="Style for {rule.app}">
                  {#each styles as style (style.id)}<option value={style.id}>{style.label}</option>{/each}
                </select>
                <button class="ghost small" onclick={() => removeRule(index)} aria-label="Remove rule for {rule.app}" title="Remove"><Icon name="trash" size={15} /></button>
              </li>
            {/each}
          </ul>
        {/if}

        <form class="rule" onsubmit={(e) => { e.preventDefault(); addRule(); }}>
          <input type="text" class="grow" list="recent-apps" placeholder="App name or bundle id" bind:value={newRule.app} aria-label="App" />
          <datalist id="recent-apps">
            {#each unruledApps as name (name)}<option value={name}></option>{/each}
          </datalist>
          <select bind:value={newRule.style} aria-label="Style for the new rule">
            {#each styles as style (style.id)}<option value={style.id}>{style.label}</option>{/each}
          </select>
          <button class="primary" type="submit" disabled={!newRule.app.trim()}><Icon name="plus" />Add rule</button>
        </form>

        {#if unruledApps.length}
          <div class="chips">
            <span class="small muted">You dictate into</span>
            {#each unruledApps.slice(0, 8) as name (name)}
              <button class="small chip" onclick={() => (newRule.app = name)}>{name}</button>
            {/each}
          </div>
        {/if}
      </div>
    </section>
  </div>
</div>

<style>
  .off { opacity: 0.6; }
  .toggle { gap: 8px; font-weight: 500; flex: none; margin-top: 4px; }
  textarea { font-size: 13.5px; }
  .rule { display: flex; align-items: center; gap: 8px; }
  .rows .rule { padding-top: 8px; padding-bottom: 8px; }
  .rule select { width: 170px; flex: none; }
  .chips { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; }
  .chip { border-radius: 999px; }
  code { font-size: 12px; background: var(--raised); padding: 1px 4px; border-radius: 4px; }
</style>
