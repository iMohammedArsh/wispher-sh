<script lang="ts">
  import { api, type Snippet } from "../lib/ipc";
  import { app } from "../lib/state.svelte";
  import Icon from "../components/Icon.svelte";
  import ConfirmButton from "../components/ConfirmButton.svelte";

  let snippets = $state<Snippet[]>([]);
  let loaded = $state(false);
  let trigger = $state("");
  let expansion = $state("");

  async function load() {
    snippets = await api.snippets();
    loaded = true;
  }
  $effect(() => {
    load().catch(() => {});
  });

  async function save(event: SubmitEvent) {
    event.preventDefault();
    const existing = snippets.some((s) => s.trigger.toLowerCase() === trigger.trim().toLowerCase());
    await app.run(() => api.snippetUpsert(trigger, expansion), existing ? "Snippet updated" : "Snippet saved");
    trigger = "";
    expansion = "";
    await load();
  }

  function editSnippet(s: Snippet) {
    trigger = s.trigger;
    expansion = s.expansion;
  }
</script>

<div class="page narrow">
  <header class="title-row">
    <div>
      <h1>Snippets</h1>
      <p class="sub">Say a short phrase on its own, get the full text. “Insert” in front works too, as in “insert my address”.</p>
    </div>
  </header>

  <form class="panel form" onsubmit={save}>
    <label class="field">
      <span>When I say</span>
      <input type="text" bind:value={trigger} placeholder="my address" />
    </label>
    <label class="field">
      <span>Type this</span>
      <textarea bind:value={expansion} rows="3" placeholder="221B Baker Street, London NW1 6XE"></textarea>
    </label>
    <div class="row end">
      {#if trigger || expansion}<button type="button" class="ghost" onclick={() => { trigger = ""; expansion = ""; }}>Clear</button>{/if}
      <button class="primary" type="submit" disabled={!trigger.trim() || !expansion}>
        {snippets.some((s) => s.trigger.toLowerCase() === trigger.trim().toLowerCase()) ? "Update snippet" : "Save snippet"}
      </button>
    </div>
  </form>

  <section aria-labelledby="list-title">
    <div class="section-head">
      <h2 id="list-title">Your snippets</h2>
      {#if snippets.length}<span class="aside num">{snippets.length}</span>{/if}
    </div>
    {#if loaded && snippets.length === 0}
      <p class="empty">Ideas: your email, a meeting link, a sign-off, your address, a reply you send every week.</p>
    {:else}
      <ul class="panel flush rows">
        {#each snippets as s (s.id)}
          <li>
            <span class="trigger">“{s.trigger}”</span>
            <p class="expansion clamp-2">{s.expansion}</p>
            <div class="tools">
              <button class="ghost small" title="Edit" aria-label="Edit {s.trigger}" onclick={() => editSnippet(s)}><Icon name="edit" size={15} /></button>
              <ConfirmButton small ghost label="Delete {s.trigger}" confirm="Delete?" onconfirm={() => app.run(async () => { await api.snippetDelete(s.id); await load(); })}><Icon name="trash" size={15} /></ConfirmButton>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</div>

<style>
  .narrow { max-width: 820px; }
  .form { display: flex; flex-direction: column; gap: 16px; }
  .end { justify-content: flex-end; }
  li { display: grid; grid-template-columns: minmax(140px, 220px) 1fr auto; gap: 16px; align-items: start; }
  .trigger { font-weight: 600; }
  .expansion { color: var(--muted); white-space: pre-wrap; }
  .tools { display: flex; margin: -3px 0; }
  .tools :global(button) { width: 28px; padding: 0; }
  .tools :global(button.armed) { width: auto; padding: 0 8px; }
</style>
