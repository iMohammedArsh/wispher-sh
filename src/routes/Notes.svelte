<script lang="ts">
  import { api, errorText, on, type Note } from "../lib/ipc";
  import { app } from "../lib/state.svelte";
  import { clockTime, dayLabel, duration, groupedNumber, lengthLabel } from "../lib/format";
  import Icon from "../components/Icon.svelte";
  import ConfirmButton from "../components/ConfirmButton.svelte";

  let notes = $state<Note[]>([]);
  let query = $state("");
  let selectedId = $state<number | null>(null);
  let live = $state("");
  let loaded = $state(false);
  let now = $state(Date.now());

  async function load() {
    try {
      notes = await api.notes(query);
      if (selectedId === null || !notes.some((n) => n.id === selectedId)) selectedId = notes[0]?.id ?? null;
    } catch (error) {
      app.notify(errorText(error), "error");
    } finally {
      loaded = true;
    }
  }

  $effect(() => {
    void app.dataVersion;
    void query;
    const timer = setTimeout(load, 120);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    const off = on<string>("note-progress", (text) => (live = text));
    const tick = setInterval(() => (now = Date.now()), 500);
    return () => {
      clearInterval(tick);
      void off.then((f) => f());
    };
  });

  const status = $derived(app.status);
  const noteActive = $derived(status.kind === "note" && ["recording", "transcribing", "polishing"].includes(status.phase));
  const noteError = $derived(status.kind === "note" && status.phase === "error" ? status.message : null);
  const recordingNote = $derived(noteActive && status.phase === "recording");
  const dictating = $derived(status.kind !== "note" && status.phase !== "idle" && status.phase !== "done" && status.phase !== "error");
  const selected = $derived(notes.find((n) => n.id === selectedId) ?? null);
  const totalMs = $derived(notes.reduce((a, n) => a + n.duration_ms, 0));

  // When a note finishes, jump to it.
  let wasActive = false;
  $effect(() => {
    if (wasActive && !noteActive) {
      selectedId = null;
      live = "";
      void load();
    }
    wasActive = noteActive;
  });

  function start() {
    live = "";
    void app.run(api.noteToggle);
  }

  type Block = { kind: "point" | "heading" | "text"; text: string };
  function summaryBlocks(summary: string): Block[] {
    return summary
      .split("\n")
      .map((l) => l.trim())
      .filter(Boolean)
      .map((line): Block => {
        if (/^[-•*]\s+/.test(line)) return { kind: "point", text: line.replace(/^[-•*]\s+/, "") };
        if (/^action items:?$/i.test(line)) return { kind: "heading", text: "Action items" };
        return { kind: "text", text: line };
      });
  }

  function groups(blocks: Block[]): { heading: string | null; items: Block[] }[] {
    const out: { heading: string | null; items: Block[] }[] = [{ heading: null, items: [] }];
    for (const b of blocks) {
      if (b.kind === "heading") out.push({ heading: b.text, items: [] });
      else out[out.length - 1].items.push(b);
    }
    return out.filter((g) => g.items.length);
  }

  function fullText(note: Note): string {
    return [note.title, note.summary, note.transcript].filter((t) => t.trim()).join("\n\n");
  }

  async function rename(note: Note, title: string) {
    const clean = title.trim();
    if (!clean || clean === note.title) return;
    await app.run(() => api.noteRename(note.id, clean));
    await load();
  }

  async function remove(note: Note) {
    await app.run(() => api.noteDelete(note.id), "Note deleted");
    selectedId = null;
    await load();
  }
</script>

