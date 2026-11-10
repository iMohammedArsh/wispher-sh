<script lang="ts">
  // Settings: one view per area, picked from the list on the left. Every control saves itself.
  import { api, errorText, on, type AiProvider, type DeliveryMode, type ModelProgress, type ModelStatus, type Permissions, type RestyleResult } from "../lib/ipc";
  import { app, type SettingsArea } from "../lib/state.svelte";
  import { shortcutLabel } from "../lib/format";
  import ConfirmButton from "../components/ConfirmButton.svelte";
  import HeardWritten from "../components/HeardWritten.svelte";
  import Icon from "../components/Icon.svelte";

  let models = $state.raw<ModelStatus[]>([]);
  let progress = $state<Record<string, number>>({});
  let microphones = $state.raw<string[]>([]);
  let permissions = $state.raw<Permissions | null>(null);
  let shortcuts = $state({ dictate: "", command: "", paste: "" });
  let apiKey = $state("");
  let defaultModel = $state("");
  let trial = $state<{ ok: true; result: RestyleResult } | { ok: false; text: string } | null>(null);
  let trying = $state(false);
  let openAtLogin = $state(false);
  let version = $state("");
  let checking = $state(false);
  let checkedAt = $state<number | null>(null);

  const s = $derived(app.settings);
  const mac = $derived(app.platform === "macos");
  const key = (v: string) => shortcutLabel(v, app.platform);

  $effect(() => {
    api.models().then((m) => (models = m)).catch(() => {});
    api.microphones().then((m) => (microphones = m)).catch(() => {});
    api.permissions().then((p) => (permissions = p)).catch(() => {});
    api.autostartEnabled().then((v) => (openAtLogin = v)).catch(() => {});
    api.appVersion().then((v) => (version = v)).catch(() => {});
    const off = on<ModelProgress>("model-progress", (p) => (progress = { ...progress, [p.id]: p.progress ?? 0 }));
    return () => void off.then((f) => f());
  });

  $effect(() => {
    if (s) shortcuts = { dictate: s.dictate_shortcut, command: s.command_shortcut, paste: s.paste_last_shortcut };
  });

  $effect(() => {
    const provider = s?.ai.provider;
    if (provider) api.defaultAiModel(provider).then((m) => (defaultModel = m)).catch(() => {});
  });

  const areas = $derived(
    (
      [
        { id: "general", label: "General", sub: "How Hams starts and where it lives." },
        { id: "shortcuts", label: "Shortcuts", sub: "The keys you hold to talk." },
        { id: "speech", label: "Speech", sub: "Recognised on this computer, so it works offline and your voice never leaves it." },
        { id: "output", label: "Output", sub: "What happens to the text once it's written." },
        { id: "ai", label: "AI polish", sub: "Optional. Fixes grammar, follows each app's style, edits selected text and summarises notes. Only text is sent, never audio." },
        { id: "learning", label: "Learning", sub: "Hams learns your words from corrections. It only suggests; you approve each one in Dictionary." },
        { id: "updates", label: "Updates", sub: "New versions are signed, and Hams checks the signature before installing anything." },
        { id: "permissions", label: "Permissions", sub: "macOS asks for these once." },
      ] satisfies { id: SettingsArea; label: string; sub: string }[]
    ).filter((a) => a.id !== "permissions" || permissions?.applicable),
  );
  const area = $derived(areas.find((a) => a.id === app.settingsArea) ?? areas[0]);

  const presets = $derived(
    mac
      ? ["Meta+Shift+Space", "Ctrl+Shift+Space", "Fn", "RightAlt", "RightMeta", "Meta+Alt+Shift+V", "F13"]
      : ["Ctrl+Shift+Space", "Ctrl+Alt+Space", "RightAlt", "RightCtrl", "Ctrl+Alt+Shift+V", "F13"],
  );

  const providers: { id: AiProvider; label: string; key: boolean }[] = [
    { id: "off", label: "Off (local only)", key: false },
    { id: "gemini", label: "Google Gemini", key: true },
    { id: "open_ai", label: "OpenAI", key: true },
    { id: "anthropic", label: "Anthropic Claude", key: true },
    { id: "groq", label: "Groq (very fast)", key: true },
    { id: "open_router", label: "OpenRouter", key: true },
    { id: "ollama", label: "Ollama (on this computer)", key: false },
    { id: "custom", label: "Custom OpenAI-compatible", key: true },
  ];
  const languages = [
    ["auto", "Detect automatically"], ["en", "English"], ["ml", "Malayalam"], ["ms", "Malay"], ["id", "Indonesian"], ["hi", "Hindi"],
    ["ta", "Tamil"], ["ur", "Urdu"], ["ar", "Arabic"], ["es", "Spanish"], ["fr", "French"], ["de", "German"], ["pt", "Portuguese"],
    ["ja", "Japanese"], ["zh", "Chinese"],
  ];
  const languageName = (id: string) => languages.find(([code]) => code === id)?.[1] ?? id;
  /** Tick or untick a language detection may choose; the first one stays the main language. */
  function toggleLanguage(id: string) {
    const current = s?.languages ?? [];
    app.save({ languages: current.includes(id) ? current.filter((l) => l !== id) : [...current, id] });
  }
  function setMainLanguage(id: string) {
    const current = s?.languages ?? [];
    app.save({ languages: [id, ...current.filter((l) => l !== id)] });
  }
  // Each delivery mode, and what's true once a dictation has been delivered.
  const deliveries: { id: DeliveryMode; label: string; pasted: boolean; clipboard: string }[] = [
    { id: "paste_restore", label: "Paste, keep my clipboard", pasted: true, clipboard: "What you had copied before" },
    { id: "paste", label: "Paste and keep it copied", pasted: true, clipboard: "The dictation, to paste again" },
    { id: "copy_only", label: "Copy only", pasted: false, clipboard: "The dictation, for you to paste" },
  ];
  const delivery = $derived(deliveries.find((d) => d.id === s?.delivery) ?? deliveries[0]);

  const SAMPLE = "um so i think we should uh move the review to thursday if that works for everyone";

  const shortcutsChanged = $derived(
    !!s && (shortcuts.dictate !== s.dictate_shortcut || shortcuts.command !== s.command_shortcut || shortcuts.paste !== s.paste_last_shortcut),
  );
  const providerInfo = $derived(providers.find((p) => p.id === s?.ai.provider));

  async function toggleAutostart(enabled: boolean) {
    await app.run(() => api.setAutostart(enabled), enabled ? "Hams will open at login" : "Hams won’t open at login");
    openAtLogin = await api.autostartEnabled().catch(() => enabled);
  }

  async function saveShortcuts() {
    if (await app.save({ dictate_shortcut: shortcuts.dictate, command_shortcut: shortcuts.command, paste_last_shortcut: shortcuts.paste })) {
      app.notify("Shortcuts updated");
    }
  }

  async function download(id: string) {
    progress = { ...progress, [id]: 0 };
    try {
      await api.downloadModel(id);
      models = await api.models();
      app.notify("Model ready");
    } catch (error) {
      app.notify(errorText(error), "error");
    } finally {
      const { [id]: _, ...rest } = progress;
      progress = rest;
    }
  }

  async function remove(id: string) {
    await app.run(() => api.deleteModel(id), "Model deleted");
    models = await api.models();
  }

  async function saveKey() {
    if (!s) return;
    await app.run(() => api.setApiKey(s.ai.provider, apiKey), apiKey ? "Key saved to your keychain" : "Key removed");
    apiKey = "";
    await app.reloadSettings();
  }

  async function tryPolish() {
    trying = true;
    trial = null;
    try {
      trial = { ok: true, result: await api.styleTry("default", SAMPLE) };
    } catch (error) {
      trial = { ok: false, text: errorText(error) };
    } finally {
      trying = false;
    }
  }

  async function checkForUpdate() {
    checking = true;
    try {
      app.update = await api.updateCheck();
      checkedAt = Date.now();
    } catch (error) {
      app.notify(errorText(error), "error");
    } finally {
      checking = false;
    }
  }
