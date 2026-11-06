<script lang="ts">
  import { api, type HistoryItem, type ModelStatus, type Permissions, type Stats } from "../lib/ipc";
  import { app } from "../lib/state.svelte";
  import { calendarWeeks, duration, groupedNumber, relativeTime, shortcutLabel, timeParts } from "../lib/format";
  import Icon from "../components/Icon.svelte";

  const WEEKS = 17;

  let stats = $state<Stats | null>(null);
  let recent = $state<HistoryItem[]>([]);
  let permissions = $state<Permissions | null>(null);
  let models = $state<ModelStatus[]>([]);
  let loaded = $state(false);
  let now = $state(Date.now());

  $effect(() => {
    void app.dataVersion;
    Promise.all([
      api.stats().then((s) => (stats = s)),
      api.history("", 4).then((h) => (recent = h)),
    ])
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
  const modelReady = $derived(models.length === 0 || models.some((m) => m.id === settings?.model && m.installed));
  const phase = $derived(app.status.phase);
  const busy = $derived(["transcribing", "polishing", "delivering"].includes(phase));
  const recording = $derived(phase === "recording");
  const key = (s: string | undefined) => (s ? shortcutLabel(s, app.platform) : "");
  const elapsed = $derived(recording && app.status.started_at ? duration(now - app.status.started_at) : "");

  const weeks = $derived(stats ? calendarWeeks(stats.activity, WEEKS) : []);
  const days = $derived(weeks.flat().filter((d) => d.inRange));
  const activeDays = $derived(days.filter((d) => d.words > 0).length);
  const best = $derived(days.reduce<(typeof days)[number] | null>((b, d) => (d.words > (b?.words ?? 0) ? d : b), null));
  const months = $derived(
    weeks.map((week, i) => {
      // Label the first column and each column holding the 1st of a month.
      const first = week.find((d) => d.date.getDate() === 1);
      return (i === 0 && week[0].date.getDate() < 24) || first ? (first ?? week[0]).date.toLocaleDateString(undefined, { month: "short" }) : "";
    }),
  );

  const finding = $derived.by(() => {
    if (!stats) return "";
    if (stats.total_words === 0) return "Nothing dictated yet. Your first one takes a few seconds.";
    const today = stats.words_today ? `${groupedNumber(stats.words_today)} words today` : "Nothing dictated today yet";
    return stats.current_streak > 1 ? `${today} · ${stats.current_streak}-day streak` : today;
  });

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
    <div class="actions">
      <button onclick={() => (app.route = "notes")}><Icon name="notes" />New note</button>
    </div>
  </header>

  {#if app.view?.hotkey_error}
    <div class="notice bad">
      <span class="grow">{app.view.hotkey_error}</span>
      <button onclick={() => api.openPermissionSettings("accessibility")}>Open System Settings</button>
    </div>
  {:else if permissions?.applicable && !permissions.accessibility}
    <div class="notice">
      <span class="grow">Hams needs Accessibility access to hear its shortcut and paste for you.</span>
      <button onclick={recheckPermissions}>Grant access</button>
    </div>
  {/if}
  {#if !modelReady}
    <div class="notice">
      <span class="grow">The speech model isn't downloaded yet, so dictation can't start.</span>
      <button onclick={() => (app.route = "settings")}>Download model</button>
    </div>
  {/if}

  <section class="panel ready" class:recording aria-label="Dictation">
    <button
      class="mic"
      class:live={recording}
      onclick={() => (recording ? api.stop() : api.toggle())}
      disabled={busy || (recording && app.status.kind === "note")}
      aria-label={recording ? "Finish dictation" : "Start hands-free dictation"}
      title={recording ? "Finish" : "Start hands-free"}
    >
      <Icon name={recording ? "stop" : "mic"} size={20} />
    </button>
    <div class="grow">
      {#if recording}
        <p class="lead"><span class="live-dot"></span> {app.status.kind === "note" ? "Recording a note" : "Listening"} <span class="num muted">{elapsed}</span></p>
        <p class="muted small">{app.status.kind === "note" ? "Finish it from Notes or the overlay." : "Release the shortcut, or press the button, to paste. Esc cancels."}</p>
      {:else if busy}
        <p class="lead">Writing it down…</p>
        <p class="muted small">{app.status.app ? `Pasting into ${app.status.app}` : "Almost there"}</p>
      {:else if settings}
        <p class="lead">Hold <kbd>{key(settings.dictate_shortcut)}</kbd> in any app and speak. Let go to paste.</p>
        <p class="muted small">
          {#if settings.double_tap_lock}Double-tap for hands-free.{" "}{/if}Select text and hold <kbd>{key(settings.command_shortcut)}</kbd> to edit it by voice.
        </p>
      {/if}
      {#if phase === "error" && app.status.message}
        <p class="error small">{app.status.message}</p>
      {/if}
    </div>
  </section>

  {#if stats}
    <section aria-label="Totals">
      <dl class="panel flush kpis">
        <div><dt>Words today</dt><dd class="num">{groupedNumber(stats.words_today)}</dd></div>
        <div><dt>Words in total</dt><dd class="num">{groupedNumber(stats.total_words)}</dd></div>
        <div><dt>Speaking pace</dt><dd class="num">{#if stats.average_wpm}{stats.average_wpm}<small>&nbsp;wpm</small>{:else}<span class="dash">–</span>{/if}</dd></div>
        <div><dt>Time saved</dt><dd class="num">{#each timeParts(stats.minutes_saved) as [value, unit], i}{i ? " " : ""}{value}<small>&nbsp;{unit}</small>{/each}</dd></div>
        <div><dt>Dictations</dt><dd class="num">{groupedNumber(stats.total_dictations)}</dd></div>
      </dl>
    </section>

    <section class="panel activity" aria-labelledby="activity-title">
      <div class="section-head">
        <h2 id="activity-title">Words per day</h2>
        <span class="aside">Last {WEEKS} weeks</span>
      </div>
      <div class="activity-body">
        <div class="calendar-wrap">
          <div class="months" style="grid-template-columns: repeat({WEEKS}, 1fr)">
            {#each months as m}<span>{m}</span>{/each}
          </div>
          <div class="calendar" style="grid-template-columns: repeat({WEEKS}, 1fr)" role="img" aria-label="{activeDays} active days in the last {WEEKS} weeks">
            {#each weeks as week}
              <div class="week">
                {#each week as day}
                  <span
                    class="cell l{day.level}"
                    class:out={!day.inRange}
                    class:today={day.isToday}
                    title={day.inRange ? `${day.date.toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric" })}: ${groupedNumber(day.words)} words` : ""}
                  ></span>
                {/each}
              </div>
            {/each}
          </div>
          <div class="legend small muted" aria-hidden="true">
            Less
            {#each [0, 1, 2, 3, 4] as l}<span class="cell l{l}"></span>{/each}
            More
          </div>
        </div>
        <dl class="facts">
          <div><dt>Active days</dt><dd class="num">{activeDays}<small>&nbsp;of {days.length}</small></dd></div>
          <div><dt>Best day</dt><dd class="num">{#if best}{groupedNumber(best.words)}<small>&nbsp;words, {best.date.toLocaleDateString(undefined, { month: "short", day: "numeric" })}</small>{:else}<span class="dash">–</span>{/if}</dd></div>
          <div><dt>Current streak</dt><dd class="num">{stats.current_streak}<small>&nbsp;{stats.current_streak === 1 ? "day" : "days"}</small></dd></div>
          <div><dt>Longest streak</dt><dd class="num">{stats.longest_streak}<small>&nbsp;{stats.longest_streak === 1 ? "day" : "days"}</small></dd></div>
        </dl>
      </div>
    </section>
  {/if}

  <section aria-labelledby="recent-title">
    <div class="section-head">
      <h2 id="recent-title">Recent dictations</h2>
      {#if recent.length}<button class="ghost small" onclick={() => (app.route = "history")}>View all <Icon name="chevron" size={14} /></button>{/if}
    </div>
    {#if loaded && recent.length === 0}
      <p class="empty">Your dictations will show up here, newest first.</p>
    {:else}
      <ul class="panel flush rows recent">
        {#each recent as item (item.id)}
          <li>
            <p class="text clamp-2">{item.text}</p>
            <span class="meta small muted">{item.app || "Unknown app"}<br />{relativeTime(item.created_at)}</span>
            <button class="ghost small" onclick={() => app.run(() => api.copy(item.text), "Copied")} aria-label="Copy" title="Copy"><Icon name="copy" size={15} /></button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</div>

<style>
  .ready { display: flex; align-items: center; gap: 18px; padding: 18px var(--pad); background: var(--raised); border-color: transparent; }
  .ready.recording { background: color-mix(in srgb, var(--live) 7%, var(--surface)); }
  .mic { width: 48px; height: 48px; border-radius: 50%; padding: 0; flex: none; background: var(--primary); border: none; color: var(--primary-ink); }
  .mic:hover:not(:disabled) { background: color-mix(in srgb, var(--primary) 86%, var(--ground)); }
  .mic.live { background: var(--live); color: #fff; }
  .mic :global(.icon) { stroke-width: 1.8; }
  .lead { font-size: 15px; font-weight: 500; display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin-bottom: 4px; }
  .ready .small kbd { height: 20px; font-size: 11px; }
  .error { color: var(--negative); margin-top: 6px; }

  .kpis { display: grid; grid-template-columns: repeat(5, 1fr); margin: 0; }
  .kpis > div { padding: 14px 18px 16px; border-left: 1px solid var(--rule); }
  .kpis > div:first-child { border-left: none; }
  dt { font-size: 12.5px; color: var(--muted); }
  dd { margin: 4px 0 0; font-size: var(--fs-figure); font-weight: 400; letter-spacing: -0.01em; line-height: 1.2; }
  dd small { font-size: 13px; color: var(--muted); letter-spacing: 0; }
  .dash { color: var(--faint); }

  .activity-body { display: grid; grid-template-columns: minmax(0, 1fr) 200px; gap: 28px; align-items: start; }
  .months, .calendar { display: grid; gap: 4px; }
  .months span { font-size: 11px; color: var(--muted); height: 16px; white-space: nowrap; }
  .week { display: grid; grid-template-rows: repeat(7, 1fr); gap: 4px; }
  .cell { display: block; aspect-ratio: 1; border-radius: 3px; background: var(--rule); }
  .cell.l1 { background: var(--data-1); }
  .cell.l2 { background: var(--data-2); }
  .cell.l3 { background: var(--data-3); }
  .cell.l4 { background: var(--data-4); }
  .cell.out { background: transparent; }
  .cell.today { outline: 1.5px solid var(--text); outline-offset: 1px; }
  .legend { display: flex; align-items: center; gap: 4px; justify-content: flex-end; margin-top: 10px; }
  .legend .cell { width: 11px; }
  .facts { margin: 0; display: flex; flex-direction: column; gap: 14px; padding-top: 18px; }
  .facts dd { font-size: 18px; }

  .recent > li { display: grid; grid-template-columns: 1fr 120px 28px; gap: 16px; align-items: center; }
  .recent .text { user-select: text; }
  .recent .meta { text-align: right; line-height: 1.4; }

  @media (max-width: 900px) {
    .kpis { grid-template-columns: repeat(3, 1fr); }
    .kpis > div:nth-child(4) { border-left: none; }
    .kpis > div:nth-child(n + 4) { border-top: 1px solid var(--rule); }
    .activity-body { grid-template-columns: 1fr; }
    .facts { flex-direction: row; flex-wrap: wrap; padding-top: 0; gap: 14px 28px; }
  }
</style>
