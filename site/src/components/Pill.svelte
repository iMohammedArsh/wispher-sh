<script lang="ts">
  // The Hams overlay pill, drawn as it looks on screen (src/overlay/Overlay.svelte).
  import type { PillState } from "../lib/content";

  let { state }: { state: PillState } = $props();
</script>

{#snippet tick()}<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 8.5l3 3 6-7" /></svg>{/snippet}

<div class="pill {state.kind}" aria-hidden="true">
  {#if state.kind === "recording"}
    <span class="round x"><svg viewBox="0 0 16 16"><path d="M4.5 4.5l7 7M11.5 4.5l-7 7" /></svg></span>
    <span class="dot"></span>
    <span class="bars">{#each { length: 11 } as _, i (i)}<i></i>{/each}</span>
    <span class="time">{state.time}</span>
    <span class="round check">{@render tick()}</span>
  {:else if state.kind === "working"}
    <span class="bars thinking">{#each { length: 5 } as _, i (i)}<i></i>{/each}</span>{state.label}
  {:else}
    <span class="tick">{@render tick()}</span>{state.label}
  {/if}
</div>