<div class="page wide">
  <header class="title-row">
    <div>
      <h1>Notes</h1>
      <p class="sub">
        {#if notes.length}{notes.length} {notes.length === 1 ? "note" : "notes"} · {lengthLabel(totalMs)} recorded{:else}Lectures, meetings and thinking out loud, transcribed on this Mac{/if}
      </p>
    </div>
    <div class="actions">
      {#if recordingNote}
        <ConfirmButton onconfirm={() => app.run(api.cancel, "Note discarded")} confirm="Discard the recording?">Discard</ConfirmButton>
        <button class="primary" onclick={() => app.run(api.noteToggle)}><Icon name="check" />Finish note</button>
      {:else}
        <button class="primary" onclick={start} disabled={dictating || noteActive}><Icon name="mic" />New voice note</button>
      {/if}
    </div>
  </header>

  {#if noteError}
    <div class="notice bad"><span class="grow">{noteError}</span></div>
  {/if}

  <div class="layout">
    <aside class="list" aria-label="Your notes">
      <label class="search">
        <Icon name="search" size={14} />
        <input type="search" placeholder="Search notes" bind:value={query} aria-label="Search notes" />
      </label>
      {#if loaded && notes.length === 0}
        <p class="empty small">{query ? "No note matches that search." : "Notes you record appear here."}</p>
      {:else}
        <ul>
          {#each notes as note (note.id)}
            <li>
              <button class:selected={note.id === selectedId && !noteActive} onclick={() => (selectedId = note.id)}>
                <span class="name">{note.title || "Untitled note"}</span>
                <span class="small muted num">{dayLabel(note.created_at)} · {lengthLabel(note.duration_ms)}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </aside>

    <article class="detail" aria-live={noteActive ? "polite" : undefined}>
      {#if noteActive}
        <div class="live-head">
          {#if recordingNote}
            <span class="live-dot"></span>
            <h2>Recording</h2>
            <span class="num muted clock">{duration(now - status.started_at)}</span>
          {:else}
            <h2>{status.phase === "polishing" ? "Writing the summary…" : "Finishing the transcript…"}</h2>
          {/if}
        </div>
        <p class="muted small">Hams transcribes every 30 seconds while you talk, so a long recording is ready moments after you finish. Nothing leaves this Mac{app.settings?.ai.provider !== "off" ? " until the summary, which sends only the text" : ""}.</p>
          <div class="transcript live-text">
            {#if live}
              {#each live.split("\n\n") as paragraph}<p>{paragraph}</p>{/each}
            {:else}
              <p class="muted">The first words appear about 30 seconds in.</p>
            {/if}
          </div>
      {:else if selected}
        <input
          class="title-input"
          type="text"
          value={selected.title}
          placeholder="Untitled note"
          aria-label="Note title"
          onchange={(e) => rename(selected, e.currentTarget.value)}
          onkeydown={(e) => e.key === "Enter" && e.currentTarget.blur()}
        />
        <div class="meta">
          <span class="small muted num">{dayLabel(selected.created_at)}, {clockTime(selected.created_at)} · {lengthLabel(selected.duration_ms)} · {groupedNumber(selected.words)} words</span>
          <div class="row">
            {#if selected.summary}
              <button class="small" onclick={() => app.run(() => api.copy(selected.summary), "Summary copied")}><Icon name="copy" size={14} />Copy summary</button>
            {/if}
            <button class="small" onclick={() => app.run(() => api.copy(fullText(selected)), "Note copied")}><Icon name="copy" size={14} />Copy all</button>
            <ConfirmButton small ghost label="Delete note" confirm="Delete this note?" onconfirm={() => remove(selected)}><Icon name="trash" size={14} /></ConfirmButton>
          </div>
        </div>

        {#if selected.summary}
          <section aria-label="Key points">
          <h3 class="label">Key points</h3>
          <div class="well summary">
            {#each groups(summaryBlocks(selected.summary)) as group}
              {#if group.heading}<h3>{group.heading}</h3>{/if}
              <ul>
                {#each group.items as item}<li class:plain={item.kind === "text"}>{item.text}</li>{/each}
              </ul>
            {/each}
          </div>
          </section>
        {:else if app.settings?.ai.provider === "off"}
          <p class="small muted">Turn on an AI provider in Settings to get a title and key points with each note.</p>
        {/if}

        <section aria-label="Transcript">
          <h3 class="label">Transcript</h3>
          <div class="transcript">
            {#each selected.transcript.split("\n\n") as paragraph}<p>{paragraph}</p>{/each}
          </div>
        </section>
      {:else if loaded}
        <div class="first">
          <h2>Record your first note</h2>
          <p class="muted">Start a note for a lecture, a meeting or a walk-and-talk. It can run for as long as you like. When you finish, Hams saves the full transcript and, with AI on, a title and the key points.</p>
          <button class="primary" onclick={start} disabled={dictating}><Icon name="mic" />New voice note</button>
          <p class="small muted">You can also start one from the Hams menu in the menu bar.</p>
        </div>
      {/if}
    </article>
  </div>
</div>

<style>
  .wide { max-width: 1040px; }
  .layout { display: grid; grid-template-columns: 250px minmax(0, 1fr); gap: 28px; align-items: start; }
  .list { display: flex; flex-direction: column; gap: 10px; position: sticky; top: 0; }
  .search { position: relative; display: block; color: var(--muted); }
  .search :global(.icon) { position: absolute; left: 10px; top: 10px; }
  .search input { padding-left: 30px; }
  .list ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 2px; }
  .list button {
    width: 100%; height: auto; padding: 8px 10px; border: none; background: transparent;
    flex-direction: column; align-items: flex-start; gap: 2px; text-align: left;
  }
  .list button:hover:not(.selected) { background: var(--raised); }
  .list button.selected { background: var(--active); }
  .name { font-weight: 500; font-size: 14px; width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .detail { min-width: 0; display: flex; flex-direction: column; gap: 16px; }
  .title-input {
    border: 1px solid transparent; background: transparent; padding: 2px 6px; margin: -3px -7px;
    font-size: 19px; font-weight: 650; letter-spacing: -0.01em; min-height: 0; width: calc(100% + 14px);
  }
  .title-input:hover { border-color: var(--border); }
  .meta { display: flex; justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap; }
  .summary h3 { margin: 12px 0 6px; }
  .summary h3:first-child { margin-top: 0; }
  .summary ul { margin: 0; padding-left: 18px; display: flex; flex-direction: column; gap: 5px; }
  .summary li.plain { list-style: none; margin-left: -18px; }
  .summary li::marker { color: var(--data); }
  .label { font-size: 13px; color: var(--muted); font-weight: 500; margin-bottom: 8px; }
  .transcript { display: flex; flex-direction: column; gap: 12px; line-height: 1.6; max-width: 68ch; user-select: text; }
  .live-head { display: flex; align-items: center; gap: 10px; }
  .clock { font-size: 15px; }
  .live-text { padding: 16px 18px; border-radius: var(--radius-card); background: var(--raised); min-height: 120px; }
  .first { display: flex; flex-direction: column; align-items: flex-start; gap: 12px; padding: 24px; border: 1px dashed var(--control); border-radius: var(--radius-card); }
  .first p { max-width: 56ch; }
  @media (max-width: 820px) {
    .layout { grid-template-columns: 1fr; }
    .list { position: static; }
  }
</style>
