<script lang="ts">
  // The always-on-top pill. Deliberately tiny: no shared CSS, no router, no fonts.
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import type { Status } from "../lib/ipc";

  let status = $state<Status | null>(null);
  let level = $state(0);
  let partial = $state("");
  let now = $state(Date.now());
  const BARS = 9;
  const shape = Array.from({ length: BARS }, (_, i) => 0.45 + 0.55 * Math.sin(((i + 0.5) / BARS) * Math.PI));

  onMount(() => {
    const unlisten = [
      listen<Status>("status", (e) => {
        if (e.payload.phase !== "recording") partial = "";
        status = e.payload;
      }),
      listen<string>("partial", (e) => (partial = e.payload)),
      listen<number>("level", (e) => (level = e.payload)),
    ];
    invoke<Status>("get_status").then((s) => (status = s)).catch(() => {});
    const timer = setInterval(() => (now = Date.now()), 250);
    return () => {
      clearInterval(timer);
      unlisten.forEach((p) => p.then((f) => f()));
    };
  });

  const phase = $derived(status?.phase ?? "idle");
  const elapsed = $derived(status?.started_at ? Math.max(0, Math.floor((now - status.started_at) / 1000)) : 0);
  const clock = $derived(`${Math.floor(elapsed / 60)}:${String(elapsed % 60).padStart(2, "0")}`);
  const label = $derived(
    phase === "transcribing" ? "Transcribing" : phase === "polishing" ? (status?.kind === "command" ? "Working" : "Polishing") : phase === "delivering" ? "Pasting" : "",
  );
</script>

{#if phase === "recording" && status?.preview}
  <div class="caption" aria-live="polite">
    {#if partial}<p>{partial}</p>{:else}<p class="hint">Listening…</p>{/if}
  </div>
{/if}

<div class="pill {phase}" class:command={status?.kind === "command"} class:below={phase === "recording" && status?.preview}>
  {#if phase === "recording"}
    <button class="icon" title="Cancel (Esc)" aria-label="Cancel" onclick={() => invoke("dictation_cancel")}>
      <svg viewBox="0 0 16 16"><path d="M4 4l8 8M12 4l-8 8" /></svg>
    </button>
    <div class="bars" aria-hidden="true">
      {#each shape as s}
        <span style="transform: scaleY({Math.max(0.12, Math.min(1, level * s * 1.25))})"></span>
      {/each}
    </div>
    <span class="time">{status?.locked ? "∞ " : ""}{clock}</span>
    <button class="icon finish" title="Finish" aria-label="Finish" onclick={() => invoke("dictation_stop")}>
      <svg viewBox="0 0 16 16"><path d="M3.5 8.5l3 3 6-7" /></svg>
    </button>
  {:else if phase === "done"}
    <svg class="tick" viewBox="0 0 16 16"><path d="M3.5 8.5l3 3 6-7" /></svg>
    <span>Done</span>
  {:else if phase === "error"}
    <span class="message">{status?.message ?? "Something went wrong"}</span>
  {:else if phase !== "idle"}
    <span class="spinner" aria-hidden="true"></span>
    <span>{label}</span>
  {/if}
</div>

<style>
  :global(html), :global(body) { margin: 0; background: transparent; font: 600 13px -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif; }
  .pill {
    position: fixed; inset: 4px;
    display: flex; align-items: center; justify-content: center; gap: 10px;
    padding: 0 8px; border-radius: 999px;
    background: rgba(18, 18, 18, 0.92); color: #f3f3f3;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.28), inset 0 0 0 1px rgba(255, 255, 255, 0.08);
    -webkit-user-select: none; user-select: none; cursor: default;
  }
  .pill.idle { display: none; }
  .pill.below { inset: auto; bottom: 4px; left: 50%; transform: translateX(-50%); width: 204px; height: 40px; }
  .caption {
    position: fixed; left: 4px; right: 4px; top: 4px; bottom: 52px;
    display: flex; flex-direction: column; justify-content: flex-end;
    padding: 8px 14px; border-radius: 14px; overflow: hidden;
    background: rgba(18, 18, 18, 0.86); color: #f3f3f3;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.22), inset 0 0 0 1px rgba(255, 255, 255, 0.08);
    -webkit-user-select: none; user-select: none;
  }
  .caption p { margin: 0; font-weight: 500; font-size: 13px; line-height: 1.35; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .caption .hint { opacity: 0.55; }
  .pill.error { border-radius: 14px; padding: 0 16px; background: rgba(120, 22, 18, 0.95); }
  .pill.command .bars span { background: #9ec5ff; }
  .icon {
    width: 26px; height: 26px; border-radius: 50%; border: none; padding: 0;
    background: rgba(255, 255, 255, 0.12); color: #fff; display: grid; place-items: center; cursor: pointer;
  }
  .icon:hover { background: rgba(255, 255, 255, 0.22); }
  .icon.finish { background: #e5484d; }
  .icon svg, .tick { width: 14px; height: 14px; fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
  .tick { color: #6fcf97; }
  .bars { display: flex; align-items: center; gap: 3px; height: 24px; }
  .bars span { width: 3px; height: 24px; border-radius: 2px; background: #fff; transform-origin: center; transition: transform 70ms linear; }
  .time { font-variant-numeric: tabular-nums; font-size: 12px; opacity: 0.8; min-width: 34px; }
  .message { font-weight: 500; font-size: 12.5px; line-height: 1.3; overflow: hidden; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; }
  .spinner { width: 14px; height: 14px; border-radius: 50%; border: 2px solid rgba(255, 255, 255, 0.25); border-top-color: #fff; animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
