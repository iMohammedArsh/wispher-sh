<script lang="ts">
  import { api, type DictionaryEntry, type Suggestion } from "../lib/ipc";
  import { app } from "../lib/state.svelte";
  import Icon from "../components/Icon.svelte";

  let entries = $state<DictionaryEntry[]>([]);
  let suggestions = $state<Suggestion[]>([]);
  let loaded = $state(false);
  let term = $state("");
  let replacement = $state("");
  let filter = $state("");

  async function load() {
    [entries, suggestions] = await Promise.all([api.dictionary(), api.suggestions()]);
    loaded = true;
  }

  $effect(() => {
    void app.dataVersion;
    load().catch(() => {});
  });

  const shown = $derived(
    entries.filter((e) => !filter || `${e.term} ${e.replacement ?? ""}`.toLowerCase().includes(filter.toLowerCase())),
  );
  const fixes = $derived(entries.filter((e) => e.replacement).length);
  const sourceLabel: Record<string, string> = { user: "Added by you", learned: "Learned from an edit", imported: "From Hams v1" };

  async function add(event: SubmitEvent) {
    event.preventDefault();
    if (!term.trim()) return;
    await app.run(() => api.dictionaryUpsert(term, replacement.trim() || null), "Added to your dictionary");
    term = "";
    replacement = "";
    await load();
  }

  async function resolve(s: Suggestion, approve: boolean) {
    await app.run(() => api.resolveSuggestion(s.id, approve), approve ? `Hams will write “${s.correct}”` : undefined);
    await load();
  }
</script>

<div class="page narrow">
  <header class="title-row">
    <div>
      <h1>Dictionary</h1>
      <p class="sub">
        {#if entries.length}{entries.length} {entries.length === 1 ? "word" : "words"}, {fixes} with a fixed spelling. Hams listens for these and writes them your way.{:else}Names, jargon and fixes. Hams listens for these words and spells them your way.{/if}
      </p>
    </div>
  </header>

  {#if suggestions.length}
    <section aria-labelledby="suggest-title">
      <div class="section-head">
        <h2 id="suggest-title">Waiting for you <span class="pill warn">{suggestions.length}</span></h2>
        <span class="aside">Learned from your corrections. Nothing changes until you add it.</span>
      </div>
      <ul class="panel flush rows">
        {#each suggestions as s (s.id)}
          <li class="pair">
            <span class="heard">{s.heard}</span>
            <Icon name="chevron" size={14} />
            <strong class="grow">{s.correct}</strong>
            <span class="small muted num">{s.count === 1 ? "seen once" : `seen ${s.count} times`}</span>
            <button class="small" onclick={() => resolve(s, false)}>Dismiss</button>
            <button class="small primary" onclick={() => resolve(s, true)}>Add</button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  <form class="panel add" onsubmit={add}>
    <label class="field grow">
      <span>Word or phrase</span>
      <input type="text" bind:value={term} placeholder="Tauri, or what Hams hears, like “jon doe”" />
    </label>
    <label class="field grow">
      <span>Always write it as <span class="hint">optional</span></span>
      <input type="text" bind:value={replacement} placeholder="Jon Doh" />
    </label>
    <button class="primary" type="submit" disabled={!term.trim()}><Icon name="plus" />Add</button>
  </form>

  <section aria-labelledby="words-title">
    <div class="section-head">
      <h2 id="words-title">Your words</h2>
      {#if entries.length > 8}
        <input class="filter" type="search" placeholder="Filter" bind:value={filter} aria-label="Filter words" />
      {/if}
    </div>
    {#if loaded && entries.length === 0}
      <p class="empty">Add product names, people and terms you use. They help recognition from the next dictation.</p>
    {:else if shown.length === 0}
      <p class="empty">No word matches “{filter}”.</p>
    {:else}
      <table class="panel flush">
        <thead>
          <tr><th>Hams hears</th><th>Writes</th><th>Source</th><th><span class="visually-hidden">Actions</span></th></tr>
        </thead>
        <tbody>
          {#each shown as e (e.id)}
            <tr>
              <td class={e.replacement ? "muted" : ""}>{e.term}</td>
              <td><strong>{e.replacement ?? e.term}</strong></td>
              <td class="small muted">{sourceLabel[e.source] ?? e.source}</td>
              <td class="end">
                <button class="ghost small" aria-label="Remove {e.term}" title="Remove" onclick={() => app.run(async () => { await api.dictionaryDelete(e.id); await load(); })}><Icon name="trash" size={15} /></button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </section>
</div>

<style>
  .narrow { max-width: 820px; }
  .pair { display: flex; align-items: center; gap: 10px; }
  .pair :global(.icon) { color: var(--faint); }
  .heard { color: var(--muted); }
  .add { display: flex; gap: 12px; align-items: flex-end; padding: 16px var(--pad); }
  .filter { width: 200px; min-height: 30px; }
  table { width: 100%; border-collapse: separate; border-spacing: 0; overflow: hidden; }
  th { text-align: left; font-size: 12px; font-weight: 400; color: var(--muted); padding: 10px var(--pad) 8px; }
  td { padding: 9px var(--pad); border-top: 1px solid var(--rule); }
  th:first-child, td:first-child { width: 34%; }
  td.end { text-align: right; width: 56px; padding-right: 12px; }
  .visually-hidden { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); }
  h2 .pill { vertical-align: 2px; margin-left: 4px; }
</style>
