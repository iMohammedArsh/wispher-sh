<script lang="ts">
  import { api, type Snippet } from "../lib/ipc";
  import { app } from "../lib/state.svelte";

  let snippets = $state<Snippet[]>([]);
  let trigger = $state("");
  let expansion = $state("");

  async function load() {
    snippets = await api.snippets();
  }
  $effect(() => {
    load().catch(() => {});
  });

  async function add(event: SubmitEvent) {
    event.preventDefault();
    await app.run(() => api.snippetUpsert(trigger, expansion), "Snippet saved");
    trigger = "";
    expansion = "";
    await load();
  }
</script>

<div class="stack page">
  <div>
    <h1>Snippets</h1>
    <p class="muted">Say a short phrase, get the full text. Dictate just the trigger (or “insert …” plus the trigger) on its own.</p>
  </div>

  <form class="card stack" onsubmit={add}>
    <label>
      <span class="small muted">When I say</span>
      <input type="text" bind:value={trigger} placeholder="my address" />
    </label>
    <label>
      <span class="small muted">Type this</span>
      <textarea bind:value={expansion} rows="3" placeholder="221B Baker Street, London"></textarea>
    </label>
    <div class="row" style="justify-content: flex-end">
      <button class="primary" type="submit" disabled={!trigger.trim() || !expansion}>Save snippet</button>
    </div>
  </form>

  {#if snippets.length === 0}
    <div class="card empty">Ideas: your email, a meeting link, a sign-off, an address.</div>
  {:else}
    <ul>
      {#each snippets as s (s.id)}
        <li class="card">
          <div class="spread">
            <strong>“{s.trigger}”</strong>
            <button class="ghost small danger" onclick={() => app.run(async () => { await api.snippetDelete(s.id); await load(); })}>Delete</button>
          </div>
          <p class="muted expansion">{s.expansion}</p>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .page { max-width: 760px; }
  label { display: flex; flex-direction: column; gap: 4px; }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 8px; }
  li { padding: 10px 14px; }
  .expansion { white-space: pre-wrap; margin-top: 4px; }
</style>
