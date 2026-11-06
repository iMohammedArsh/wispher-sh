<script lang="ts" module>
  import type { StyleId } from "../lib/ipc";

  export const rewriteStyles: { id: StyleId; label: string; hint: string }[] = [
    { id: "default", label: "Plain", hint: "Clear, natural writing" },
    { id: "messaging", label: "Message", hint: "Short and casual" },
    { id: "email", label: "Email", hint: "Professional, in paragraphs" },
    { id: "docs", label: "Document", hint: "Complete sentences" },
    { id: "code", label: "Code", hint: "Keeps symbols, no AI" },
  ];
</script>

<script lang="ts">
  // "Rewrite as": runs a saved dictation through another style, from what was heard.
  import { api, errorText } from "../lib/ipc";
  import { app } from "../lib/state.svelte";
  import Icon from "./Icon.svelte";

  let {
    id,
    current,
    label = false,
    onbusy,
  }: {
    id: number;
    /** The item's style now, ticked in the menu. */
    current: string;
    /** Show "Rewrite as" next to the icon. */
    label?: boolean;
    onbusy?: (busy: boolean) => void;
  } = $props();

  let open = $state(false);
  let busy = $state(false);
  let root: HTMLDivElement;

  async function pick(style: (typeof rewriteStyles)[number]) {
    open = false;
    busy = true;
    onbusy?.(true);
    try {
      const result = await api.historyRestyle(id, style.id);
      app.dataVersion++;
      app.notify(
        result.polished || style.id === "code"
          ? `Rewritten as ${style.label.toLowerCase()}`
          : `Applied the ${style.label.toLowerCase()} style. Turn on AI polish in Settings for a full rewrite.`,
      );
    } catch (error) {
      app.notify(errorText(error), "error");
    } finally {
      busy = false;
      onbusy?.(false);
    }
  }

  function dismiss(event: MouseEvent | KeyboardEvent) {
    if (!open) return;
    if (event instanceof KeyboardEvent ? event.key === "Escape" : !root.contains(event.target as Node)) open = false;
  }
</script>

<svelte:window onclick={dismiss} onkeydown={dismiss} />

<div class="rewrite" bind:this={root}>
  <button
    class={label ? "" : "ghost small icon-only"}
    title="Rewrite as…"
    aria-label="Rewrite as another style"
    aria-haspopup="menu"
    aria-expanded={open}
    disabled={busy}
    onclick={() => (open = !open)}
  >
    {#if busy}<span class="spinner" aria-label="Rewriting"></span>{:else}<Icon name="rewrite" size={15} />{/if}
    {#if label}Rewrite as…{/if}
  </button>
  {#if open}
    <div class="popover" role="menu" aria-label="Rewrite as">
      <p class="popover-title">Rewrite as</p>
      {#each rewriteStyles as style (style.id)}
        <button role="menuitem" onclick={() => pick(style)}>
          <span class="grow"><span class="item-label">{style.label}</span><span class="item-hint">{style.hint}</span></span>
          {#if current === style.id}<Icon name="check" size={14} />{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .rewrite { position: relative; }
  .popover { left: 0; top: calc(100% + 4px); width: 228px; }
  .icon-only + .popover { left: auto; right: 0; }
  .icon-only { width: 28px; padding: 0; }
</style>
