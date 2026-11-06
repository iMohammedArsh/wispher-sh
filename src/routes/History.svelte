<script lang="ts">
  import { api, errorText, type HistoryItem } from "../lib/ipc";
  import { app } from "../lib/state.svelte";
  import { clockTime, dayLabel, groupedNumber } from "../lib/format";
  import Icon from "../components/Icon.svelte";
  import ConfirmButton from "../components/ConfirmButton.svelte";
  import HeardWritten from "../components/HeardWritten.svelte";
  import RewriteMenu from "../components/RewriteMenu.svelte";
  import TeachWord from "../components/TeachWord.svelte";

  let query = $state("");
  let appFilter = $state<string | null>(null);
  let items = $state.raw<HistoryItem[]>([]);
  let apps = $state.raw<string[]>([]);
  let loaded = $state(false);
  /** The one row that's expanded, and whether it's being edited. */
  let open = $state<number | null>(null);
  let editing = $state(false);
  let draft = $state("");
  let rewriting = $state(false);

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

  $effect(() => {
    api.recentApps().then((a) => (apps = a.slice(0, 6))).catch(() => {});
  });

  const styleNames: Record<string, string> = { messaging: "Message", email: "Email", code: "Code", docs: "Document", default: "" };

  const shown = $derived(appFilter ? items.filter((i) => i.app === appFilter) : items);
  // Pinned first, then one group per day, newest first.
  const sections = $derived.by(() => {
    const out: { label: string; items: HistoryItem[] }[] = [];
    const filtered = query || appFilter;
    const pinned = shown.filter((i) => i.pinned);
    if (pinned.length && !filtered) out.push({ label: "Pinned", items: pinned });
    for (const item of filtered ? shown : shown.filter((i) => !i.pinned)) {
      const label = dayLabel(item.created_at);
      const last = out[out.length - 1];
      if (last && last.label === label) last.items.push(item);
      else out.push({ label, items: [item] });
    }
    return out;
  });
  const totalWords = $derived(shown.reduce((a, i) => a + i.words, 0));

  function toggle(item: HistoryItem) {
    editing = false;
    open = open === item.id ? null : item.id;
  }

  function startEdit(item: HistoryItem) {
    draft = item.text;
    editing = true;
  }

  async function saveEdit(item: HistoryItem) {
    try {
      const learned = await api.historyUpdate(item.id, draft);
      editing = false;
      await load();
      app.notify(learned ? "Saved. Hams has a suggestion for you in Dictionary." : "Saved");
      void app.refreshSuggestions();
    } catch (error) {
      app.notify(errorText(error), "error");
    }
  }

  async function act(action: () => Promise<unknown>, done?: string) {
    await app.run(action, done);
    await load();
  }

  function keydown(event: KeyboardEvent) {
    if (event.key === "Escape" && open !== null && !document.querySelector(".popover")) {
      if (editing) editing = false;
      else open = null;
    }
  }
</script>

<svelte:window onkeydown={keydown} />

