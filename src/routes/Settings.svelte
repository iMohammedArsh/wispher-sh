<script lang="ts">
  import { api, errorText, on, type AiProvider, type DeliveryMode, type ModelStatus, type Permissions } from "../lib/ipc";
  import { app } from "../lib/state.svelte";
  import { shortcutLabel } from "../lib/format";

  let models = $state<ModelStatus[]>([]);
  let progress = $state<Record<string, number>>({});
  let microphones = $state<string[]>([]);
  let permissions = $state<Permissions | null>(null);
  let shortcuts = $state({ dictate: "", command: "", paste: "" });
  let apiKey = $state("");
  let defaultModel = $state("");
  let aiTest = $state<{ ok: boolean; text: string } | null>(null);
  let testing = $state(false);
  let openAtLogin = $state(false);

  const s = $derived(app.settings);

  $effect(() => {
    api.models().then((m) => (models = m)).catch(() => {});
    api.microphones().then((m) => (microphones = m)).catch(() => {});
    api.permissions().then((p) => (permissions = p)).catch(() => {});
    api.autostartEnabled().then((v) => (openAtLogin = v)).catch(() => {});
    const off = on<{ id: string; progress: number }>("model-progress", (p) => (progress = { ...progress, [p.id]: p.progress }));
    return () => void off.then((f) => f());
  });

  $effect(() => {
    if (s) shortcuts = { dictate: s.dictate_shortcut, command: s.command_shortcut, paste: s.paste_last_shortcut };
  });

  $effect(() => {
    const provider = s?.ai.provider;
    if (provider) api.defaultAiModel(provider).then((m) => (defaultModel = m)).catch(() => {});
  });

  const presets = $derived(
    app.platform === "macos"
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
    ["auto", "Detect automatically"], ["en", "English"], ["ml", "Malayalam"], ["hi", "Hindi"], ["ta", "Tamil"],
    ["ar", "Arabic"], ["es", "Spanish"], ["fr", "French"], ["de", "German"], ["pt", "Portuguese"], ["ja", "Japanese"], ["zh", "Chinese"],
  ];
  const deliveries: { id: DeliveryMode; label: string; hint: string }[] = [
    { id: "paste_restore", label: "Paste, keep my clipboard", hint: "Pastes, then puts back whatever you had copied." },
    { id: "paste", label: "Paste and leave on clipboard", hint: "Handy if you want to paste it again elsewhere." },
    { id: "copy_only", label: "Copy only", hint: "You paste it yourself." },
  ];

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
    if (!confirm("Delete this model from disk?")) return;
    await app.run(() => api.deleteModel(id));
    models = await api.models();
  }

  async function saveKey() {
    if (!s) return;
    await app.run(() => api.setApiKey(s.ai.provider, apiKey), apiKey ? "Key saved to your keychain" : "Key removed");
    apiKey = "";
    await app.reloadSettings();
  }

  async function testAi() {
    testing = true;
    aiTest = null;
    try {
      aiTest = { ok: true, text: await api.testAi() };
    } catch (error) {
      aiTest = { ok: false, text: errorText(error) };
    } finally {
      testing = false;
    }
  }

  const providerInfo = $derived(providers.find((p) => p.id === s?.ai.provider));
</script>

