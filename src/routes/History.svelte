<script lang="ts">
  import { api, errorText, type HistoryItem } from "../lib/ipc";
  import { app } from "../lib/state.svelte";
  import { relativeTime } from "../lib/format";

  let query = $state("");
  let items = $state<HistoryItem[]>([]);
  let editing = $state<number | null>(null);
  let draft = $state("");

  async function load() {
    try {
      items = await api.history(query, 300);
    } catch (error) {
      app.notify(errorText(error), "error");
    }
  }

  $effect(() => {
    void app.dataVersion;
    void query;
    const timer = setTimeout(load, 120);
    return () => clearTimeout(timer);
  });

  function edit(item: HistoryItem) {
    editing = item.id;
    draft = item.text;
  }

  async function saveEdit(item: HistoryItem) {
    try {
      const learned = await api.historyUpdate(item.id, draft);
      editing = null;
      await load();
      app.notify(learned ? "Saved. Hams suggested a dictionary fix; review it in Dictionary." : "Saved");
    } catch (error) {
      app.notify(errorText(error), "error");
    }
  }

  async function clearAll() {
    if (!confirm("Delete all history except pinned items?")) return;
    await app.run(api.historyClear, "History cleared");
    await load();
  }
</script>

<div class="stack page">
  <div class="spread">
    <div>
      <h1>History</h1>
      <p class="muted">Everything you've dictated, stored only on this computer.</p>
    </div>
    <button class="danger" onclick={clearAll} disabled={!items.length}>Clear</button>
  </div>
  <input type="search" placeholder="Search your dictations" bind:value={query} />

  {#if items.length === 0}
    <div class="card empty">{query ? "Nothing matches that search." : "No dictations yet."}</div>
  {:else}
    <ul>
      {#each items as item (item.id)}
        <li class="card" class:pinned={item.pinned}>
          {#if editing === item.id}
            <!-- svelte-ignore a11y_autofocus -->
            <textarea bind:value={draft} rows="4" autofocus></textarea>
            <div class="row end">
              <span class="small muted grow">Fixing a misheard word teaches Hams for next time.</span>
              <button onclick={() => (editing = null)}>Cancel</button>
              <button class="primary" onclick={() => saveEdit(item)}>Save</button>
            </div>
          {:else}
            <p class="text">{item.text}</p>
            <div class="row meta">
              <span class="small muted grow">
                {relativeTime(item.created_at)}
                {#if item.app}· {item.app}{/if}
                {#if item.words}· {item.words} words{/if}
                {#if item.delivery === "failed"}<span class="pill warn">not pasted</span>{/if}
              </span>
              <button class="ghost small" onclick={() => app.run(() => api.copy(item.text), "Copied")}>Copy</button>
              <button class="ghost small" onclick={() => edit(item)}>Edit</button>
              <button class="ghost small" onclick={() => app.run(async () => { await api.historyPin(item.id, !item.pinned); await load(); })}>
                {item.pinned ? "Unpin" : "Pin"}
              </button>
              <button class="ghost small danger" onclick={() => app.run(async () => { await api.historyDelete(item.id); await load(); })}>Delete</button>
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .page { max-width: 860px; }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 8px; }
  li { padding: 12px 14px; display: flex; flex-direction: column; gap: 6px; }
  li.pinned { border-color: var(--accent); }
  .text { white-space: pre-wrap; user-select: text; }
  .meta { flex-wrap: wrap; }
  .end { justify-content: flex-end; }
</style>
