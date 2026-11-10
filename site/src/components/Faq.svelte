<script lang="ts">
  // The heading stays while the questions scroll past; one answer open at a time.
  import { FAQ } from "../lib/content";
  import { lines, stagger } from "../lib/motion";
</script>

<section id="faq" class="faq" aria-labelledby="faq-title">
  <div class="faq-side">
    <h2 id="faq-title" class="head" use:lines><span class="line"><span>Before you install</span></span></h2>
  </div>
  <div class="tiles" use:stagger={{ gap: 60 }}>
    {#each FAQ as item, i (item.q)}
      <details name="faq" open={i === 0}>
        <summary>{item.q}<span class="plus" aria-hidden="true"><i></i><i></i></span></summary>
        <div class="answer-text">{#each item.a as paragraph (paragraph)}<p>{paragraph}</p>{/each}</div>
      </details>
    {/each}
  </div>
</section>
