<script lang="ts">
  // The always-on-top pill. Deliberately tiny: no shared CSS, no router, no fonts.
  // One size for every phase (set in overlay.rs), so it never jumps while you talk.
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import type { Status } from "../lib/ipc";

  const BARS = 11;
  let status = $state<Status | null>(null);
  /** The last few mic levels, oldest first: the bars scroll like a waveform. */
  let levels = $state<number[]>(Array(BARS).fill(0));
  let partial = $state("");
  let now = $state(Date.now());

  onMount(() => {
    const unlisten = [
      listen<Status>("status", (e) => {
        if (e.payload.phase !== "recording") partial = "";
        if (e.payload.phase === "recording" && status?.phase !== "recording") levels = Array(BARS).fill(0);
        status = e.payload;
      }),
      listen<string>("partial", (e) => (partial = e.payload)),
      listen<number>("level", (e) => (levels = [...levels.slice(1), e.payload])),
    ];
    invoke<Status>("get_status").then((s) => (status = s)).catch(() => {});
    const timer = setInterval(() => (now = Date.now()), 250);
    return () => {
      clearInterval(timer);
      unlisten.forEach((p) => p.then((f) => f()));
    };
  });

  const phase = $derived(status?.phase ?? "idle");
  const working = $derived(phase === "transcribing" || phase === "polishing" || phase === "delivering");
  const note = $derived(status?.kind === "note");
  const command = $derived(status?.kind === "command");
  const elapsed = $derived(status?.started_at ? Math.max(0, Math.floor((now - status.started_at) / 1000)) : 0);
  const clock = $derived(`${Math.floor(elapsed / 60)}:${String(elapsed % 60).padStart(2, "0")}`);
  const label = $derived(
    note && phase === "transcribing" ? "Finishing note" : note && phase === "polishing" ? "Summarising" :
    phase === "transcribing" ? "Transcribing" : phase === "polishing" ? (command ? "Editing" : "Polishing") : phase === "delivering" ? "Pasting" : "",
  );
</script>

{#if phase === "recording" && status?.preview}
  <div class="caption" aria-live="polite">
    {#if partial}<p>{partial}</p>{:else}<p class="hint">Listening…</p>{/if}
  </div>
{/if}

<div class="pill {phase}" class:below={phase === "recording" && status?.preview}>
  {#if phase === "recording"}
    {#if note}
      <!-- A long note is never one click from being thrown away; discard it from the Notes screen. -->
      <span class="tag">Note</span>
    {:else}
      <button class="icon" title="Cancel (Esc)" aria-label="Cancel" onclick={() => invoke("dictation_cancel")}>
        <svg viewBox="0 0 16 16"><path d="M4.5 4.5l7 7M11.5 4.5l-7 7" /></svg>
      </button>
      {#if command}<span class="tag">Edit</span>{:else}<span class="dot" aria-hidden="true"></span>{/if}
    {/if}
    <div class="bars" aria-hidden="true">
      {#each levels as level, i (i)}
        <span style="transform: scaleY({Math.max(0.1, Math.min(1, level * 1.3))})"></span>
      {/each}
    </div>
    <span class="time">{status?.locked ? "∞ " : ""}{clock}</span>
    <button class="icon finish" title="Finish" aria-label="Finish" onclick={() => invoke("dictation_stop")}>
      <svg viewBox="0 0 16 16"><path d="M3.5 8.5l3 3 6-7" /></svg>
    </button>
  {:else if working}
    <div class="bars thinking" aria-hidden="true">
      {#each Array(5) as _, i (i)}<span style="animation-delay: {i * 110}ms"></span>{/each}
    </div>
    <span class="word">{label}</span>
  {:else if phase === "done"}
    <svg class="tick" viewBox="0 0 16 16"><path d="M3.5 8.5l3 3 6-7" /></svg>
    <span class="word">{note ? "Note saved" : status?.app ? `Pasted into ${status.app}` : "Done"}</span>
  {:else if phase === "error"}
    <span class="dot bad" aria-hidden="true"></span>
    <span class="message">{status?.message ?? "Something went wrong"}</span>
  {/if}
</div>

<style>
  :global(html), :global(body) { margin: 0; background: transparent; font: 500 13px -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif; }
  .pill {
    position: fixed; inset: 4px;
    display: flex; align-items: center; justify-content: center; gap: 10px;
    padding: 0 7px; border-radius: 999px;
    background: rgba(20, 22, 20, 0.94); color: #f2f4f2;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.26), inset 0 0 0 1px rgba(255, 255, 255, 0.08);
    -webkit-user-select: none; user-select: none; cursor: default;
  }
  .pill.idle { display: none; }
  .pill.below { inset: auto; bottom: 4px; left: 50%; transform: translateX(-50%); width: 224px; height: 40px; }
  .caption {
    position: fixed; left: 4px; right: 4px; top: 4px; bottom: 52px;
    display: flex; flex-direction: column; justify-content: flex-end;
    padding: 8px 14px; border-radius: 14px; overflow: hidden;
    background: rgba(20, 22, 20, 0.9); color: #f2f4f2;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.22), inset 0 0 0 1px rgba(255, 255, 255, 0.08);
    -webkit-user-select: none; user-select: none;
  }
  .caption p { margin: 0; font-size: 13px; line-height: 1.35; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .caption .hint { opacity: 0.55; }
  .pill.error { border-radius: 16px; padding: 0 16px; justify-content: flex-start; }

  .icon {
    width: 26px; height: 26px; border-radius: 50%; border: none; padding: 0; flex: none;
    background: rgba(255, 255, 255, 0.1); color: #f2f4f2; display: grid; place-items: center; cursor: pointer;
  }
  .icon:hover { background: rgba(255, 255, 255, 0.2); }
  .icon.finish { background: #f2f4f2; color: #141614; }
  .icon.finish:hover { background: #fff; }
  .icon svg, .tick { width: 14px; height: 14px; fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
  .tick { color: #5fcf96; }
  .dot { width: 7px; height: 7px; border-radius: 50%; background: #ff5a5f; flex: none; animation: pulse 1.2s ease-in-out infinite; }
  .dot.bad { animation: none; }
  .tag { font-size: 12px; padding: 2px 8px; border-radius: 999px; background: rgba(255, 255, 255, 0.12); flex: none; }

  .bars { display: flex; align-items: center; gap: 3px; height: 22px; }
  .bars span { width: 3px; height: 22px; border-radius: 2px; background: #f2f4f2; transform-origin: center; transition: transform 90ms linear; }
  .bars.thinking span { height: 14px; opacity: 0.7; animation: think 900ms ease-in-out infinite; transform: scaleY(0.3); }
  .time { font-variant-numeric: tabular-nums; font-size: 12px; opacity: 0.75; min-width: 32px; }
  .word { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 160px; }
  .message { font-size: 12px; line-height: 1.35; overflow: hidden; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; }

  @keyframes pulse { 50% { opacity: 0.35; } }
  @keyframes think { 50% { transform: scaleY(1); opacity: 1; } }
  @media (prefers-reduced-motion: reduce) { .dot, .bars.thinking span { animation: none; } }
</style>
