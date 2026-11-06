<script lang="ts">
  // The headline is an input: type the way you'd talk, and see what Hams'
  // own clean-up (before any AI polish) writes from it.
  import HeardWritten from "../../../src/components/HeardWritten.svelte";
  import { clean } from "../lib/clean";
  import { EXAMPLES } from "../lib/content";
  import { reveal } from "../lib/reveal";

  let spoken = $state(EXAMPLES[2].text);
  let input: HTMLInputElement;
  const written = $derived(clean(spoken));

  function use(text: string) {
    spoken = text;
    input.focus();
    queueMicrotask(() => input.setSelectionRange(text.length, text.length));
  }
</script>

<section id="try" class="try" aria-labelledby="try-title">
  <h2 id="try-title" class="head" use:reveal>Type it the way you’d say it</h2>
  <div class="say" data-ui>
    <label class="visually-hidden" for="spoken">What you said</label>
    <input id="spoken" class="say-input" type="text" spellcheck="false" autocomplete="off" bind:value={spoken} bind:this={input} />
  </div>
  <div class="try-body">
    <div class="answer" data-ui aria-live="polite">
      {#if written}
        <HeardWritten raw={spoken} text={written} summary />
      {:else}
        <p class="nothing">Nothing left to paste. Every word was a filler or got scratched.</p>
      {/if}
    </div>
    <div class="try-side">
      <div class="examples" use:reveal={1}>
        <p class="examples-title">Or try</p>
        {#each EXAMPLES as example (example.label)}
          <button type="button" class="example" onclick={() => use(example.text)}>{example.label}</button>
        {/each}
      </div>
      <p class="try-note" use:reveal={2}>
        This box runs Hams’ own clean-up rules in your browser. In the app, Whisper does the hearing on your Mac, and if you turn on AI polish it also fixes grammar and punctuation.
      </p>
    </div>
  </div>
</section>
