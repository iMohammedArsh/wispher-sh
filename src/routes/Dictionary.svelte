<script lang="ts">
  import { api, type DictionaryEntry, type Suggestion } from "../lib/ipc";
  import { app } from "../lib/state.svelte";

  let entries = $state<DictionaryEntry[]>([]);
  let suggestions = $state<Suggestion[]>([]);
  let term = $state("");
  let replacement = $state("");
  let filter = $state("");

  async function load() {
    [entries, suggestions] = await Promise.all([api.dictionary(), api.suggestions()]);
  }

  $effect(() => {
    void app.dataVersion;
    load().catch(() => {});
  });

  const shown = $derived(
    entries.filter((e) => !filter || `${e.term} ${e.replacement ?? ""}`.toLowerCase().includes(filter.toLowerCase())),
  );

  async function add(event: SubmitEvent) {
    event.preventDefault();
    if (!term.trim()) return;
    await app.run(() => api.dictionaryUpsert(term, replacement.trim() || null), "Added");
    term = "";
    replacement = "";
    await load();
  }

  async function resolve(s: Suggestion, approve: boolean) {
    await app.run(() => api.resolveSuggestion(s.id, approve), approve ? "Added to dictionary" : undefined);
    await load();
  }
</script>

<div class="stack page">
  <div>
    <h1>Dictionary</h1>
    <p class="muted">Names, jargon and fixes. Hams listens for these words and spells them your way.</p>
  </div>

  {#if suggestions.length}
    <section class="card stack suggestions">
      <h2>Suggested from your edits</h2>
      {#each suggestions as s (s.id)}
        <div class="row">
          <span class="grow"><span class="muted">{s.heard}</span> → <strong>{s.correct}</strong>
            {#if s.count > 1}<span class="pill">{s.count}×</span>{/if}</span>
          <button onclick={() => resolve(s, false)}>Dismiss</button>
          <button class="primary" onclick={() => resolve(s, true)}>Add</button>
        </div>
      {/each}
    </section>
  {/if}

  <form class="card add" onsubmit={add}>
    <label class="grow">
      <span class="small muted">Word or phrase</span>
      <input type="text" bind:value={term} placeholder="e.g. Tauri, or 'jon doe'" />
    </label>
    <label class="grow">
      <span class="small muted">Always write it as (optional)</span>
      <input type="text" bind:value={replacement} placeholder="e.g. Jon Doh" />
    </label>
    <button class="primary" type="submit" disabled={!term.trim()}>Add</button>
  </form>

  {#if entries.length > 8}
    <input type="search" placeholder="Filter" bind:value={filter} />
  {/if}

  {#if entries.length === 0}
    <div class="card empty">Add product names, people and terms you use. They help recognition right away.</div>
  {:else}
    <ul class="card">
      {#each shown as e (e.id)}
        <li class="row">
          <span class="grow">
            {#if e.replacement}<span class="muted">{e.term}</span> → <strong>{e.replacement}</strong>{:else}<strong>{e.term}</strong>{/if}
            {#if e.source !== "user"}<span class="pill">{e.source}</span>{/if}
          </span>
          <button class="ghost small danger" onclick={() => app.run(async () => { await api.dictionaryDelete(e.id); await load(); })}>Remove</button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .page { max-width: 760px; }
  .suggestions { border-color: var(--accent); }
  .add { display: flex; gap: 10px; align-items: flex-end; }
  label { display: flex; flex-direction: column; gap: 4px; }
  ul { list-style: none; margin: 0; padding: 4px 14px; }
  li { padding: 8px 0; border-bottom: 1px solid var(--border); }
  li:last-child { border-bottom: none; }
</style>
