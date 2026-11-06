<script lang="ts">
  import { FAQ } from "../lib/content";
  import { reveal } from "../lib/reveal";
</script>

<section id="faq" class="faq" aria-labelledby="faq-title">
  <h2 id="faq-title" class="head faq-title" use:reveal>Before you install</h2>
  <div class="questions">
    {#each FAQ as item (item.q)}
      <details>
        <summary>{item.q}</summary>
        <div class="answer-text">{#each item.a as paragraph (paragraph)}<p>{paragraph}</p>{/each}</div>
      </details>
    {/each}
  </div>
</section>
