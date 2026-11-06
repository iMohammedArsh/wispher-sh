<script lang="ts">
  // One dictation in one app, as a cropped window with the pill at its foot.
  import HeardWritten from "../../../src/components/HeardWritten.svelte";
  import type { Frame } from "../lib/content";
  import Pill from "./Pill.svelte";

  let { frame, on = false }: { frame: Frame; on?: boolean } = $props();
</script>

<figure class="frame" class:on data-ui aria-label={frame.app}>
  <div class="win">
    <p class="app-name">{frame.app}</p>
    {#if "live" in frame}
      <p class="live-caption">{frame.live}</p>
    {:else if "note" in frame}
      <p class="note-title">{frame.note.title}</p>
      <ul class="points">{#each frame.note.points as point (point)}<li>{point}</li>{/each}</ul>
    {:else}
      <HeardWritten raw={frame.raw} text={frame.text} />
    {/if}
  </div>
  <Pill state={frame.pill} />
</figure>
