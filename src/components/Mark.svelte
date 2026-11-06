<script lang="ts">
  // The Hams glyph: five voice-wave bars that also read as the letter H.
  let { size = 18 }: { size?: number } = $props();
</script>

<svg class="mark" viewBox="212 212 600 600" width={size} height={size} aria-hidden="true">
  <rect x="252" y="252" width="92" height="520" rx="46" />
  <rect x="371" y="402" width="76" height="220" rx="38" />
  <rect x="474" y="352" width="76" height="320" rx="38" />
  <rect x="577" y="402" width="76" height="220" rx="38" />
  <rect x="680" y="252" width="92" height="520" rx="46" />
</svg>

<style>
  .mark { fill: var(--data); flex: none; }
</style>
