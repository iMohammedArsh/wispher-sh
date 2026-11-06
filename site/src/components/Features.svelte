<script lang="ts">
  // Rows hung from one axis, each with a real screen of the app at its right.
  import { FEATURES } from "../lib/content";
  import { reveal } from "../lib/reveal";
</script>

<section class="does" aria-labelledby="does-title">
  <h2 id="does-title" class="head does-title" use:reveal>What it does, quietly</h2>
  {#each FEATURES as feature (feature.title)}
    <article class="row" use:reveal>
      <div class="row-text">
        <h3>{feature.title}</h3>
        <p>{feature.body}</p>
      </div>
      <img
        class="shot"
        src={feature.image.src}
        width={feature.image.width}
        height={feature.image.height}
        style:width="min(100%, {feature.image.max}px)"
        loading="lazy"
        alt={feature.image.alt}
      />
    </article>
  {/each}
</section>
