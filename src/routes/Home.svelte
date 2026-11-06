<script lang="ts">
  // Home answers one question: did my last dictation come out right?
  // The biggest block is that dictation as Heard / Written, with its fixes one click away.
  import { api, type HistoryItem, type ModelStatus, type Permissions, type Stats } from "../lib/ipc";
  import { app } from "../lib/state.svelte";
  import { clockTime, duration, groupedNumber, lastDays, niceCeiling, shortcutLabel, shortWhen } from "../lib/format";
  import Icon from "../components/Icon.svelte";
  import HeardWritten from "../components/HeardWritten.svelte";
  import RewriteMenu from "../components/RewriteMenu.svelte";
  import TeachWord from "../components/TeachWord.svelte";

  let stats = $state.raw<Stats | null>(null);
  let recent = $state.raw<HistoryItem[]>([]);
  let permissions = $state.raw<Permissions | null>(null);
  let models = $state.raw<ModelStatus[]>([]);
  let loaded = $state(false);
  let rewriting = $state(false);
  let now = $state(Date.now());

  $effect(() => {
    void app.dataVersion;
    Promise.all([api.stats().then((s) => (stats = s)), api.historyRecent(6).then((h) => (recent = h))])
      .catch(() => {})
      .finally(() => (loaded = true));
  });

  $effect(() => {
    api.permissions().then((p) => (permissions = p)).catch(() => {});
    api.models().then((m) => (models = m)).catch(() => {});
    const timer = setInterval(() => (now = Date.now()), 500);
    return () => clearInterval(timer);
  });

  const settings = $derived(app.settings);
  const key = (s: string | undefined) => (s ? shortcutLabel(s, app.platform) : "");
  const phase = $derived(app.status.phase);
  const recording = $derived(phase === "recording");
  const busy = $derived(["transcribing", "polishing", "delivering"].includes(phase));
  const elapsed = $derived(recording && app.status.started_at ? duration(now - app.status.started_at) : "");

  const modelReady = $derived(models.length === 0 || models.some((m) => m.id === settings?.model && m.installed));
  const accessOk = $derived(!permissions?.applicable || permissions.accessibility);
  const last = $derived(recent[0]);
  const earlier = $derived(recent.slice(1, 6));

  const styleNames: Record<string, string> = { messaging: "Message", email: "Email", code: "Code", docs: "Document", default: "" };

  // This week: words per day for the last 7 days, today's column in the data green.
  const week = $derived(stats ? lastDays(stats.activity, 7) : []);
  const weekWords = $derived(week.reduce((a, d) => a + d.words, 0));
  const weekTop = $derived(niceCeiling(Math.max(0, ...week.map((d) => d.words))));

  const finding = $derived.by(() => {
    if (!stats || stats.total_words === 0) return "Nothing dictated yet";
    const today = stats.words_today ? `${groupedNumber(stats.words_today)} words today` : "Nothing dictated today yet";
    return stats.current_streak > 1 ? `${today} · ${stats.current_streak}-day streak` : today;
  });

  // First run: the steps between installing Hams and the first dictation.
  const steps = $derived([
    { done: accessOk, label: "Allow Accessibility", detail: "So Hams can hear its shortcut and paste for you.", action: "Grant access", run: recheckPermissions },
    { done: modelReady, label: "Download the speech model", detail: "Speech is recognised on this computer.", action: "Open Speech settings", run: () => app.openSettings("speech") },
    { done: false, label: "Make your first dictation", detail: settings ? `Hold ${key(settings.dictate_shortcut)} in any app, speak, and let go.` : "", action: "", run: () => {} },
  ]);
  const stepsDone = $derived(steps.filter((s) => s.done).length);

  async function recheckPermissions() {
    await api.requestAccessibility().catch(() => {});
    setTimeout(async () => (permissions = await api.permissions()), 1500);
  }
</script>

