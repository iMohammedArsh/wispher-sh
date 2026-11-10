<script lang="ts">
  // "You talk. … Hams types." with real dictations passing between the two
  // halves. The stage is sticky; scrolling through the section moves the reel
  // one dictation at a time. The first frame only shows as the strip's top edge.
  import { DMG, FRAMES } from "../lib/content";
  import AppFrame from "./AppFrame.svelte";

  let hero: HTMLElement;
  let index = $state(1);

  $effect(() => {
    let queued = false;
    const step = () => {
      queued = false;
      const travel = hero.offsetHeight - innerHeight;
      const progress = travel > 0 ? Math.min(1, Math.max(0, -hero.getBoundingClientRect().top / travel)) : 0;
      const steps = FRAMES.length - 1;
      index = 1 + Math.min(steps - 1, Math.floor(progress * steps));
      hero.style.setProperty("--p", progress.toFixed(3));
    };
    const onScroll = () => {
      if (queued) return;
      queued = true;
      requestAnimationFrame(step);
    };
    step();
    addEventListener("scroll", onScroll, { passive: true });
    addEventListener("resize", onScroll);
    return () => {
      removeEventListener("scroll", onScroll);
      removeEventListener("resize", onScroll);
    };
  });
</script>

<section class="hero" aria-labelledby="hero-title" bind:this={hero}>
  <div class="hero-stage">
    <h1 id="hero-title" class="hero-title">
      <span class="half line"><span>You talk.</span></span>
      <span class="half second line"><span>Hams types.</span></span>
    </h1>
    <div class="reel-window">
      <div class="reel" style:--i={index}>
        {#each FRAMES as frame, i (i)}
          <AppFrame {frame} on={i === index} />
        {/each}
      </div>
    </div>
    <div class="hero-foot">
      <p>Hams is a free Mac app. Hold <kbd>⌘</kbd><kbd>⇧</kbd><kbd>Space</kbd> in any app, speak, let go, and clean text appears at your cursor.</p>
      <div class="hero-actions">
        <a class="button voice" href={DMG}>Download for Mac</a>
        <a class="link" href="#get">or install with one line</a>
      </div>
    </div>
  </div>
</section>
