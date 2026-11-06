<script lang="ts">
  // "Teach a word": tell Hams how to write something it keeps mishearing,
  // or a name it should know. Saved straight to the Dictionary.
  import { api, errorText } from "../lib/ipc";
  import { app } from "../lib/state.svelte";
  import Icon from "./Icon.svelte";

  let { label = true }: { label?: boolean } = $props();

  let open = $state(false);
  let heard = $state("");
  let write = $state("");
  let saving = $state(false);
  let root: HTMLDivElement;
  let first = $state<HTMLInputElement>();

  function toggle() {
    if (open) {
      open = false;
      return;
    }
    // Start from whatever the person selected in the dictation.
    const selected = window.getSelection()?.toString().trim() ?? "";
    heard = selected.split(/\s+/).length <= 4 ? selected : "";
    write = "";
    open = true;
    queueMicrotask(() => first?.focus());
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    const term = heard.trim();
    const replacement = write.trim();
    if (!term) return;
    saving = true;
    try {
      await api.dictionaryUpsert(term, replacement && replacement !== term ? replacement : null);
      app.dataVersion++;
      app.notify(replacement && replacement !== term ? `Hams will write “${replacement}” from now on` : `Added “${term}” to your words`);
      open = false;
    } catch (error) {
      app.notify(errorText(error), "error");
    } finally {
      saving = false;
    }
  }

  function dismiss(event: MouseEvent | KeyboardEvent) {
    if (!open) return;
    if (event instanceof KeyboardEvent ? event.key === "Escape" : !root.contains(event.target as Node)) open = false;
  }
</script>

<svelte:window onclick={dismiss} onkeydown={dismiss} />

<div class="teach" bind:this={root}>
  <!-- mousedown would clear the selection we want to start from -->
  <button
    class={label ? "" : "ghost small icon-only"}
    title="Teach Hams a word"
    aria-label="Teach Hams a word"
    aria-expanded={open}
    onmousedown={(e) => e.preventDefault()}
    onclick={toggle}
  >
    <Icon name="dictionary" size={15} />
    {#if label}Teach a word{/if}
  </button>
  {#if open}
    <form class="popover" onsubmit={save}>
      <p class="popover-title">Teach Hams a word</p>
      <label class="field">
        <span>When I say</span>
        <input type="text" bind:this={first} bind:value={heard} placeholder="tory" autocomplete="off" spellcheck="false" />
      </label>
      <label class="field">
        <span>Write it as <span class="hint">Leave empty to just add the word</span></span>
        <input type="text" bind:value={write} placeholder="Tauri" autocomplete="off" spellcheck="false" />
      </label>
      <div class="bar">
        <button type="button" onclick={() => (open = false)}>Cancel</button>
        <button type="submit" class="primary" disabled={!heard.trim() || saving}>Save</button>
      </div>
    </form>
  {/if}
</div>

<style>
  .teach { position: relative; }
  .popover { left: 0; top: calc(100% + 4px); width: 264px; padding: 12px; display: flex; flex-direction: column; gap: 12px; }
  .popover .popover-title { padding: 0; }
  .field .hint { display: block; margin-top: 2px; }
  .bar { display: flex; justify-content: flex-end; gap: 8px; }
  .icon-only { width: 28px; padding: 0; }
  .icon-only + .popover { left: auto; right: 0; }
</style>