</script>

{#snippet toggle(title: string, detail: string, checked: boolean, onchange: (on: boolean) => void, disabled = false)}
  <label class="switch-row">
    <span class="text"><strong>{title}</strong><span>{detail}</span></span>
    <input class="switch" type="checkbox" {checked} {disabled} onchange={(e) => onchange(e.currentTarget.checked)} />
  </label>
{/snippet}

{#if s}
  <div class="settings">
    <nav class="areas" aria-label="Settings">
      <p class="areas-title">Settings</p>
      {#each areas as a (a.id)}
        <button class:active={a.id === area.id} aria-current={a.id === area.id ? "page" : undefined} onclick={() => (app.settingsArea = a.id)}>
          {a.label}
          {#if a.id === "updates" && app.update}<span class="dot" title="Update ready"></span>{/if}
          {#if a.id === "permissions" && permissions && !permissions.accessibility}<span class="dot warn" title="Needs attention"></span>{/if}
        </button>
      {/each}
    </nav>

    <div class="page view">
      <header class="title-row">
        <div>
          <h1>{area.label}</h1>
          <p class="sub">{area.sub}</p>
        </div>
      </header>

      {#if area.id === "general"}
        <div class="panel">
          {@render toggle("Open Hams at login", "Starts quietly in the menu bar, ready for your shortcut.", openAtLogin, toggleAutostart)}
          <div class="switch-row">
            <span class="text"><strong>Closing the window</strong><span>Keeps Hams running in the menu bar. Quit from the menu bar icon.</span></span>
          </div>
        </div>

      {:else if area.id === "shortcuts"}
        <datalist id="shortcut-presets">{#each presets as p (p)}<option value={p}></option>{/each}</datalist>
        <div class="panel flush">
          <ul class="rows keys">
            <li>
              <label class="field">
                <span>Dictate</span>
                <input type="text" list="shortcut-presets" bind:value={shortcuts.dictate} />
              </label>
              <dl class="does">
                <div><dt>Hold <kbd>{key(shortcuts.dictate)}</kbd></dt><dd>Talk, let go to paste</dd></div>
                {#if s.double_tap_lock}<div><dt>Double-tap</dt><dd>Hands-free until you tap again</dd></div>{/if}
                <div><dt><kbd>Esc</kbd></dt><dd>Cancel while recording</dd></div>
              </dl>
            </li>
            <li>
              <label class="field">
                <span>Edit selected text</span>
                <input type="text" list="shortcut-presets" bind:value={shortcuts.command} />
              </label>
              <dl class="does">
                <div><dt>Select, hold <kbd>{key(shortcuts.command)}</kbd></dt><dd>Say what to change: “make it formal”</dd></div>
                <div><dt>Nothing selected</dt><dd>Say what to write: “reply that I'll be late”</dd></div>
              </dl>
            </li>
            <li>
              <label class="field">
                <span>Paste last dictation</span>
                <input type="text" list="shortcut-presets" bind:value={shortcuts.paste} placeholder="Off" />
              </label>
              <dl class="does">
                <div><dt>{#if shortcuts.paste}Press <kbd>{key(shortcuts.paste)}</kbd>{:else}Off{/if}</dt><dd>Pastes your last dictation again</dd></div>
              </dl>
            </li>
          </ul>
          <div class="panel-foot">
            <span class="muted small grow">
              Combine Ctrl, {mac ? "Option, Shift and Cmd" : "Alt, Shift and Win"} with a key, or use one key on its own like {mac ? "Fn or RightAlt. For Fn, set “Press 🌐 key to” to “Do Nothing” in Keyboard settings." : "RightAlt or RightCtrl."} <!-- taste-skill-ignore: 🌐 is how macOS names this key setting -->
            </span>
            <button class="primary" onclick={saveShortcuts} disabled={!shortcutsChanged}>Save shortcuts</button>
          </div>
        </div>
        <div class="panel">
          {@render toggle("Double-tap for hands-free", "Tap the dictate shortcut twice to keep recording without holding it.", s.double_tap_lock, (v) => app.save({ double_tap_lock: v }))}
        </div>

      {:else if area.id === "speech"}
        <section aria-labelledby="models-title">
          <div class="section-head"><h2 id="models-title">Model</h2><span class="aside">Bigger is more accurate and slower</span></div>
          <table class="panel models">
            <thead><tr><th>Name</th><th class="num">Size</th><th></th></tr></thead>
            <tbody>
              {#each models as m (m.id)}
                <tr class:selected={s.model === m.id}>
                  <td>
                    <label class="model">
                      <input type="radio" name="model" checked={s.model === m.id} disabled={!m.installed} onchange={() => app.save({ model: m.id })} />
                      <span><strong>{m.label}</strong><span class="muted small block">{m.note}</span></span>
                    </label>
                  </td>
                  <td class="num muted">{m.size_mb} MB</td>
                  <td class="act">
                    {#if progress[m.id] !== undefined}
                      <span class="progress"><progress max="1" value={progress[m.id]}></progress><span class="small muted num">{Math.round(progress[m.id] * 100)}%</span></span>
                    {:else if s.model === m.id && m.installed}
                      <span class="pill ok">In use</span>
                    {:else if m.installed}
                      <ConfirmButton small ghost confirm="Delete from disk?" onconfirm={() => remove(m.id)}>Delete</ConfirmButton>
                    {:else}
                      <button class="small" onclick={() => download(m.id)}>Download</button>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </section>
        <div class="fields">
          <label class="field">
            <span>Language</span>
            <select value={s.language} onchange={(e) => app.save({ language: e.currentTarget.value })}>
              {#each languages as [id, label] (id)}<option value={id}>{label}</option>{/each}
            </select>
          </label>
          <label class="field">
            <span>Microphone</span>
            <select value={s.microphone ?? ""} onchange={(e) => app.save({ microphone: e.currentTarget.value || null })}>
              <option value="">System default</option>
              {#each microphones as mic (mic)}<option value={mic}>{mic}</option>{/each}
            </select>
          </label>
        </div>
        {#if s.language === "auto"}
          <section class="spoken" aria-labelledby="spoken-title">
            <div class="section-head">
              <h2 id="spoken-title">Languages you speak</h2>
              <span class="aside">Detection only picks from these</span>
            </div>
            <div class="chips" role="group" aria-labelledby="spoken-title">
              {#each languages.slice(1) as [id, label] (id)}
                {@const on = s.languages.includes(id)}
                <button class="chip" class:on aria-pressed={on} onclick={() => toggleLanguage(id)}>{label}</button>
              {/each}
            </div>
            {#if s.languages.length === 0}
              <p class="small muted">Any language can be picked, so English with a strong accent may come out in another language. Tick the ones you speak.</p>
            {:else}
              <p class="small muted">
                A strong accent can't switch you to a language that isn't ticked. Speech in an unticked language comes out in one of these.
                {#if s.languages.length === 1}With one language ticked, everything is written in {languageName(s.languages[0])}.{/if}
              </p>
              {#if s.languages.length > 1}
                <label class="field main-language">
                  <span>When unsure, use</span>
                  <select value={s.languages[0]} onchange={(e) => setMainLanguage(e.currentTarget.value)}>
                    {#each s.languages as id (id)}<option value={id}>{languageName(id)}</option>{/each}
                  </select>
                </label>
              {/if}
            {/if}
          </section>
        {/if}
        <div class="panel">
          {@render toggle("Live preview", "Shows your words in the overlay while you speak. Uses more battery.", s.live_preview, (v) => app.save({ live_preview: v }))}
        </div>

      {:else if area.id === "output"}
        <div class="output">
          <div class="choices" role="radiogroup" aria-label="Delivery">
            {#each deliveries as d (d.id)}
              <label class="choice" class:selected={s.delivery === d.id}>
                <input type="radio" name="delivery" checked={s.delivery === d.id} onchange={() => app.save({ delivery: d.id })} />
                <strong>{d.label}</strong>
              </label>
            {/each}
          </div>
          <dl class="panel result" aria-label="After a dictation">
            <p class="result-title">After a dictation</p>
            <div><dt>In the app you were using</dt><dd>{#if delivery.pasted}<Icon name="check" size={14} /> The text is pasted{:else}Nothing is pasted{/if}</dd></div>
            <div><dt>On your clipboard</dt><dd>{delivery.clipboard}</dd></div>
          </dl>
        </div>
        <div class="panel">
          {@render toggle("Remove filler words", "Drops “um”, “uh” and stutters.", s.remove_fillers, (v) => app.save({ remove_fillers: v }))}
          {@render toggle("Voice commands", "“New line”, “new paragraph” and “scratch that” edit as you speak.", s.voice_commands, (v) => app.save({ voice_commands: v }))}
        </div>

      {:else if area.id === "ai"}
        <div class="panel stack">
          <div class="fields">
            <label class="field">
              <span>Provider</span>
              <select value={s.ai.provider} onchange={(e) => { trial = null; app.save({ ai: { ...s.ai, provider: e.currentTarget.value as AiProvider, model: "" } }); }}>
                {#each providers as p (p.id)}<option value={p.id}>{p.label}</option>{/each}
              </select>
            </label>
            {#if s.ai.provider !== "off"}
              <label class="field">
                <span>Model</span>
                <input type="text" value={s.ai.model} placeholder={defaultModel || "model name"} onchange={(e) => app.save({ ai: { ...s.ai, model: e.currentTarget.value } })} />
              </label>
            {/if}
          </div>
          {#if s.ai.provider === "custom" || s.ai.provider === "ollama"}
            <label class="field">
              <span>Endpoint URL</span>
              <input type="text" value={s.ai.base_url} placeholder={s.ai.provider === "ollama" ? "http://127.0.0.1:11434/v1" : "https://…/v1"}
                onchange={(e) => app.save({ ai: { ...s.ai, base_url: e.currentTarget.value } })} />
            </label>
          {/if}
          {#if providerInfo?.key}
            <div class="field">
              <span>API key <span class="hint">{app.view?.has_api_key ? "Saved in your keychain" : "Kept in your keychain, never in a file"}</span></span>
              <div class="joined">
                <input type="password" bind:value={apiKey} placeholder={app.view?.has_api_key ? "••••••••••••" : "Paste your API key"} aria-label="API key" />
                <button onclick={saveKey}>{apiKey ? "Save key" : app.view?.has_api_key ? "Remove key" : "Save key"}</button>
              </div>
            </div>
          {/if}
          {#if s.ai.provider !== "off"}
            <div class="fields">
              <label class="field">
                <span>Skip AI for dictations shorter than</span>
                <select value={s.ai.smart_skip_words} onchange={(e) => app.save({ ai: { ...s.ai, smart_skip_words: Number(e.currentTarget.value) } })}>
                  {#each [0, 2, 4, 6, 10] as n (n)}<option value={n}>{n === 0 ? "Never skip" : `${n} words`}</option>{/each}
                </select>
              </label>
              <label class="field">
                <span>Paste the plain text after</span>
                <select value={s.ai.timeout_ms} onchange={(e) => app.save({ ai: { ...s.ai, timeout_ms: Number(e.currentTarget.value) } })}>
                  {#each [3000, 6000, 10000, 20000] as n (n)}<option value={n}>{n / 1000} seconds</option>{/each}
                </select>
              </label>
            </div>
          {/if}
        </div>

        <section class="panel stack" aria-labelledby="try-title">
          <div class="spread">
            <h2 id="try-title">Try it</h2>
            <button onclick={tryPolish} disabled={trying}>{#if trying}<span class="spinner"></span>Writing…{:else}Run on a sample{/if}</button>
          </div>
          {#if trial?.ok}
            <HeardWritten raw={SAMPLE} text={trial.result.text} summary />
            {#if !trial.result.polished}<p class="muted small">AI polish is off, so this is Hams' local clean-up only.</p>{/if}
          {:else if trial}
            <p class="danger-text small">{trial.text}</p>
          {:else}
            <p class="muted">See how a rambling sentence comes out with these settings: <q>{SAMPLE}</q></p>
          {/if}
        </section>

      {:else if area.id === "learning"}
        <div class="panel">
          {@render toggle("Learn from edits in History", "When you fix a dictation in History, the change becomes a suggestion.", s.learn_from_edits, (v) => app.save({ learn_from_edits: v }))}
          {@render toggle(
            "Learn from edits in other apps",
            mac
              ? "After a paste, Hams re-reads that text field for up to a minute. If you correct a word it wrote, it suggests the fix. Password fields are never read, and the field's text is never saved."
              : "Coming to Windows later. Fix dictations in History instead.",
            s.learn_from_fields,
            (v) => app.save({ learn_from_fields: v }),
            !mac,
          )}
        </div>
        {#if app.pendingSuggestions}
          <div class="notice">
            <span class="grow">{app.pendingSuggestions} {app.pendingSuggestions === 1 ? "suggestion is" : "suggestions are"} waiting for you.</span>
            <button onclick={() => (app.route = "dictionary")}>Review in Dictionary</button>
          </div>
        {/if}

      {:else if area.id === "updates"}
        <div class="panel version">
          <div class="grow">
            {#if app.update}
              <strong>Hams {app.update.version} is ready</strong>
              <span class="small muted block">You have {app.update.current_version}.{app.update.notes ? ` ${app.update.notes}` : ""}</span>
            {:else}
              <strong>Hams <span class="num">{version}</span></strong>
              <span class="small muted block">{checkedAt ? "You're up to date." : "Checked automatically every few hours."}</span>
            {/if}
          </div>
          {#if app.update}
            {#if app.updateProgress !== undefined}
              <span class="progress">
                <progress max="1" value={app.updateProgress ?? undefined}></progress>
                <span class="small muted num">{app.updateProgress === null ? "Downloading" : app.updateProgress >= 1 ? "Installing" : `${Math.round(app.updateProgress * 100)}%`}</span>
              </span>
            {:else}
              <button class="primary" onclick={() => app.installUpdate()}>Install and restart</button>
            {/if}
          {:else}
            <button onclick={checkForUpdate} disabled={checking}>{checking ? "Checking…" : "Check now"}</button>
          {/if}
        </div>
        <div class="panel">
          {@render toggle("Check for updates automatically", "Hams only downloads an update when you click Install.", s.check_updates, (v) => app.save({ check_updates: v }))}
        </div>

      {:else if area.id === "permissions" && permissions}
        <ul class="panel flush rows">
          <li class="spread">
            <span><strong class="label">Accessibility</strong><span class="small muted block">Shortcut, paste, selected text and learning from fields</span></span>
            {#if permissions.accessibility}<span class="pill ok">Allowed</span>{:else}<button class="small" onclick={() => api.openPermissionSettings("accessibility")}>Open System Settings</button>{/if}
          </li>
          <li class="spread">
            <span><strong class="label">Microphone</strong><span class="small muted block">macOS asks the first time you dictate</span></span>
            <button class="small" onclick={() => api.openPermissionSettings("microphone")}>Open System Settings</button>
          </li>
        </ul>
        <p class="muted small">Signing Hams with the local certificate keeps these permissions across updates.</p>
      {/if}
    </div>
  </div>
{/if}

<style>
  .settings { display: grid; grid-template-columns: 168px minmax(0, 1fr); gap: 32px; align-items: start; max-width: 1000px; }
  .areas { position: sticky; top: 0; display: flex; flex-direction: column; gap: 2px; padding-top: 2px; }
  .areas-title { font-size: 12px; color: var(--muted); padding: 0 10px 8px; }
  .areas button { height: 30px; justify-content: flex-start; border: none; background: transparent; padding: 0 10px; font-weight: 400; color: var(--muted); }
  .areas button:hover:not(.active) { background: color-mix(in srgb, var(--active) 60%, transparent); color: var(--text); }
  .areas button.active { background: var(--active); color: var(--text); font-weight: 500; }
  .areas .dot { width: 6px; height: 6px; border-radius: 50%; background: var(--data); margin-left: auto; }
  .areas .dot.warn { background: var(--pill-warning-ink); }
  .view { max-width: 720px; }

  .block { display: block; }
  .label { font-weight: 500; }
  .panel > .switch-row:first-child { padding-top: 0; }
  .stack { gap: 16px; }

  .keys > li { display: flex; flex-direction: column; gap: 12px; padding-top: 16px; padding-bottom: 16px; }
  .keys .field { max-width: 300px; }
  .spoken { display: flex; flex-direction: column; gap: 10px; }
  .spoken p { margin: 0; max-width: 68ch; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip { height: 26px; padding: 0 10px; border-radius: 999px; font-size: 13px; font-weight: 400; color: var(--muted); border-color: var(--border); }
  .chip.on { background: var(--active); border-color: var(--active); color: var(--text); font-weight: 500; }
  .main-language { max-width: 240px; }
  .does { margin: 0; display: flex; flex-direction: column; gap: 6px; }
  .does div { display: grid; grid-template-columns: 180px minmax(0, 1fr); gap: 12px; align-items: baseline; font-size: 13px; }
  .does dt { color: var(--text); }
  .does dd { margin: 0; color: var(--muted); }
  .does kbd { height: 20px; font-size: 12px; }
  .panel-foot { display: flex; align-items: center; gap: 16px; padding: 14px var(--pad); border-top: 1px solid var(--rule); }

  .models { width: 100%; border-collapse: separate; border-spacing: 0; padding: 0; overflow: hidden; }
  .models th { font-size: 12px; font-weight: 400; color: var(--muted); text-align: left; padding: 10px 16px 8px; }
  .models th.num, .models td.num { text-align: right; white-space: nowrap; }
  .models td { padding: 12px 16px; border-top: 1px solid var(--rule); vertical-align: middle; }
  .models tr.selected td { background: var(--raised); }
  .models .act { text-align: right; width: 140px; }
  .model { display: flex; align-items: flex-start; gap: 10px; cursor: pointer; }
  .model input { margin-top: 4px; }
  .model strong, .choice strong { font-weight: 500; }
  .progress { display: inline-flex; align-items: center; gap: 8px; }
  .progress progress { width: 80px; }

  .output { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 16px; align-items: stretch; }
  .choices { display: grid; gap: 6px; grid-auto-rows: 1fr; }
  .choice { display: flex; align-items: center; gap: 10px; padding: 0 12px; min-height: 44px; border: 1px solid var(--border); border-radius: var(--radius-card); cursor: pointer; background: var(--surface); }
  .choice.selected { border-color: var(--text); }
  .result { margin: 0; display: flex; flex-direction: column; gap: 12px; background: var(--raised); border-color: transparent; }
  .result-title { font-size: 12px; color: var(--muted); }
  .result dt { font-size: 12px; color: var(--muted); }
  .result dd { margin: 2px 0 0; display: flex; align-items: center; gap: 6px; font-weight: 500; }
  .result dd :global(.icon) { color: var(--data); }

  .joined { display: flex; }
  .joined input { border-top-right-radius: 0; border-bottom-right-radius: 0; }
  .joined button { height: auto; border-top-left-radius: 0; border-bottom-left-radius: 0; margin-left: -1px; }
  q { color: var(--text); }
  .version { display: flex; align-items: center; gap: 16px; padding: 16px var(--pad); }
  .version strong { font-weight: 600; }

  @media (max-width: 820px) {
    .settings { grid-template-columns: 1fr; gap: 16px; }
    .areas { position: static; flex-direction: row; flex-wrap: wrap; }
    .areas-title { display: none; }
    .output { grid-template-columns: 1fr; }
  }
</style>
