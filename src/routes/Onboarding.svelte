<script lang="ts">
  import { api, errorText, on, type HistoryItem, type ModelStatus } from "../lib/ipc";
  import { app } from "../lib/state.svelte";
  import { shortcutLabel } from "../lib/format";
  import HeardWritten from "../components/HeardWritten.svelte";

  let step = $state(0);
  let models = $state<ModelStatus[]>([]);
  /** Download progress by model id, 0..1. */
  let progress = $state<Record<string, number>>({});
  let downloading = $state<string | null>(null);
  let downloadError = $state<string | null>(null);
  /** The size the model note recommends: handles accents and mixed speech. */
  const RECOMMENDED = "small";

  const s = $derived(app.settings);
  const permissions = $derived(app.permissions);
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
    const off = on<{ id: string; progress: number }>("model-progress", (p) => (progress = { ...progress, [p.id]: p.progress }));
    // Granting a permission happens in System Settings: keep looking, and the
    // store re-arms the shortcut the moment Accessibility is allowed.
    const poll = setInterval(() => void app.refreshPermissions(), 1500);
    void app.refreshPermissions();
    return () => {
      clearInterval(poll);
      void off.then((f) => f());
    };
  });

  async function download() {
    if (!s) return;
    const id = s.model;
    downloading = id;
    downloadError = null;
    progress = { ...progress, [id]: progress[id] ?? 0 };
    try {
      await api.downloadModel(id);
      models = await api.models();
    } catch (error) {
      downloadError = errorText(error);
    } finally {
      downloading = null;
    }
  }

  const micOk = $derived(permissions?.microphone === "granted" || permissions?.microphone === "unknown");
  const shortcutOk = $derived(permissions?.hotkeys === "active");

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
        {#if permissions?.accessibility && shortcutOk}
          <span class="pill ok">Allowed</span>
        {:else if permissions?.accessibility && app.needsRestart}
          <button class="primary" onclick={() => api.restartApp()}>Restart Hams</button>
        {:else if permissions?.accessibility}
          <span class="pill">Starting the shortcut…</span>
        {:else}
          <button class="primary" onclick={() => api.requestAccessibility()}>Allow</button>
        {/if}
      </div>
      <div class="perm">
        <div class="grow">
          <strong>Microphone</strong>
          <p class="small muted">
            {#if permissions?.microphone === "denied"}Microphone access is off for Hams. Turn it on in System Settings → Privacy &amp; Security → Microphone.
            {:else}Hams only listens while you hold the shortcut.{/if}
          </p>
        </div>
        {#if permissions?.microphone === "granted"}
          <span class="pill ok">Allowed</span>
        {:else if permissions?.microphone === "denied"}
          <button class="primary" onclick={() => api.openPermissionSettings("microphone")}>Open System Settings</button>
        {:else if permissions?.microphone === "undetermined"}
          <button class="primary" onclick={() => api.requestMicrophone()}>Allow</button>
        {:else}
          <span class="pill">On first use</span>
        {/if}
      </div>
      {#if app.needsRestart}
        <p class="small muted">macOS hasn't applied the Accessibility permission to the running Hams yet. Restarting fixes that.</p>
      {:else if permissions && !permissions.accessibility}
        <p class="small muted">Switch Hams on in System Settings → Privacy &amp; Security → Accessibility. This screen updates by itself.</p>
      {/if}
      <div class="actions">
        <button class="ghost" onclick={() => step++}>Skip for now</button>
        <button class="primary" disabled={!permissions?.accessibility || !micOk} onclick={() => step++}>Continue</button>
      </div>
    {:else if current === "Model"}
      <h1>Download the speech model</h1>
      <p class="muted">A one-time download. After it, speech is recognised on this computer, with no internet needed. You can change this in Settings.</p>
      <div class="stack" role="radiogroup" aria-label="Speech model">
        {#each models as m (m.id)}
          <label class="perm pick" class:picked={m.id === s?.model}>
            <input type="radio" name="model" checked={m.id === s?.model} disabled={downloading !== null} onchange={() => app.save({ model: m.id })} />
            <span class="grow">
              <strong>{m.label}</strong> <span class="small muted">· {m.size_mb} MB{m.id === RECOMMENDED ? " · Recommended" : ""}</span>
              <span class="small muted block">{m.note}</span>
            </span>
            {#if m.installed}<span class="pill ok">Ready</span>{/if}
          </label>
        {/each}
      </div>
      {#if chosen && !chosen.installed}
        <div class="download">
          {#if downloading}
            <progress max="1" value={progress[downloading] ?? 0}></progress>
            <span class="small muted num">{Math.round((progress[downloading] ?? 0) * 100)}% of {chosen.size_mb} MB</span>
            <button onclick={() => api.cancelModelDownload(downloading!)}>Cancel</button>
          {:else}
            <button class="primary" onclick={download}>{downloadError ? "Try again" : `Download ${chosen.size_mb} MB`}</button>
            {#if downloadError}
              <span class="small error">{downloadError} Hams continues where it stopped.</span>
            {:else if progress[chosen.id]}
              <span class="small muted">Stopped at {Math.round(progress[chosen.id] * 100)}%. Hams continues where it stopped.</span>
            {/if}
          {/if}
        </div>
      {/if}
      <div class="actions">
        <button class="ghost" disabled={downloading !== null} onclick={() => step--}>Back</button>
        <button class="ghost" disabled={downloading !== null} onclick={() => step++}>Skip for now</button>
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
      {#if chosen && !chosen.installed}
        <div class="notice">
          <span class="grow">The speech model isn't downloaded yet, so there's nothing to hear you with.</span>
          <button onclick={() => (step = steps.indexOf("Model"))}>Download it</button>
        </div>
      {/if}
      {#if permissions?.applicable && !shortcutOk}
        <div class="notice bad">
          <span class="grow">The shortcut isn't running yet{permissions.accessibility ? "." : ", because Hams doesn't have Accessibility access."}</span>
          {#if app.needsRestart}
            <button onclick={() => api.restartApp()}>Restart Hams</button>
          {:else if !permissions.accessibility}
            <button onclick={() => api.openPermissionSettings("accessibility")}>Open System Settings</button>
          {/if}
        </div>
      {/if}
      <div class="talk">
        {#if app.status.phase === "recording"}
          <button class="primary" onclick={() => api.stop()}>Finish</button>
          <button onclick={() => api.cancel()}>Cancel</button>
        {:else}
          <button onclick={() => api.toggle()} disabled={app.status.phase !== "idle" && app.status.phase !== "done" && app.status.phase !== "error"}>Or click here to talk</button>
          <span class="small muted">Handy if the shortcut isn't working yet.</span>
        {/if}
      </div>
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
  .talk { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  progress { flex: 1; min-width: 120px; max-width: 240px; }
  .pick { cursor: pointer; border: 1px solid transparent; }
  .pick.picked { border-color: var(--control); background: var(--surface); }
  .pick input { accent-color: var(--primary); margin: 0; }
  .download { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .block { display: block; }
</style>
