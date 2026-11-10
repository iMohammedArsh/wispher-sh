<script lang="ts">
  // History, set as a departure board: one day of dictations, one per band.
  // Click a band to open it and see what was heard next to what was written.
  import HeardWritten from "../../../src/components/HeardWritten.svelte";
  import { BOARD, BOARD_OPEN } from "../lib/content";
  import { lines, stagger } from "../lib/motion";

  let open = $state(BOARD_OPEN);
</script>

<section id="history" class="history" aria-labelledby="history-title">
  <div class="history-head">
    <h2 id="history-title" class="head" use:lines><span class="line"><span>Every dictation, kept</span></span></h2>
    <p use:stagger={{ from: 2 }}>
      History keeps what was heard next to what was written, for every app you talked into today. Search it, copy one again, or rewrite it
      as an email. Open a row.
    </p>
  </div>
  <div class="board-window">
    <ol class="board" data-ui use:stagger={{ gap: 55 }} aria-label="Today’s dictations">
      {#each BOARD as row, i (i)}
        <li class="band" class:open={open === i}>
          <button type="button" class="band-head" aria-expanded={open === i} onclick={() => (open = i)}>
            <span class="b-app">{row.app}</span>
            <span class="b-time">{row.time}</span>
            <span class="b-words">{row.words.toLocaleString("en-US")} words</span>
            <span class="b-style">{row.style}</span>
            <span class="b-status {row.status.toLowerCase()}">{row.status}</span>
          </button>
          <div class="band-body" inert={open !== i}>
            <div class="band-inner">
              <p class="band-facts">{row.time} · {row.words.toLocaleString("en-US")} words · {row.style}</p>
              <HeardWritten raw={row.raw} text={row.text} summary />
            </div>
          </div>
        </li>
      {/each}
    </ol>
  </div>
</section>
