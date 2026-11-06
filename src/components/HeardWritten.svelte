<script lang="ts">
  // The Heard / Written pair: what Whisper heard above what Hams wrote, with
  // the words Hams dropped struck through and the words it changed underlined.
  import { changeSummary, diffWords, type Run } from "../lib/diff";

  let {
    raw,
    text,
    compact = false,
    summary = false,
  }: {
    raw: string;
    text: string;
    /** One clamped line per side, for lists. */
    compact?: boolean;
    /** Show "2 words removed · 1 changed" under the pair. */
    summary?: boolean;
  } = $props();

  const diff = $derived(diffWords(raw, text));
  // No word changed (only punctuation or capitals, or no transcript kept): one row says it all.
  const same = $derived(!raw.trim() || (diff.dropped === 0 && diff.added === 0));
  const note = $derived(changeSummary(diff));
</script>

{#snippet line(runs: Run[])}
  {#each runs as run, r}
    {#if run.kind === "keep"}
      {#each run.words as word, w}{#if word === "\n"}<br />{:else}{r || w ? " " : ""}{word}{/if}{/each}
    {:else}
      {r ? " " : ""}<span class={run.kind}>{#each run.words as word, w}{#if word === "\n"}<br />{:else}{w ? " " : ""}{word}{/if}{/each}</span>
    {/if}
  {/each}
{/snippet}

<div class="pair" class:compact>
  {#if !same}
    <span class="label">Heard</span>
    <p class="heard">{@render line(diff.heard)}</p>
  {/if}
  <span class="label">Written</span>
  <p class="written">{@render line(diff.written)}</p>
  {#if summary && !compact}
    <span></span>
    <p class="note">{same ? (raw.trim() && raw.trim() !== text.trim() ? "Written as heard, with punctuation and capitals added." : "Written exactly as heard.") : note}</p>
  {/if}
</div>

<style>
  .pair {
    display: grid; grid-template-columns: var(--label-width, 64px) minmax(0, 1fr);
    column-gap: 12px; row-gap: 10px; align-items: baseline;
  }
  .label { font-size: var(--fs-small); color: var(--muted); }
  p { margin: 0; user-select: text; overflow-wrap: anywhere; }
  .heard { color: var(--muted); line-height: 1.55; }
  .written { color: var(--text); font-size: 16px; line-height: 1.55; }
  .heard :global(.drop) {
    color: var(--faint);
    text-decoration: line-through; text-decoration-thickness: 1px; text-decoration-color: var(--faint);
  }
  .written :global(.add) {
    text-decoration: underline; text-decoration-thickness: 2px;
    text-decoration-color: var(--data); text-underline-offset: 4px; text-decoration-skip-ink: none;
  }
  .note { font-size: var(--fs-small); color: var(--muted); margin-top: -2px; }
  .compact { row-gap: 4px; }
  .compact .written { font-size: var(--fs-body); }
  .compact p { overflow: hidden; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; }
  .compact .heard { -webkit-line-clamp: 1; line-clamp: 1; }
</style>
