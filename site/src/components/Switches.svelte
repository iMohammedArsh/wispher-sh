<script lang="ts">
  // What Hams can do, as sentences with their real switch at the end. A band
  // that's on fills with colour and shows that feature at work. Before the
  // script runs (or without it), every band is shown open.
  import HeardWritten from "../../../src/components/HeardWritten.svelte";
  import { SWITCHES } from "../lib/content";
  import { lines, stagger } from "../lib/motion";

  let on = $state(SWITCHES.map((s) => s.on));
  let ready = $state(false);
  $effect(() => {
    ready = true;
  });
</script>

<section class="switches" aria-labelledby="switches-title">
  <h2 id="switches-title" class="head" use:lines><span class="line"><span>Switch on what you need</span></span></h2>
  <ul class="bands" use:stagger={{ gap: 70 }}>
    {#each SWITCHES as item, i (item.id)}
      {@const isOn = !ready || on[i]}
      <li class="sw {item.id}" class:on={isOn}>
        <div class="sw-row">
          <p class="sw-says" id="sw-{item.id}">{item.says}</p>
          <button
            type="button"
            class="toggle"
            data-ui
            role="switch"
            aria-checked={isOn}
            aria-labelledby="sw-{item.id}"
            onclick={() => (on[i] = !on[i])}
          ><span class="knob"></span></button>
        </div>
        <div class="sw-body" inert={!isOn}>
          <div class="sw-inner">
            <p class="sw-then">{item.then}</p>
            {#if "raw" in item.show}
              <div class="sw-pair" data-ui><HeardWritten raw={item.show.raw} text={item.show.text} summary /></div>
            {:else}
              <img
                class="sw-shot"
                src={item.show.src}
                width={item.show.width}
                height={item.show.height}
                style:object-position={item.show.focus}
                alt={item.show.alt}
                loading="lazy"
              />
            {/if}
          </div>
        </div>
      </li>
    {/each}
  </ul>
</section>
