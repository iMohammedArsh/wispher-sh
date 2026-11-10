<script lang="ts">
  // One dictation, step by step. A Notes window and the Hams pill stay pinned
  // while the four steps scroll past, and both change with the step you're on.
  // Under the window, the steps are drawn to their real lengths.
  import HeardWritten from "../../../src/components/HeardWritten.svelte";
  import { STEPS, STEP_TEXT } from "../lib/content";
  import { lines } from "../lib/motion";
  import Pill from "./Pill.svelte";

  const GROW = [6, 0.6, 1, 0.25];
  let active = $state(0);
  let seconds = $state(4);
  let steps: HTMLElement[] = [];

  $effect(() => {
    const io = new IntersectionObserver(
      (entries) => {
        for (const e of entries) if (e.isIntersecting) active = steps.indexOf(e.target as HTMLElement);
      },
      { rootMargin: "-45% 0px -45% 0px" },
    );
    steps.forEach((s) => io.observe(s));
    return () => io.disconnect();
  });

  // The recording clock runs while the first step is on screen.
  $effect(() => {
    if (active !== 0) return;
    seconds = 1;
    const t = setInterval(() => (seconds = (seconds % 9) + 1), 1000);
    return () => clearInterval(t);
  });

  const step = $derived(STEPS[active]);
  const pill = $derived(step.pill.kind === "recording" ? { kind: "recording" as const, time: `0:0${seconds}` } : step.pill);
</script>

<section class="how" aria-labelledby="how-title">
  <h2 id="how-title" class="head how-title" use:lines><span class="line"><span>Hold, speak, let go</span></span></h2>
  <div class="how-steps">
    {#each STEPS as s, i (s.says)}
      <article class="step" class:on={active === i} bind:this={steps[i]}>
        <h3>{s.says}</h3>
        <p class="step-body"><strong>{s.time}.</strong> {s.body}</p>
        <div class="step-pill" data-ui><Pill state={s.pill} /></div>
      </article>
    {/each}
  </div>
  <div class="how-stage">
    <figure class="how-fig" data-ui aria-label="A Notes window and the Hams pill during one dictation">
      <div class="how-field">
        <div class="note-win">
          <p class="app-name">Notes · Launch plan</p>
          {#key step.window}
            <div class="note-body">
              {#if step.window === "empty"}
                <p class="note-line"><span class="caret"></span></p>
              {:else if step.window === "heard"}
                <p class="note-line heard-only">{STEP_TEXT.raw}</p>
              {:else if step.window === "diff"}
                <HeardWritten raw={STEP_TEXT.raw} text={STEP_TEXT.text} summary />
              {:else}
                <p class="note-line written-only">{STEP_TEXT.text}<span class="caret"></span></p>
              {/if}
            </div>
          {/key}
          <div class="how-pill"><Pill state={pill} /></div>
        </div>
        <div class="timeline" aria-hidden="true">
          {#each STEPS as s, i (s.says)}
            <span class="seg" class:on={active === i} class:done={active > i} style:--grow={GROW[i]}><i></i></span>
          {/each}
        </div>
      </div>
      <figcaption>{step.time}</figcaption>
    </figure>
  </div>
</section>