<div class="page">
  <header class="title-row">
    <div>
      <h1>History</h1>
      <p class="sub">
        {#if query || appFilter}{shown.length} {shown.length === 1 ? "dictation" : "dictations"}{appFilter ? ` in ${appFilter}` : ""} · {groupedNumber(totalWords)} words
        {:else if items.length}{groupedNumber(items.length)} dictations · {groupedNumber(totalWords)} words · kept only on this computer
        {:else}Kept only on this computer{/if}
      </p>
    </div>
  </header>

  <div class="filters">
    <label class="search">
      <Icon name="search" size={14} />
      <input type="search" placeholder="Search everything you've dictated" bind:value={query} aria-label="Search history" />
    </label>
    {#if apps.length > 1}
      <div class="chips" role="group" aria-label="Filter by app">
        <button class="chip" class:on={appFilter === null} aria-pressed={appFilter === null} onclick={() => (appFilter = null)}>All apps</button>
        {#each apps as name (name)}
          <button class="chip" class:on={appFilter === name} aria-pressed={appFilter === name} onclick={() => (appFilter = appFilter === name ? null : name)}>{name}</button>
        {/each}
      </div>
    {/if}
  </div>

  {#if loaded && shown.length === 0}
    <p class="empty">{query || appFilter ? "Nothing matches." : "No dictations yet. Hold your shortcut in any app to make the first one."}</p>
  {:else}
    {#each sections as section (section.label)}
      <section aria-label={section.label}>
        <h2 class="day">{section.label}</h2>
        <ul class="list">
          {#each section.items as item (item.id)}
            {@const expanded = open === item.id}
            <li class:expanded>
              <button class="row" aria-expanded={expanded} onclick={() => toggle(item)}>
                <span class="when num">{section.label === "Pinned" ? dayLabel(item.created_at).slice(0, 9) : clockTime(item.created_at)}</span>
                <span class="app">{item.app || "—"}</span>
                <span class="text clamp-2">{expanded ? "" : item.text.replace(/\s*\n+\s*/g, " ")}</span>
                <span class="tail">
                  {#if item.pinned}<Icon name="pin" size={13} />{/if}
                  {#if item.delivery === "failed"}<span class="pill warn">Not pasted</span>{:else if item.delivery === "copied"}<span class="pill">Copied</span>{/if}
                  {#if styleNames[item.style]}<span class="style">{styleNames[item.style]}</span>{/if}
                  <span class="words num">{item.words}</span>
                </span>
              </button>
              {#if expanded}
                <div class="detail">
                  {#if editing}
                    <!-- svelte-ignore a11y_autofocus -->
                    <textarea bind:value={draft} rows="4" autofocus aria-label="Edit dictation"></textarea>
                    <div class="bar">
                      <span class="small muted grow">Fixing a misheard word here teaches Hams for next time.</span>
                      <button onclick={() => (editing = false)}>Cancel</button>
                      <button class="primary" onclick={() => saveEdit(item)} disabled={draft.trim() === item.text.trim()}>Save</button>
                    </div>
                  {:else}
                    <div class="pair" class:fading={rewriting}><HeardWritten raw={item.raw} text={item.text} summary /></div>
                    <div class="bar">
                      <button onclick={() => app.run(() => api.copy(item.text), "Copied")}><Icon name="copy" size={15} />Copy</button>
                      <RewriteMenu id={item.id} current={item.style} label onbusy={(b) => (rewriting = b)} />
                      <TeachWord />
                      <button onclick={() => startEdit(item)}><Icon name="edit" size={15} />Edit</button>
                      <span class="grow"></span>
                      <button class="ghost" aria-pressed={item.pinned} onclick={() => act(() => api.historyPin(item.id, !item.pinned))}><Icon name="pin" size={15} />{item.pinned ? "Unpin" : "Pin"}</button>
                      <ConfirmButton ghost label="Delete" confirm="Delete it?" onconfirm={() => act(() => api.historyDelete(item.id))}><Icon name="trash" size={15} />Delete</ConfirmButton>
                    </div>
                  {/if}
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      </section>
    {/each}

    {#if !query && !appFilter && items.some((i) => !i.pinned)}
      <footer class="clear">
        <span class="muted">Clearing keeps your pinned dictations.</span>
        <ConfirmButton confirm="Delete all but pinned?" onconfirm={() => act(api.historyClear, "History cleared")}>Clear history</ConfirmButton>
      </footer>
    {/if}
  {/if}
</div>

<style>
  .filters { display: flex; flex-direction: column; gap: 10px; }
  .search { position: relative; display: block; color: var(--muted); }
  .search :global(.icon) { position: absolute; left: 11px; top: 10px; }
  .search input { padding-left: 32px; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip { height: 26px; padding: 0 10px; border-radius: 999px; font-size: 13px; font-weight: 400; color: var(--muted); border-color: var(--border); }
  .chip.on { background: var(--active); border-color: var(--active); color: var(--text); font-weight: 500; }

  .day { font-size: 13px; font-weight: 600; color: var(--muted); padding-bottom: 6px; border-bottom: 1px solid var(--border); }
  .list { list-style: none; margin: 0; padding: 0; }
  .list > li { border-bottom: 1px solid var(--rule); }
  .row {
    width: 100%; height: auto; padding: 10px 8px; margin: 0 -8px; box-sizing: content-box;
    display: grid; grid-template-columns: 64px 84px minmax(0, 1fr) auto; gap: 12px; align-items: baseline;
    border: none; border-radius: var(--radius-control); background: transparent;
    text-align: left; font-weight: 400; white-space: normal;
  }
  .row:hover:not(:disabled) { background: var(--raised); }
  .when { color: var(--muted); font-size: 13px; text-align: right; white-space: nowrap; }
  .app { color: var(--muted); font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .text { color: var(--text); line-height: 1.5; }
  .tail { display: flex; align-items: center; gap: 6px; color: var(--muted); }
  .style { font-size: 13px; }
  .words { min-width: 28px; text-align: right; font-size: 13px; }
  .expanded .row { background: transparent; }
  .detail { padding: 4px 0 16px 76px; --label-width: 84px; display: flex; flex-direction: column; gap: 14px; }
  .bar { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
  .fading { opacity: 0.45; transition: opacity 160ms; }
  .clear { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding-top: 8px; font-size: 13px; }

  @media (max-width: 760px) {
    .row { grid-template-columns: 56px minmax(0, 1fr) auto; }
    .app { display: none; }
    .detail { padding-left: 0; }
  }
</style>
