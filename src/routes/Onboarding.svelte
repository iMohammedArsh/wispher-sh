<script lang="ts">
  import { api, errorText, on, type HistoryItem, type ModelStatus, type Permissions } from "../lib/ipc";
  import { app } from "../lib/state.svelte";
  import { shortcutLabel } from "../lib/format";
  import HeardWritten from "../components/HeardWritten.svelte";

  let step = $state(0);
  let permissions = $state<Permissions | null>(null);
  let models = $state<ModelStatus[]>([]);
  let progress = $state<number | null>(null);

  const s = $derived(app.settings);
  const steps = $derived(permissions?.applicable === false ? ["Welcome", "Model", "Try it"] : ["Welcome", "Permissions", "Model", "Try it"]);
  const current = $derived(steps[step]);
  const chosen = $derived(models.find((m) => m.id === s?.model));

  // The test dictation: the first one made after reaching the last step.
  let tryFrom = 0;
  let attempt = $state.raw<HistoryItem | null>(null);
  $effect(() => {
    if (current !== "Try it") return;
    if (!tryFrom) tryFrom = Date.now();
    void app.dataVersion;
    void app.status.phase;
    api.historyRecent(1).then(([item]) => { if (item && item.created_at >= tryFrom - 1000) attempt = item; }).catch(() => {});
  });

  $effect(() => {
    api.models().then((m) => (models = m)).catch(() => {});
    const off = on<{ id: string; progress: number }>("model-progress", (p) => (progress = p.progress));
    const poll = setInterval(() => api.permissions().then((p) => (permissions = p)).catch(() => {}), 1500);
    api.permissions().then((p) => (permissions = p)).catch(() => {});
    return () => {
      clearInterval(poll);
      void off.then((f) => f());
    };
  });

  async function download() {
    if (!s) return;
    progress = 0;
    try {
      await api.downloadModel(s.model);
      models = await api.models();
    } catch (error) {
      app.notify(errorText(error), "error");
    } finally {
      progress = null;
    }
  }

  async function finish() {
    await app.save({ onboarding_done: true });
    app.route = "home";
  }
</script>

<div class="wrap">
  <ol class="steps" aria-label="Setup steps">
    {#each steps as name, i}
      <li class:done={i < step} class:active={i === step} aria-current={i === step ? "step" : undefined}>{name}</li>
    {/each}
  </ol>

  <section class="panel stack">
    {#if current === "Welcome"}
      <h1>Write at the speed you talk</h1>
      <p>Hold a shortcut in any app, talk naturally, let go. Hams writes clean text right where your cursor is.</p>
      <div class="example">
        <HeardWritten raw="um so can we uh move the review to thursday" text="So, can we move the review to Thursday?" />
      </div>
      <p class="small muted">Speech is recognised on this computer, so it works offline and stays private. Optional AI polish adapts the writing to Slack, email, code and more.</p>
      <div class="actions"><button class="primary" onclick={() => step++}>Get started</button></div>
    {:else if current === "Permissions"}
      <h1>Two quick permissions</h1>
      <div class="perm">
        <div class="grow">
          <strong>Accessibility</strong>
          <p class="small muted">Lets Hams see its shortcut and paste text for you.</p>
        </div>
        {#if permissions?.accessibility}
          <span class="pill ok">Allowed</span>
        {:else}
          <button class="primary" onclick={() => api.requestAccessibility()}>Allow</button>
        {/if}
      </div>
      <div class="perm">
        <div class="grow">
          <strong>Microphone</strong>
          <p class="small muted">macOS asks the first time you dictate.</p>
        </div>
        <span class="pill">On first use</span>
      </div>
      {#if permissions && !permissions.accessibility}
        <p class="small muted">After allowing Hams in System Settings you may need to quit and reopen Hams.</p>
      {/if}
      <div class="actions">
        <button class="ghost" onclick={() => step++}>Skip for now</button>
        <button class="primary" disabled={!permissions?.accessibility} onclick={() => step++}>Continue</button>
      </div>
    {:else if current === "Model"}
      <h1>Download the speech model</h1>
      <p class="muted">A one-time download. Pick a different size any time in Settings.</p>
      {#if chosen}
        <div class="perm">
          <div class="grow">
            <strong>{chosen.label}</strong> <span class="small muted">· {chosen.size_mb} MB</span>
            <p class="small muted">{chosen.note}</p>
          </div>
          {#if chosen.installed}
            <span class="pill ok">Ready</span>
          {:else if progress !== null}
            <progress max="1" value={progress}></progress>
          {:else}
            <button class="primary" onclick={download}>Download</button>
          {/if}
        </div>
      {/if}
      <div class="actions">
        <button class="ghost" onclick={() => step--}>Back</button>
        <button class="primary" disabled={!chosen?.installed} onclick={() => step++}>Continue</button>
      </div>
    {:else if s}
      <h1>Try it</h1>
      <p>Hold <kbd>{shortcutLabel(s.dictate_shortcut, app.platform)}</kbd>, say a sentence, and let go.</p>
      <div class="try" class:live={app.status.phase === "recording"} class:result={attempt && app.status.phase === "idle"} aria-live="polite">
        {#if app.status.phase === "recording"}
          <span class="listening"><span class="live-dot"></span>Listening…</span>
        {:else if ["transcribing", "polishing", "delivering"].includes(app.status.phase)}
          <span class="muted">Writing…</span>
        {:else if app.status.phase === "error"}
          <span class="error">{app.status.message}</span>
        {:else if attempt}
          <HeardWritten raw={attempt.raw} text={attempt.text} summary />
        {:else}
          <span class="muted">What Hams heard, and what it wrote, will appear here.</span>
        {/if}
      </div>
      <p class="small muted">In other apps the text is pasted at your cursor. Here it's just shown.</p>
      <div class="actions">
        <button class="ghost" onclick={() => step--}>Back</button>
        <button class="primary" onclick={finish}>Finish</button>
      </div>
    {/if}
  </section>
</div>

<style>
  .wrap { max-width: 560px; margin: 8px auto 0; display: flex; flex-direction: column; gap: 18px; }
  .steps { display: grid; grid-template-columns: repeat(auto-fit, minmax(0, 1fr)); gap: 6px; list-style: none; padding: 0; margin: 0; }
  .steps li { font-size: 12px; color: var(--muted); padding-top: 8px; position: relative; }
  .steps li::before { content: ""; position: absolute; inset: 0 0 auto; height: 3px; border-radius: 2px; background: var(--rule); }
  .steps li.done::before, .steps li.active::before { background: var(--data); }
  .steps li.active { color: var(--text); font-weight: 500; }
  .panel { padding: 28px; gap: 16px; }
  .panel :global(h1) { font-size: 22px; }
  .perm { display: flex; align-items: center; gap: 12px; padding: 12px 14px; background: var(--raised); border-radius: var(--radius-card); }
  .actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 8px; }
  .try { min-height: 84px; border: 1px dashed var(--control); border-radius: var(--radius-card); padding: 16px; display: grid; place-items: center; text-align: center; font-size: 16px; }
  .try.live { border-color: var(--live); }
  .try.result { display: block; text-align: left; border-style: solid; border-color: var(--border); }
  .listening { display: inline-flex; align-items: center; gap: 8px; }
  .example { padding: 16px; background: var(--raised); border-radius: var(--radius-card); }
  .error { color: var(--negative); }
  progress { width: 140px; }
</style>
