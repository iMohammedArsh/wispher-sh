<script lang="ts">
  // A destructive action that needs a second click within a few seconds.
  // Used instead of window.confirm, which the desktop webview may not show.
  import type { Snippet } from "svelte";

  let {
    onconfirm,
    confirm = "Click again to confirm",
    disabled = false,
    small = false,
    ghost = false,
    label,
    children,
  }: {
    onconfirm: () => void | Promise<void>;
    confirm?: string;
    disabled?: boolean;
    small?: boolean;
    ghost?: boolean;
    label?: string;
    children: Snippet;
  } = $props();

  let armed = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  function click() {
    if (!armed) {
      armed = true;
      clearTimeout(timer);
      timer = setTimeout(() => (armed = false), 4000);
      return;
    }
    armed = false;
    clearTimeout(timer);
    void onconfirm();
  }
</script>

<button class="danger" class:small class:ghost={ghost && !armed} class:armed {disabled} aria-label={armed ? confirm : label} onclick={click}>
  {#if armed}{confirm}{:else}{@render children()}{/if}
</button>

<style>
  .armed { background: var(--pill-negative-bg); border-color: color-mix(in srgb, var(--negative) 40%, transparent); color: var(--pill-negative-ink); }
</style>
