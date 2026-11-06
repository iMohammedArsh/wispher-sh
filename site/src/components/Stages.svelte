<script lang="ts">
  // The time one dictation takes, drawn to scale.
  import { STAGES } from "../lib/content";
  import { reveal } from "../lib/reveal";
</script>

<section class="how" aria-labelledby="how-title">
  <h2 id="how-title" class="head" use:reveal>Hold, speak, let go</h2>
  <div class="stages" data-ui use:reveal={1}>
    {#each STAGES as stage (stage.id)}
      <div class="stage {stage.id}" style:--grow={stage.grow}>
        <p class="dur">{stage.time}</p>
        <p class="clause">{stage.clause}</p>
      </div>
    {/each}
  </div>
</section>
