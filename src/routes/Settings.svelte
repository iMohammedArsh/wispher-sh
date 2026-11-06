<script lang="ts">
  import { api, errorText, on, type AiProvider, type DeliveryMode, type ModelProgress, type ModelStatus, type Permissions } from "../lib/ipc";
  import { app } from "../lib/state.svelte";
  import { shortcutLabel } from "../lib/format";
  import ConfirmButton from "../components/ConfirmButton.svelte";

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
  let version = $state("");
  let checking = $state(false);
  let checkedAt = $state<number | null>(null);

  const s = $derived(app.settings);

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

  const shortcutsChanged = $derived(
    !!s && (shortcuts.dictate !== s.dictate_shortcut || shortcuts.command !== s.command_shortcut || shortcuts.paste !== s.paste_last_shortcut),
  );

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

  const providerInfo = $derived(providers.find((p) => p.id === s?.ai.provider));
</script>

{#if s}
  <div class="page">
    <header class="title-row">
      <div>
        <h1>Settings</h1>
        <p class="sub">Changes save as you make them.</p>
      </div>
    </header>

    <div>
      <section class="setting" aria-labelledby="general">
        <header>
          <h2 id="general">General</h2>
          <p>How Hams starts and where it lives.</p>
        </header>
        <div>
          <label class="switch-row">
            <span class="text"><strong>Open Hams at login</strong><span>Starts quietly in the menu bar, ready for your shortcut.</span></span>
            <input class="switch" type="checkbox" checked={openAtLogin} onchange={(e) => toggleAutostart(e.currentTarget.checked)} />
          </label>
          <div class="switch-row">
            <span class="text"><strong>Closing the window</strong><span>Keeps Hams running in the menu bar. Quit from the menu bar icon.</span></span>
          </div>
        </div>
      </section>

      <section class="setting" aria-labelledby="shortcuts">
        <header>
          <h2 id="shortcuts">Shortcuts</h2>
          <p>
            Combine Ctrl, {app.platform === "macos" ? "Option, Shift and Cmd" : "Alt, Shift and Win"} with a key, or hold one key on its own, like
            {app.platform === "macos" ? "Fn or RightAlt. For Fn, set “Press 🌐 key to” to “Do Nothing” in Keyboard settings." : "RightAlt or RightCtrl."}
          </p>
        </header>
        <div class="stack wide-gap">
          <datalist id="shortcut-presets">{#each presets as p}<option value={p}></option>{/each}</datalist>
          <div class="fields">
            <label class="field">
              <span>Dictate <span class="hint">hold</span></span>
              <input type="text" list="shortcut-presets" bind:value={shortcuts.dictate} />
              <span class="hint keys"><kbd>{shortcutLabel(shortcuts.dictate, app.platform)}</kbd></span>
            </label>
            <label class="field">
              <span>Edit selected text <span class="hint">hold</span></span>
              <input type="text" list="shortcut-presets" bind:value={shortcuts.command} />
              <span class="hint keys"><kbd>{shortcutLabel(shortcuts.command, app.platform)}</kbd></span>
            </label>
            <label class="field">
              <span>Paste last dictation</span>
              <input type="text" list="shortcut-presets" bind:value={shortcuts.paste} placeholder="Off" />
              <span class="hint keys">{#if shortcuts.paste}<kbd>{shortcutLabel(shortcuts.paste, app.platform)}</kbd>{:else}Off{/if}</span>
            </label>
          </div>
          <div class="spread">
            <label class="row small">
              <input class="switch" type="checkbox" checked={s.double_tap_lock} onchange={(e) => app.save({ double_tap_lock: e.currentTarget.checked })} />
              Double-tap the dictate shortcut for hands-free
            </label>
            <button class="primary" onclick={saveShortcuts} disabled={!shortcutsChanged}>Save shortcuts</button>
          </div>
        </div>
      </section>

      <section class="setting" aria-labelledby="speech">
        <header>
          <h2 id="speech">Speech recognition</h2>
          <p>Runs on this computer, so it works offline and your voice never leaves it.</p>
        </header>
        <div class="stack wide-gap">
          <ul class="panel flush rows models" aria-label="Speech models">
            {#each models as m (m.id)}
              <li class:selected={s.model === m.id}>
                <label class="grow model">
                  <input type="radio" name="model" checked={s.model === m.id} disabled={!m.installed} onchange={() => app.save({ model: m.id })} />
                  <span class="grow">
                    <strong>{m.label}</strong> <span class="small muted num">{m.size_mb} MB</span>
                    <span class="small muted block">{m.note}</span>
                  </span>
                </label>
                {#if progress[m.id] !== undefined}
                  <span class="progress"><progress max="1" value={progress[m.id]}></progress><span class="small muted num">{Math.round(progress[m.id] * 100)}%</span></span>
                {:else if s.model === m.id && m.installed}
                  <span class="pill ok">In use</span>
                {:else if m.installed}
                  <ConfirmButton small ghost confirm="Delete from disk?" onconfirm={() => remove(m.id)}>Delete</ConfirmButton>
                {:else}
                  <button class="small" onclick={() => download(m.id)}>Download</button>
                {/if}
              </li>
            {/each}
          </ul>
          <div class="fields">
            <label class="field">
              <span>Language</span>
              <select value={s.language} onchange={(e) => app.save({ language: e.currentTarget.value })}>
                {#each languages as [id, label]}<option value={id}>{label}</option>{/each}
              </select>
            </label>
            <label class="field">
              <span>Microphone</span>
              <select value={s.microphone ?? ""} onchange={(e) => app.save({ microphone: e.currentTarget.value || null })}>
                <option value="">System default</option>
                {#each microphones as mic}<option value={mic}>{mic}</option>{/each}
              </select>
            </label>
          </div>
          <label class="switch-row">
            <span class="text">
              <strong>Keep the microphone ready</strong>
              <span>Recording starts instantly and includes the moment before you pressed the shortcut. The mic stays on while Hams runs{app.platform === "macos" ? ", so macOS shows its microphone dot" : ""}. Only the last moment (under a second) is kept, in memory, and nothing is saved until you dictate.</span>
            </span>
            <input class="switch" type="checkbox" checked={s.keep_mic_warm} onchange={(e) => app.save({ keep_mic_warm: e.currentTarget.checked })} />
          </label>
        </div>
      </section>

      <section class="setting" aria-labelledby="output">
        <header>
          <h2 id="output">Output</h2>
          <p>What happens to the text once it's written.</p>
        </header>
        <div class="stack wide-gap">
          <div class="choices" role="radiogroup" aria-label="Delivery">
            {#each deliveries as d}
              <label class="choice" class:selected={s.delivery === d.id}>
                <input type="radio" name="delivery" checked={s.delivery === d.id} onchange={() => app.save({ delivery: d.id })} />
                <span><strong>{d.label}</strong><span class="small muted block">{d.hint}</span></span>
              </label>
            {/each}
          </div>
          <div>
            <label class="switch-row">
              <span class="text"><strong>Remove filler words</strong><span>Drops “um”, “uh” and stutters.</span></span>
              <input class="switch" type="checkbox" checked={s.remove_fillers} onchange={(e) => app.save({ remove_fillers: e.currentTarget.checked })} />
            </label>
            <label class="switch-row">
              <span class="text"><strong>Voice commands</strong><span>“New line”, “new paragraph” and “scratch that” edit as you speak.</span></span>
              <input class="switch" type="checkbox" checked={s.voice_commands} onchange={(e) => app.save({ voice_commands: e.currentTarget.checked })} />
            </label>
            <label class="switch-row">
              <span class="text"><strong>Live preview</strong><span>Shows your words in the overlay while you speak. Uses more battery.</span></span>
              <input class="switch" type="checkbox" checked={s.live_preview} onchange={(e) => app.save({ live_preview: e.currentTarget.checked })} />
            </label>
          </div>
        </div>
      </section>

      <section class="setting" aria-labelledby="ai">
        <header>
          <h2 id="ai">AI polish</h2>
          <p>Optional. Fixes grammar, follows each app's style, edits selected text and summarises notes. Only the transcript text is sent, never audio.</p>
        </header>
        <div class="stack wide-gap">
          <div class="fields">
            <label class="field">
              <span>Provider</span>
              <select value={s.ai.provider} onchange={(e) => app.save({ ai: { ...s.ai, provider: e.currentTarget.value as AiProvider, model: "" } })}>
                {#each providers as p}<option value={p.id}>{p.label}</option>{/each}
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
              <span>API key <span class="hint">{app.view?.has_api_key ? "saved in your keychain" : "stored in your keychain, never in a file"}</span></span>
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
                  {#each [0, 2, 4, 6, 10] as n}<option value={n}>{n === 0 ? "Never skip" : `${n} words`}</option>{/each}
                </select>
              </label>
              <label class="field">
                <span>Paste the plain text after</span>
                <select value={s.ai.timeout_ms} onchange={(e) => app.save({ ai: { ...s.ai, timeout_ms: Number(e.currentTarget.value) } })}>
                  {#each [3000, 6000, 10000, 20000] as n}<option value={n}>{n / 1000} seconds</option>{/each}
                </select>
              </label>
            </div>
            <div class="row">
              <button onclick={testAi} disabled={testing}>{testing ? "Testing…" : "Test connection"}</button>
              {#if aiTest}<span class="small" class:danger-text={!aiTest.ok} class:ok-text={aiTest.ok}>{aiTest.ok ? "Works: " : ""}{aiTest.text}</span>{/if}
            </div>
          {/if}
        </div>
      </section>

      <section class="setting" aria-labelledby="learning">
        <header>
          <h2 id="learning">Learning</h2>
          <p>Hams learns your words from corrections. It only ever suggests; you approve each one in Dictionary.</p>
        </header>
        <div>
          <label class="switch-row">
            <span class="text"><strong>Learn from edits in History</strong><span>When you fix a dictation in History, the change becomes a suggestion.</span></span>
            <input class="switch" type="checkbox" checked={s.learn_from_edits} onchange={(e) => app.save({ learn_from_edits: e.currentTarget.checked })} />
          </label>
          <label class="switch-row">
            <span class="text">
              <strong>Learn from edits in other apps</strong>
              <span>
                {app.platform === "macos"
                  ? "After a paste, Hams re-reads that text field for up to a minute. If you correct a word it wrote, it suggests the fix. Password fields are never read, and the field's text is never saved."
                  : "Coming to Windows later. On Windows, fix dictations in History instead."}
              </span>
            </span>
            <input class="switch" type="checkbox" checked={s.learn_from_fields} disabled={app.platform !== "macos"} onchange={(e) => app.save({ learn_from_fields: e.currentTarget.checked })} />
          </label>
        </div>
      </section>

      <section class="setting" aria-labelledby="updates">
        <header>
          <h2 id="updates">Updates</h2>
          <p>New versions are signed, and Hams checks the signature before installing anything.</p>
        </header>
        <div class="stack wide-gap">
          <div class="panel version">
            <div class="grow">
              {#if app.update}
                <strong>Hams {app.update.version} is available</strong>
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
          <label class="switch-row">
            <span class="text"><strong>Check for updates automatically</strong><span>Hams only downloads an update when you click Install.</span></span>
            <input class="switch" type="checkbox" checked={s.check_updates} onchange={(e) => app.save({ check_updates: e.currentTarget.checked })} />
          </label>
        </div>
      </section>

      {#if permissions?.applicable}
        <section class="setting" aria-labelledby="permissions">
          <header>
            <h2 id="permissions">Permissions</h2>
            <p>macOS asks for these once. Signing Hams with the local certificate keeps them across updates.</p>
          </header>
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
        </section>
      {/if}
    </div>
  </div>
{/if}

<style>
  .wide-gap { gap: 18px; }
  .block { display: block; }
  .keys { min-height: 22px; }
  .models li { display: flex; align-items: center; gap: 12px; padding-top: 10px; padding-bottom: 10px; }
  .models li.selected { background: var(--raised); }
  .model { display: flex; align-items: flex-start; gap: 10px; cursor: pointer; }
  .model input { margin-top: 4px; }
  .model strong, .choice strong, .label { font-weight: 500; }
  .progress { display: flex; align-items: center; gap: 8px; }
  .progress progress { width: 100px; }
  .choices { display: grid; gap: 6px; }
  .choice { display: flex; align-items: flex-start; gap: 8px; padding: 10px 12px; border: 1px solid var(--border); border-radius: var(--radius-card); cursor: pointer; }
  .choice input { margin-top: 3px; }
  .choice.selected { border-color: var(--text); }
  .joined { display: flex; }
  .joined input { border-top-right-radius: 0; border-bottom-right-radius: 0; }
  .joined button { height: auto; border-top-left-radius: 0; border-bottom-left-radius: 0; margin-left: -1px; }
  .ok-text { color: var(--data); }
  .version { display: flex; align-items: center; gap: 16px; padding: 14px 16px; }
  .version strong { font-weight: 600; }
</style>
