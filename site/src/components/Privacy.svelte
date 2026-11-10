<script lang="ts">
  // Everything Hams sends over the network, as a list you can check. Turning
  // AI polish on adds the one thing that then leaves: the written text.
  import { LEDGER } from "../lib/content";
  import { lines, stagger } from "../lib/motion";

  let polish = $state(false);
</script>

<section id="privacy" class="privacy" aria-labelledby="privacy-title">
  <div class="privacy-text">
    <h2 id="privacy-title" class="head" use:lines><span class="line"><span>Your voice stays on your Mac</span></span></h2>
    <p class="statement" use:stagger={{ from: 2 }}>
      <strong>Hams</strong> <span lang="ar">(همس)</span> is Arabic for a whisper. Whisper, the open speech model, runs inside the app on your
      Mac’s own chip. Nothing is recorded to keep, and it works on a plane.
    </p>
  </div>
  <figure class="ledger" data-ui use:stagger={{ gap: 70, from: 3 }} aria-label="What Hams sends over the network">
    <p class="ledger-title" data-part>What goes online</p>
    {#each LEDGER as row (row.what)}
      <div class="ledger-row" data-part>
        <span class="l-what">{row.what}</span>
        <span class="l-where">{row.where}</span>
        <span class="l-tag" class:local={row.local}>{row.local ? "Stays here" : "Once in a while"}</span>
      </div>
    {/each}
    <div class="ledger-row polish" class:on={polish} data-part>
      <span class="l-what" id="ledger-polish">AI polish</span>
      <span class="l-where">
        {#if polish}The written text only, to the provider you chose, with your key from the keychain{:else}Off, so nothing is sent{/if}
      </span>
      <button type="button" class="toggle" role="switch" aria-checked={polish} aria-labelledby="ledger-polish" onclick={() => (polish = !polish)}
        ><span class="knob"></span></button>
    </div>
    <p class="ledger-foot" data-part>Or run the polish on your Mac too, with Ollama. The code is open on GitHub.</p>
  </figure>
</section>
