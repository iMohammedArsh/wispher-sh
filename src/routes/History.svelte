<script lang="ts">
  import { api, errorText, type HistoryItem, type StyleId } from "../lib/ipc";
  import { app } from "../lib/state.svelte";
  import { clockTime, dayLabel, groupedNumber } from "../lib/format";
  import Icon from "../components/Icon.svelte";
  import ConfirmButton from "../components/ConfirmButton.svelte";

  let query = $state("");
  let items = $state<HistoryItem[]>([]);
  let loaded = $state(false);
  let editing = $state<number | null>(null);
  let draft = $state("");
  /** The item whose "Rewrite as" menu is open, and the one being rewritten. */
  let menuFor = $state<number | null>(null);
  let rewriting = $state<number | null>(null);

  const rewriteStyles: { id: StyleId; label: string; hint: string }[] = [
    { id: "default", label: "Plain", hint: "Clear, natural writing" },
    { id: "messaging", label: "Message", hint: "Short and casual" },
    { id: "email", label: "Email", hint: "Professional, in paragraphs" },
    { id: "docs", label: "Document", hint: "Complete sentences" },
    { id: "code", label: "Code", hint: "Keeps symbols, no AI" },
  ];

  async function restyle(item: HistoryItem, style: (typeof rewriteStyles)[number]) {
    menuFor = null;
    rewriting = item.id;
    try {
      const result = await api.historyRestyle(item.id, style.id);
      await load();
      app.notify(result.polished || style.id === "code" ? `Rewritten as ${style.label.toLowerCase()}` : `Applied the ${style.label.toLowerCase()} style. Turn on AI polish in Settings for a full rewrite.`);
    } catch (error) {
      app.notify(errorText(error), "error");
    } finally {
      rewriting = null;
    }
  }

  function closeMenu(event: MouseEvent | KeyboardEvent) {
    if (menuFor === null) return;
    if (event instanceof KeyboardEvent ? event.key === "Escape" : !(event.target as Element).closest(".rewrite")) menuFor = null;
  }

  async function load() {
    try {
      items = await api.history(query, 300);
    } catch (error) {
      app.notify(errorText(error), "error");
    } finally {
      loaded = true;
    }
  }

  $effect(() => {
    void app.dataVersion;
    void query;
    const timer = setTimeout(load, 120);
    return () => clearTimeout(timer);
  });

  const styleNames: Record<string, string> = { messaging: "Messaging", email: "Email", code: "Code", docs: "Documents", default: "" };

  // Pinned first, then one group per day, newest first.
  const sections = $derived.by(() => {
    const out: { label: string; items: HistoryItem[] }[] = [];
    const pinned = items.filter((i) => i.pinned);
    if (pinned.length && !query) out.push({ label: "Pinned", items: pinned });
    for (const item of query ? items : items.filter((i) => !i.pinned)) {
      const label = dayLabel(item.created_at);
      const last = out[out.length - 1];
      if (last && last.label === label) last.items.push(item);
      else out.push({ label, items: [item] });
    }
    return out;
  });
  const totalWords = $derived(items.reduce((a, i) => a + i.words, 0));

  function edit(item: HistoryItem) {
    editing = item.id;
    draft = item.text;
  }

  async function saveEdit(item: HistoryItem) {
    try {
      const learned = await api.historyUpdate(item.id, draft);
      editing = null;
      await load();
      app.notify(learned ? "Saved. Hams has a dictionary suggestion for you in Dictionary." : "Saved");
    } catch (error) {
      app.notify(errorText(error), "error");
    }
  }

  async function act(action: () => Promise<unknown>, done?: string) {
    await app.run(action, done);
    await load();
  }
</script>

<svelte:window onclick={closeMenu} onkeydown={closeMenu} />

