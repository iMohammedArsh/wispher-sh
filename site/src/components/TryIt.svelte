<script lang="ts">
  // The headline is an input: type the way you'd talk, and see what Hams'
  // own clean-up (before any AI polish) writes from it.
  import HeardWritten from "../../../src/components/HeardWritten.svelte";
  import { clean } from "../lib/clean";
  import { EXAMPLES } from "../lib/content";
  import { lines, onceInView, stagger, typeOut } from "../lib/motion";

  let spoken = $state(EXAMPLES[2].text);
  let input: HTMLInputElement;
  let section: HTMLElement;
  const written = $derived(clean(spoken));
  let stopTyping = () => {};

  // The first time the box comes into view it types its example out, as if
  // someone were saying it. Any touch from the visitor stops it.
  $effect(() => {
    const text = EXAMPLES[2].text;
    const stop = onceInView(section, () => {
      stopTyping = typeOut(text, (s) => (spoken = s), { speed: 38 });
    }, "0px 0px -30% 0px");
    return () => {
      stop();
      stopTyping();
    };
  });

  function use(text: string) {
    stopTyping();
    spoken = text;
    input.focus();
    queueMicrotask(() => input.setSelectionRange(text.length, text.length));
  }
</script>

<section id="try" class="try" aria-labelledby="try-title" bind:this={section}>
  <h2 id="try-title" class="head" use:lines><span class="line"><span>Type it the way you’d say it</span></span></h2>
  <div class="say" data-ui>
    <label class="visually-hidden" for="spoken">What you said</label>
    <input id="spoken" class="say-input" type="text" spellcheck="false" autocomplete="off" bind:value={spoken} bind:this={input} onfocus={() => stopTyping()} />
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
      <div class="examples" use:stagger={{ gap: 70, from: 2 }}>
        <p class="examples-title">Or try</p>
        {#each EXAMPLES as example (example.label)}
          <button type="button" class="example" onclick={() => use(example.text)}>{example.label}</button>
        {/each}
      </div>
      <p class="try-note" use:stagger={{ from: 6 }}>
        This box runs Hams’ own clean-up rules in your browser. In the app, Whisper does the hearing on your Mac, and if you turn on AI polish it also fixes grammar and punctuation.
      </p>
    </div>
  </div>
</section>