<div class="page">
  <header class="title-row">
    <div>
      <h1>Today</h1>
      <p class="sub">{finding}</p>
    </div>
    {#if settings}
      <p class="hint">Hold <kbd>{key(settings.dictate_shortcut)}</kbd> anywhere to talk</p>
    {/if}
  </header>

  {#if app.view?.hotkey_error}
    <div class="notice bad">
      <span class="grow">{app.view.hotkey_error}</span>
      <button onclick={() => api.openPermissionSettings("accessibility")}>Open System Settings</button>
    </div>
  {:else if last && !accessOk}
    <div class="notice">
      <span class="grow">Hams needs Accessibility access again to hear its shortcut and paste for you.</span>
      <button onclick={recheckPermissions}>Grant access</button>
    </div>
  {/if}
  {#if last && !modelReady}
    <div class="notice">
      <span class="grow">The speech model isn't downloaded, so dictation can't start.</span>
      <button onclick={() => app.openSettings("speech")}>Download model</button>
    </div>
  {/if}

  <section class="panel lead" class:live={recording} aria-label="Last dictation" aria-live="polite">
    {#if recording || busy}
      <div class="lead-head">
        <h2>
          {#if recording}<span class="live-dot"></span>{app.status.kind === "note" ? "Recording a note" : app.status.kind === "command" ? "Listening for an instruction" : "Listening"}
          {:else}<span class="spinner"></span>Writing it down{/if}
        </h2>
        {#if recording}<span class="num muted">{elapsed}</span>{:else if app.status.app}<span class="muted">into {app.status.app}</span>{/if}
      </div>
      <p class="live-hint muted">
        {#if !recording}Almost there.
        {:else if app.status.kind === "note"}Finish it from Notes or the overlay.
        {:else if app.status.locked}Hands-free. Press Finish, or your shortcut again, to paste.
        {:else}Let go of <kbd>{key(settings?.dictate_shortcut)}</kbd> to paste. <kbd>Esc</kbd> cancels.{/if}
      </p>
      {#if recording && app.status.kind !== "note"}
        <div class="actions">
          <button class="primary" onclick={() => api.stop()}><Icon name="check" size={15} />Finish</button>
          <button onclick={() => api.cancel()}>Cancel</button>
        </div>
      {/if}
    {:else if last}
      <div class="lead-head">
        <h2>Last dictation</h2>
        <span class="meta muted">
          {#if last.app}<span>{last.app}</span>{/if}
          <span class="num">{clockTime(last.created_at)}</span>
          {#if styleNames[last.style]}<span>{styleNames[last.style]}</span>{/if}
          {#if last.delivery === "failed"}<span class="pill warn">Not pasted</span>{:else if last.delivery === "copied"}<span class="pill">Copied only</span>{/if}
        </span>
      </div>
      <div class:fading={rewriting}>
        <HeardWritten raw={last.raw} text={last.text} summary />
      </div>
      <div class="actions">
        <button onclick={() => app.run(() => api.copy(last.text), "Copied")}><Icon name="copy" size={15} />Copy</button>
        <RewriteMenu id={last.id} current={last.style} label onbusy={(b) => (rewriting = b)} />
        <TeachWord />
        <button class="ghost" onclick={() => (app.route = "history")}>Edit in History</button>
      </div>
      {#if phase === "error" && app.status.message}
        <p class="error">{app.status.message}</p>
      {/if}
    {:else if loaded}
      <div class="lead-head">
        <h2>Set up Hams</h2>
        <span class="muted num">{stepsDone} of {steps.length} done</span>
      </div>
      <div class="progress" aria-hidden="true"><span style:width="{(stepsDone / steps.length) * 100}%"></span></div>
      <ol class="steps">
        {#each steps as step, i (step.label)}
          <li class:done={step.done}>
            <span class="mark" aria-hidden="true">{#if step.done}<Icon name="check" size={13} />{:else}{i + 1}{/if}</span>
            <span class="grow">
              <strong>{step.label}</strong>
              <span class="muted">{step.done ? "Done" : step.detail}</span>
            </span>
            {#if !step.done && step.action}<button onclick={step.run}>{step.action}</button>{/if}
          </li>
        {/each}
      </ol>
    {/if}
  </section>

  {#if earlier.length}
    <section aria-labelledby="earlier-title">
      <div class="section-head">
        <h2 id="earlier-title">Earlier</h2>
        <button class="ghost small" onclick={() => (app.route = "history")}>View all <Icon name="chevron" size={14} /></button>
      </div>
      <ul class="panel flush rows earlier">
        {#each earlier as item (item.id)}
          <li>
            <span class="when num muted">{shortWhen(item.created_at, now)}</span>
            <span class="app muted">{item.app || "—"}</span>
            <span class="text">{item.text}</span>
            <button class="ghost small copy" onclick={() => app.run(() => api.copy(item.text), "Copied")} aria-label="Copy" title="Copy"><Icon name="copy" size={15} /></button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if stats && stats.total_words > 0}
    <section class="panel week" aria-labelledby="week-title">
      <div class="week-chart">
        <div class="section-head">
          <h2 id="week-title">This week</h2>
          <span class="aside">Words per day</span>
        </div>
        <div class="plot" role="img" aria-label="{groupedNumber(weekWords)} words in the last 7 days">
          <div class="grid" aria-hidden="true">
            {#each [weekTop, weekTop / 2, 0] as tick (tick)}
              <span class="tick"><span class="num">{groupedNumber(tick)}</span></span>
            {/each}
          </div>
          <div class="bars">
            {#each week as day (day.date.getTime())}
              <div class="day" title="{day.date.toLocaleDateString(undefined, { weekday: 'long', month: 'short', day: 'numeric' })}: {groupedNumber(day.words)} words">
                <span class="bar" class:today={day.isToday} style:height="{(day.words / weekTop) * 100}%"></span>
              </div>
            {/each}
          </div>
          <div class="labels" aria-hidden="true">
            {#each week as day (day.date.getTime())}
              <span class:today={day.isToday}>{day.isToday ? "Today" : day.date.toLocaleDateString(undefined, { weekday: "short" })}</span>
            {/each}
          </div>
        </div>
      </div>
      <dl class="figures">
        <div><dt>Last 7 days</dt><dd class="num">{groupedNumber(weekWords)}<small>&nbsp;words</small></dd></div>
        <div><dt>Speaking pace</dt><dd class="num">{#if stats.average_wpm}{stats.average_wpm}<small>&nbsp;wpm</small>{:else}<span class="dash">—</span>{/if}</dd></div>
        <div><dt>Streak</dt><dd class="num">{stats.current_streak}<small>&nbsp;{stats.current_streak === 1 ? "day" : "days"}</small></dd></div>
      </dl>
    </section>
  {/if}
</div>

<style>
  .hint { color: var(--muted); font-size: 13px; padding-top: 4px; white-space: nowrap; }
  .hint kbd { margin: 0 2px; }

  .lead { display: flex; flex-direction: column; gap: 16px; padding: 20px var(--pad) 18px; }
  .lead.live { border-color: color-mix(in srgb, var(--live) 35%, var(--border)); }
  .lead-head { display: flex; align-items: baseline; justify-content: space-between; gap: 16px; }
  .lead-head h2 { display: flex; align-items: center; gap: 8px; }
  .meta { display: flex; flex-wrap: wrap; align-items: center; justify-content: flex-end; gap: 0 4px; font-size: 13px; }
  .meta > span:not(.pill) + span:not(.pill)::before { content: "·"; margin-right: 4px; }
  .meta .pill { margin-left: 6px; }
  .actions { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
  .live-hint kbd { height: 20px; font-size: 12px; margin: 0 2px; }
  .fading { opacity: 0.45; transition: opacity 160ms; }
  .error { color: var(--negative); font-size: 13px; }

  .progress { height: 4px; border-radius: 999px; background: var(--rule); overflow: hidden; margin-top: -6px; }
  .progress span { display: block; height: 100%; background: var(--data); border-radius: inherit; }
  .steps { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; }
  .steps li { display: flex; align-items: center; gap: 12px; padding: 12px 0; border-top: 1px solid var(--rule); }
  .steps li:first-child { border-top: none; padding-top: 0; }
  .steps strong { display: block; font-weight: 500; }
  .steps .muted { font-size: 13px; }
  .steps .mark {
    width: 22px; height: 22px; border-radius: 50%; flex: none; display: grid; place-items: center;
    border: 1px solid var(--control); font-size: 12px; color: var(--muted);
  }
  .steps .done .mark { background: var(--data); border-color: var(--data); color: #fff; }
  .steps .done strong { color: var(--muted); font-weight: 400; }

  .earlier > li { display: grid; grid-template-columns: 72px 88px minmax(0, 1fr) 28px; gap: 12px; align-items: center; padding-top: 10px; padding-bottom: 10px; }
  .earlier .when { text-align: right; font-size: 13px; }
  .earlier .app { font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .earlier .text { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; user-select: text; }
  .earlier .copy { width: 28px; padding: 0; opacity: 0; }
  .earlier li:hover .copy, .earlier li:focus-within .copy { opacity: 1; }

  .week { display: grid; grid-template-columns: minmax(0, 1fr) 168px; gap: 28px; }
  .plot { display: grid; grid-template-columns: 40px minmax(0, 1fr); grid-template-rows: 96px auto; column-gap: 8px; }
  .grid { grid-row: 1; grid-column: 1 / -1; display: flex; flex-direction: column; justify-content: space-between; }
  .tick { display: flex; align-items: center; gap: 8px; height: 0; }
  .tick span { width: 40px; text-align: right; font-size: 12px; color: var(--muted); }
  .tick::after { content: ""; flex: 1; border-top: 1px solid var(--rule); }
  .tick:last-child::after { border-top-color: var(--border); }
  .bars { grid-row: 1; grid-column: 2; display: grid; grid-template-columns: repeat(7, 1fr); gap: 10px; align-items: end; padding-left: 8px; }
  .day { height: 100%; display: flex; align-items: flex-end; justify-content: center; }
  .bar { width: 100%; max-width: 28px; background: var(--data-3); border-radius: 3px 3px 0 0; min-height: 0; }
  .bar.today { background: var(--data); }
  .labels { grid-row: 2; grid-column: 2; display: grid; grid-template-columns: repeat(7, 1fr); gap: 10px; padding: 8px 0 0 8px; }
  .labels span { text-align: center; font-size: 12px; color: var(--muted); }
  .labels .today { color: var(--text); font-weight: 500; }
  .figures { margin: 0; display: flex; flex-direction: column; gap: 14px; justify-content: center; border-left: 1px solid var(--rule); padding-left: 24px; }
  dt { font-size: 12px; color: var(--muted); }
  dd { margin: 2px 0 0; font-size: 20px; letter-spacing: -0.01em; line-height: 1.2; }
  dd small { font-size: 13px; color: var(--muted); letter-spacing: 0; }
  .dash { color: var(--faint); }

  @media (max-width: 760px) {
    .week { grid-template-columns: 1fr; }
    .figures { flex-direction: row; border-left: none; padding-left: 0; gap: 28px; }
    .earlier > li { grid-template-columns: 64px minmax(0, 1fr) 28px; }
    .earlier .app { display: none; }
  }
</style>