<div class="page">
  <header class="title-row">
    <div>
      <h1>History</h1>
      <p class="sub">
        {#if query}{items.length} {items.length === 1 ? "match" : "matches"}{:else if items.length}{groupedNumber(items.length)} dictations · {groupedNumber(totalWords)} words · stored only on this computer{:else}Stored only on this computer{/if}
      </p>
    </div>
    <div class="actions">
      <ConfirmButton disabled={!items.some((i) => !i.pinned)} confirm="Delete all but pinned?" onconfirm={() => act(api.historyClear, "History cleared")}>Clear history</ConfirmButton>
    </div>
  </header>

  <label class="search">
    <Icon name="search" size={14} />
    <input type="search" placeholder="Search everything you've dictated" bind:value={query} aria-label="Search history" />
  </label>

  {#if loaded && items.length === 0}
    <p class="empty">{query ? "Nothing matches that search." : "No dictations yet. Hold your shortcut in any app to make the first one."}</p>
  {:else}
    {#each sections as section (section.label)}
      <section aria-label={section.label}>
        <h2 class="day">{section.label}</h2>
        <ul class="panel flush rows">
          {#each section.items as item (item.id)}
            <li class:editing={editing === item.id} class:open={menuFor === item.id || rewriting === item.id}>
              {#if editing === item.id}
                <!-- svelte-ignore a11y_autofocus -->
                <textarea bind:value={draft} rows="4" autofocus aria-label="Edit dictation"></textarea>
                <div class="edit-bar">
                  <span class="small muted grow">Fixing a misheard word here teaches Hams for next time.</span>
                  <button onclick={() => (editing = null)}>Cancel</button>
                  <button class="primary" onclick={() => saveEdit(item)} disabled={draft.trim() === item.text.trim()}>Save</button>
                </div>
              {:else}
                <div class="entry">
                  <p class="text" class:fading={rewriting === item.id}>{item.text}</p>
                  <p class="meta small muted num">
                    <span>{section.label === "Pinned" ? `${dayLabel(item.created_at)}, ${clockTime(item.created_at)}` : clockTime(item.created_at)}</span>
                    {#if item.app}<span>{item.app}</span>{/if}
                    <span>{item.words} {item.words === 1 ? "word" : "words"}</span>
                    {#if styleNames[item.style]}<span>{styleNames[item.style]}</span>{/if}
                    {#if item.delivery === "failed"}<span class="pill warn">Not pasted</span>{:else if item.delivery === "copied"}<span class="pill">Copied only</span>{/if}
                  </p>
                </div>
                <div class="tools">
                  <button class="ghost small" title="Copy" aria-label="Copy" onclick={() => app.run(() => api.copy(item.text), "Copied")}><Icon name="copy" size={15} /></button>
                  <button class="ghost small" title="Edit" aria-label="Edit" onclick={() => edit(item)}><Icon name="edit" size={15} /></button>
                  <div class="rewrite">
                    <button class="ghost small" title="Rewrite as…" aria-label="Rewrite as another style" aria-haspopup="menu" aria-expanded={menuFor === item.id} disabled={rewriting !== null} onclick={() => (menuFor = menuFor === item.id ? null : item.id)}>
                      {#if rewriting === item.id}<span class="spinner" aria-label="Rewriting"></span>{:else}<Icon name="rewrite" size={15} />{/if}
                    </button>
                    {#if menuFor === item.id}
                      <div class="menu" role="menu" aria-label="Rewrite as">
                        <p class="menu-title">Rewrite as</p>
                        {#each rewriteStyles as style (style.id)}
                          <button role="menuitem" onclick={() => restyle(item, style)}>
                            <span class="grow"><span class="menu-label">{style.label}</span><span class="menu-hint">{style.hint}</span></span>
                            {#if item.style === style.id}<Icon name="check" size={14} />{/if}
                          </button>
                        {/each}
                      </div>
                    {/if}
                  </div>
                  <button class="ghost small" class:on={item.pinned} title={item.pinned ? "Unpin" : "Pin"} aria-label={item.pinned ? "Unpin" : "Pin"} aria-pressed={item.pinned} onclick={() => act(() => api.historyPin(item.id, !item.pinned))}><Icon name="pin" size={15} /></button>
                  <ConfirmButton small ghost label="Delete" confirm="Delete?" onconfirm={() => act(() => api.historyDelete(item.id))}><Icon name="trash" size={15} /></ConfirmButton>
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      </section>
    {/each}
  {/if}
</div>

<style>
  .search { position: relative; display: block; color: var(--muted); margin-bottom: -8px; }
  .search :global(.icon) { position: absolute; left: 11px; top: 10px; }
  .search input { padding-left: 32px; }
  .day { font-size: 13px; font-weight: 600; color: var(--muted); margin-bottom: 8px; }
  li { display: flex; gap: 16px; align-items: flex-start; }
  li.editing { flex-direction: column; align-items: stretch; gap: 10px; }
  .entry { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
  .text { white-space: pre-wrap; user-select: text; line-height: 1.5; }
  .meta { display: flex; flex-wrap: wrap; align-items: center; gap: 0 4px; }
  .meta > span:not(.pill) + span:not(.pill)::before { content: "·"; margin-right: 4px; }
  .meta .pill { margin-left: 6px; }
  .tools { display: flex; gap: 0; flex: none; opacity: 0; transition: opacity 120ms; }
  li:hover .tools, li:focus-within .tools, li.open .tools { opacity: 1; }
  .fading { opacity: 0.45; transition: opacity 160ms; }
  .rewrite { position: relative; }
  .menu {
    position: absolute; right: 0; top: calc(100% + 4px); z-index: 10; width: 228px;
    background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card);
    box-shadow: 0 10px 28px rgb(0 0 0 / 0.12); padding: 4px;
    animation: drop 120ms ease-out;
  }
  .menu-title { font-size: var(--fs-small); color: var(--muted); padding: 6px 8px 4px; }
  .rewrite .menu button { width: 100%; height: auto; padding: 6px 8px; justify-content: flex-start; text-align: left; border: none; background: transparent; gap: 8px; }
  .rewrite .menu button:hover { background: var(--active); }
  .menu-label { display: block; font-weight: 500; color: var(--text); }
  .menu-hint { display: block; font-size: var(--fs-small); color: var(--muted); font-weight: 400; }
  .spinner { width: 13px; height: 13px; border-radius: 50%; border: 1.6px solid var(--control); border-top-color: var(--text); animation: spin 700ms linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @keyframes drop { from { opacity: 0; transform: translateY(-3px); } }
  .tools :global(button) { width: 28px; padding: 0; }
  .tools :global(button.armed) { width: auto; padding: 0 8px; }
  .tools .on { opacity: 1; color: var(--text); }
  .edit-bar { display: flex; align-items: center; gap: 8px; }
  @media (hover: none) { .tools { opacity: 1; } }
</style>