{#if s}
  <div class="stack page">
    <h1>Settings</h1>

    <section class="card stack">
      <h2>Shortcuts</h2>
      <datalist id="shortcut-presets">{#each presets as p}<option value={p}></option>{/each}</datalist>
      <div class="fields">
        <label>
          <span>Dictate (hold)</span>
          <input type="text" list="shortcut-presets" bind:value={shortcuts.dictate} />
          <span class="hint">{shortcutLabel(shortcuts.dictate, app.platform)}</span>
        </label>
        <label>
          <span>Command mode (hold)</span>
          <input type="text" list="shortcut-presets" bind:value={shortcuts.command} />
          <span class="hint">{shortcutLabel(shortcuts.command, app.platform)}</span>
        </label>
        <label>
          <span>Paste last dictation</span>
          <input type="text" list="shortcut-presets" bind:value={shortcuts.paste} placeholder="Off" />
          <span class="hint">{shortcuts.paste ? shortcutLabel(shortcuts.paste, app.platform) : "Off"}</span>
        </label>
      </div>
      <p class="small muted">
        Combine Ctrl, Alt/Option, Shift and Meta/Cmd with a key, or hold a single key like
        {app.platform === "macos" ? "Fn or RightAlt (if you use Fn, set “Press 🌐 key to” to “Do Nothing” in Keyboard settings)" : "RightAlt or RightCtrl"}.
      </p>
      <div class="spread">
        <label class="check">
          <input type="checkbox" checked={s.double_tap_lock} onchange={(e) => app.save({ double_tap_lock: e.currentTarget.checked })} />
          Double-tap the dictate shortcut for hands-free mode
        </label>
        <button class="primary" onclick={saveShortcuts}>Save shortcuts</button>
      </div>
    </section>

    <section class="card stack">
      <h2>Speech recognition</h2>
      <p class="small muted">Runs on this computer. Your voice never leaves it.</p>
      <ul class="models">
        {#each models as m (m.id)}
          <li class:selected={s.model === m.id}>
            <label class="check grow">
              <input type="radio" name="model" checked={s.model === m.id} disabled={!m.installed} onchange={() => app.save({ model: m.id })} />
              <span class="grow">
                <strong>{m.label}</strong> <span class="small muted">· {m.size_mb} MB</span>
                <span class="small muted block">{m.note}</span>
              </span>
            </label>
            {#if progress[m.id] !== undefined}
              <progress max="1" value={progress[m.id]}></progress>
            {:else if m.installed}
              <button class="ghost small danger" onclick={() => remove(m.id)} disabled={s.model === m.id}>Delete</button>
            {:else}
              <button onclick={() => download(m.id)}>Download</button>
            {/if}
          </li>
        {/each}
      </ul>
      <div class="fields">
        <label>
          <span>Language</span>
          <select value={s.language} onchange={(e) => app.save({ language: e.currentTarget.value })}>
            {#each languages as [id, label]}<option value={id}>{label}</option>{/each}
          </select>
        </label>
        <label>
          <span>Microphone</span>
          <select value={s.microphone ?? ""} onchange={(e) => app.save({ microphone: e.currentTarget.value || null })}>
            <option value="">System default</option>
            {#each microphones as mic}<option value={mic}>{mic}</option>{/each}
          </select>
        </label>
      </div>
    </section>

    <section class="card stack">
      <h2>Output</h2>
      {#each deliveries as d}
        <label class="check">
          <input type="radio" name="delivery" checked={s.delivery === d.id} onchange={() => app.save({ delivery: d.id })} />
          <span>{d.label} <span class="small muted">— {d.hint}</span></span>
        </label>
      {/each}
      <hr />
      <label class="check"><input type="checkbox" checked={s.remove_fillers} onchange={(e) => app.save({ remove_fillers: e.currentTarget.checked })} /> Remove filler words (um, uh)</label>
      <label class="check"><input type="checkbox" checked={s.voice_commands} onchange={(e) => app.save({ voice_commands: e.currentTarget.checked })} /> Voice commands: “new line”, “new paragraph”, “scratch that”</label>
      <label class="check"><input type="checkbox" checked={s.live_preview} onchange={(e) => app.save({ live_preview: e.currentTarget.checked })} /> Live preview: show words in the overlay as you speak <span class="small muted">(uses more battery)</span></label>
      <label class="check"><input type="checkbox" checked={s.learn_from_edits} onchange={(e) => app.save({ learn_from_edits: e.currentTarget.checked })} /> Suggest dictionary fixes when I edit a dictation</label>
    </section>

    <section class="card stack">
      <h2>AI polish & command mode</h2>
      <p class="small muted">Optional. Only the transcript text is sent, never audio. Required for command mode.</p>
      <div class="fields">
        <label>
          <span>Provider</span>
          <select value={s.ai.provider} onchange={(e) => app.save({ ai: { ...s.ai, provider: e.currentTarget.value as AiProvider, model: "" } })}>
            {#each providers as p}<option value={p.id}>{p.label}</option>{/each}
          </select>
        </label>
        {#if s.ai.provider !== "off"}
          <label>
            <span>Model</span>
            <input type="text" value={s.ai.model} placeholder={defaultModel || "model name"} onchange={(e) => app.save({ ai: { ...s.ai, model: e.currentTarget.value } })} />
          </label>
        {/if}
      </div>
      {#if s.ai.provider === "custom" || s.ai.provider === "ollama"}
        <label>
          <span>Endpoint URL</span>
          <input type="text" value={s.ai.base_url} placeholder={s.ai.provider === "ollama" ? "http://127.0.0.1:11434/v1" : "https://…/v1"}
            onchange={(e) => app.save({ ai: { ...s.ai, base_url: e.currentTarget.value } })} />
        </label>
      {/if}
      {#if providerInfo?.key}
        <div class="row">
          <input type="password" bind:value={apiKey} placeholder={app.view?.has_api_key ? "•••••••• saved in keychain" : "Paste your API key"} />
          <button onclick={saveKey}>{apiKey ? "Save key" : app.view?.has_api_key ? "Remove key" : "Save key"}</button>
        </div>
      {/if}
      {#if s.ai.provider !== "off"}
        <div class="fields">
          <label>
            <span>Skip AI for dictations shorter than</span>
            <select value={s.ai.smart_skip_words} onchange={(e) => app.save({ ai: { ...s.ai, smart_skip_words: Number(e.currentTarget.value) } })}>
              {#each [0, 2, 4, 6, 10] as n}<option value={n}>{n === 0 ? "never skip" : `${n} words`}</option>{/each}
            </select>
          </label>
          <label>
            <span>Give up after</span>
            <select value={s.ai.timeout_ms} onchange={(e) => app.save({ ai: { ...s.ai, timeout_ms: Number(e.currentTarget.value) } })}>
              {#each [3000, 6000, 10000, 20000] as n}<option value={n}>{n / 1000} seconds</option>{/each}
            </select>
          </label>
        </div>
        <div class="row">
          <button onclick={testAi} disabled={testing}>{testing ? "Testing…" : "Test connection"}</button>
          {#if aiTest}<span class="small" class:error={!aiTest.ok}>{aiTest.text}</span>{/if}
        </div>
      {/if}
    </section>

    <section class="card stack">
      <h2>General</h2>
      <label class="check">
        <input
          type="checkbox"
          checked={openAtLogin}
          onchange={async (e) => {
            const enabled = e.currentTarget.checked;
            await app.run(() => api.setAutostart(enabled), enabled ? "Hams will open at login" : "Hams won’t open at login");
            openAtLogin = await api.autostartEnabled().catch(() => enabled);
          }}
        />
        Open Hams at login (starts quietly in the menu bar)
      </label>
    </section>

    {#if permissions?.applicable}
      <section class="card stack">
        <h2>Permissions</h2>
        <div class="spread">
          <span>Accessibility <span class="small muted">— shortcut, paste and selected text</span></span>
          {#if permissions.accessibility}<span class="pill ok">Allowed</span>{:else}<button onclick={() => api.openPermissionSettings("accessibility")}>Open Settings</button>{/if}
        </div>
        <div class="spread">
          <span>Microphone</span>
          <button onclick={() => api.openPermissionSettings("microphone")}>Open Settings</button>
        </div>
      </section>
    {/if}
  </div>
{/if}

<style>
  .page { max-width: 760px; }
  .fields { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 12px; }
  label { display: flex; flex-direction: column; gap: 4px; }
  label > span:first-child { font-size: 12.5px; color: var(--muted); }
  label.check { flex-direction: row; align-items: center; gap: 8px; }
  label.check > span:first-child { font-size: inherit; color: inherit; }
  .hint { font-size: 12px; color: var(--muted); }
  .block { display: block; }
  .models { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  .models li { display: flex; align-items: center; gap: 12px; padding: 8px 10px; border: 1px solid var(--border); border-radius: 8px; }
  .models li.selected { border-color: var(--accent); background: var(--accent-soft); }
  progress { width: 120px; accent-color: var(--accent); }
  hr { border: none; border-top: 1px solid var(--border); margin: 4px 0; width: 100%; }
  .error { color: var(--danger); }
</style>
