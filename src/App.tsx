import { useState, useEffect, useLayoutEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Check, Info, Keyboard, Minus, Square, X } from "lucide-react";
import {
  AppSettings,
  AppStatus,
  AudioDiagnostics,
  DictationRecord,
  InsightsSummary,
  ActiveTab,
} from "./types";
import { Header } from "./components/Header";
import { DictationStudio } from "./components/DictationStudio";
import { TranscriptCard } from "./components/TranscriptCard";
import { RecentDictations } from "./components/RecentDictations";
import { HistoryStream } from "./components/HistoryStream";
import { InsightsPanel } from "./components/InsightsPanel";
import { VocabularyManager } from "./components/VocabularyManager";
import { SettingsPanel } from "./components/SettingsPanel";
import { AccessibilityGuide } from "./components/AccessibilityGuide";

function DynamicIsland() {
  const [status, setStatus] = useState<AppStatus>({
    state_name: "Idle",
    recording_mode: null,
    last_dictation: "",
    error_message: null,
    delivery_message: null,
  });
  const [audioLevel, setAudioLevel] = useState(0);
  const [errorExpanded, setErrorExpanded] = useState(false);
  useLayoutEffect(() => {
    document.body.classList.add("overlay-window");
    return () => document.body.classList.remove("overlay-window");
  }, []);
  useEffect(() => {
    invoke<AppStatus>("get_app_status")
      .then(setStatus)
      .catch(() => undefined);
    const unlistenStatus = listen<AppStatus>("status-changed", (event) =>
      setStatus(event.payload),
    );
    const unlistenLevel = listen<number>("audio-level", (event) =>
      setAudioLevel(event.payload),
    );
    // Rust keeps the native overlay hidden until this renderer has applied the
    // overlay-specific document class and produced the black idle pill.
    void invoke("overlay_ready");
    return () => {
      unlistenStatus.then((fn) => fn());
      unlistenLevel.then((fn) => fn());
    };
  }, []);
  const recording = status.state_name === "Recording";
  const busy = ["Transcribing Audio", "Pasting"].includes(status.state_name);
  const hasError = Boolean(status.error_message);
  useEffect(() => {
    if (!status.error_message) {
      setErrorExpanded(false);
      return;
    }
    setErrorExpanded(true);
    const timer = window.setTimeout(() => setErrorExpanded(false), 5000);
    return () => window.clearTimeout(timer);
  }, [status.error_message]);
  if (!recording && !busy && (!hasError || !errorExpanded)) {
    return (
      <main className="overlay-root">
        <section
          className="overlay-pill is-idle"
          aria-label="Wisper ready for dictation"
        />
      </main>
    );
  }
  const visibleLevel = Math.min(1, Math.max(0, audioLevel * 6.5));
  const errorDetail = (status.error_message ?? "Something went wrong.")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, 78);
  return (
    <main className="overlay-root">
      <section
        className={`overlay-pill ${recording ? "is-recording" : ""} ${busy ? "is-busy" : ""} ${hasError ? "has-error" : ""}`}
        aria-label={
          recording
            ? "Recording dictation"
            : busy
              ? "Processing dictation"
              : "Wisper error"
        }
      >
        {recording && (
          <>
            <button
              className="overlay-action cancel"
              type="button"
              onClick={() =>
                invoke("trigger_cancel_dictation").catch(() => undefined)
              }
              aria-label="Discard recording"
              title="Discard recording"
            >
              <X />
            </button>
            <div
              className="overlay-wave"
              aria-label="Live voice modulation level"
            >
              {Array.from({ length: 9 }).map((_, index) => {
                const contour = 0.35 + Math.sin((index + 1) * 1.1) ** 2 * 0.65;
                const height = Math.max(
                  3,
                  Math.min(22, 4 + visibleLevel * contour * 22),
                );
                const opacity = 0.45 + visibleLevel * 0.55;
                return (
                  <i
                    key={index}
                    className="overlay-bar"
                    style={{ height: `${height}px`, opacity }}
                  />
                );
              })}
            </div>
            <button
              className="overlay-action finish"
              type="button"
              onClick={() =>
                invoke("trigger_stop_dictation").catch(() => undefined)
              }
              aria-label="Finish recording"
              title="Finish recording"
            >
              <Check />
            </button>
          </>
        )}
        {busy && (
          <div
            className="overlay-processing-wave"
            aria-label="Processing dictation"
          >
            {Array.from({ length: 7 }).map((_, index) => (
              <i
                key={index}
                className="overlay-processing-bar"
                style={{ animationDelay: `${index * 90}ms` }}
              />
            ))}
          </div>
        )}
        {hasError && (
          <div className="overlay-error-container error-expanded">
            <Info className="overlay-error" />
            <div>
              <strong>Couldn&apos;t finish dictation</strong>
              <span>{errorDetail}</span>
            </div>
          </div>
        )}
      </section>
    </main>
  );
}

function WindowTitleBar({ title }: { title: string }) {
  const window = getCurrentWindow();
  const runWindowAction = (action: string, operation: () => Promise<void>) => {
    operation().catch((error) => {
      invoke("report_window_action_failure", {
        action,
        details: String(error),
      }).catch(() => undefined);
    });
  };
  return (
    <header className="window-titlebar">
      <div className="titlebar-drag" data-tauri-drag-region>
        <div className="wordmark">wisper</div>
        <span>{title}</span>
      </div>
      <div className="window-actions">
        <button
          type="button"
          aria-label="Minimize window"
          title="Minimize"
          onPointerDown={(e) => e.stopPropagation()}
          onMouseDown={(e) => e.stopPropagation()}
          onClick={() => runWindowAction("minimize", () => window.minimize())}
        >
          <Minus />
        </button>
        <button
          type="button"
          aria-label="Maximize or restore window"
          title="Maximize or restore"
          onPointerDown={(e) => e.stopPropagation()}
          onMouseDown={(e) => e.stopPropagation()}
          onClick={() =>
            runWindowAction("maximize or restore", () =>
              window.toggleMaximize(),
            )
          }
        >
          <Square />
        </button>
        <button
          type="button"
          className="close-window"
          aria-label="Close window"
          title="Close"
          onPointerDown={(e) => e.stopPropagation()}
          onMouseDown={(e) => e.stopPropagation()}
          onClick={() => runWindowAction("close", () => window.close())}
        >
          <X />
        </button>
      </div>
    </header>
  );
}

export default function App() {
  if (getCurrentWindow().label === "island") return <DynamicIsland />;
  const [settings, setSettings] = useState<AppSettings>({
    gemini_api_key: "",
    gemini_model: "gemini-2.5-flash",
    custom_dictionary:
      "Tauri, Rust, Gemini, Whisper, cpal, rdev, enigo, arboard",
    whisper_model_path: "ggml-small.bin",
    whisper_binary_path: "whisper.cpp",
    audio_gain_factor: 2.5,
    hotkey_combo: "Ctrl+Shift+Space",
    transcription_language: "auto",
    delivery_mode: "copy_and_paste",
    app_usage_tracking: false,
  });
  const [status, setStatus] = useState<AppStatus>({
    state_name: "Idle",
    recording_mode: null,
    last_dictation: "",
    error_message: null,
    delivery_message: null,
  });
  const [audio, setAudio] = useState<AudioDiagnostics>({
    device_name: "Detecting microphone…",
    native_sample_rate: 0,
    channels: 0,
    recorded_duration_secs: 0,
    peak_amplitude: 0,
    rms_amplitude: 0,
    is_clipping: false,
    is_recording: false,
    message: null,
  });
  const [history, setHistory] = useState<DictationRecord[]>([]);
  const [insights, setInsights] = useState<InsightsSummary | null>(null);
  const [activeTab, setActiveTab] = useState<ActiveTab>("studio");
  const [savedSuccess, setSavedSuccess] = useState(false);
  const [copiedText, setCopiedText] = useState(false);
  const [copyError, setCopyError] = useState<string | null>(null);
  useEffect(() => {
    invoke<AppSettings>("get_settings")
      .then(setSettings)
      .catch(() => undefined);
    invoke<AppStatus>("get_app_status")
      .then(setStatus)
      .catch(() => undefined);
    invoke<DictationRecord[]>("get_dictation_history")
      .then(setHistory)
      .catch(() => undefined);
    invoke<InsightsSummary>("get_insights_summary")
      .then(setInsights)
      .catch(() => undefined);
    const refresh = () =>
      invoke<AudioDiagnostics>("get_audio_diagnostics")
        .then(setAudio)
        .catch(() => undefined);
    const refreshHistory = () => {
      invoke<DictationRecord[]>("get_dictation_history")
        .then(setHistory)
        .catch((error) =>
          setCopyError(`Could not refresh history: ${String(error)}`),
        );
      invoke<InsightsSummary>("get_insights_summary")
        .then(setInsights)
        .catch(() => undefined);
    };
    refresh();
    const timer = window.setInterval(refresh, 3000);
    const unlisten = listen<AppStatus>("status-changed", (event) => {
      setStatus(event.payload);
      if (event.payload.last_dictation) refreshHistory();
    });
    return () => {
      unlisten.then((fn) => fn());
      window.clearInterval(timer);
    };
  }, []);
  const save = async () => {
    try {
      await invoke("save_settings", { settings });
      setCopyError(null);
      setSavedSuccess(true);
      setTimeout(() => setSavedSuccess(false), 3000);
    } catch (error) {
      setSavedSuccess(false);
      setCopyError(`Could not save settings: ${String(error)}`);
    }
  };
  const toggle = async () => {
    try {
      await invoke(
        status.state_name === "Recording"
          ? "trigger_stop_dictation"
          : "trigger_start_dictation",
      );
      setCopyError(null);
    } catch (error) {
      setCopyError(
        `Could not ${status.state_name === "Recording" ? "stop" : "start"} dictation: ${String(error)}`,
      );
    }
  };
  const copy = async (text: string) => {
    try {
      await invoke<string>("copy_to_clipboard", { text });
      setCopyError(null);
      setCopiedText(true);
      setTimeout(() => setCopiedText(false), 2000);
    } catch (error) {
      setCopiedText(false);
      setCopyError(`Could not copy to system clipboard: ${String(error)}`);
    }
  };
  const paste = async () => {
    try {
      const message = await invoke<string>("paste_last_transcript");
      setCopyError(null);
      setStatus((current) => ({ ...current, delivery_message: message }));
    } catch (error) {
      setCopyError(`Could not paste: ${String(error)}`);
    }
  };
  const pasteText = async (text: string) => {
    try {
      const message = await invoke<string>("paste_text", { text });
      setCopyError(null);
      setStatus((current) => ({ ...current, delivery_message: message }));
    } catch (error) {
      setCopyError(`Could not paste saved text: ${String(error)}`);
    }
  };
  const titles: Record<ActiveTab, string> = {
    studio: "Dictation",
    history: "History",
    insights: "Insights",
    vocabulary: "Vocabulary",
    settings: "Settings",
    guide: "Help",
  };
  const wordCount = history.reduce(
    (total, item) =>
      total + item.text.trim().split(/\s+/).filter(Boolean).length,
    0,
  );
  return (
    <div className="app-frame">
      <WindowTitleBar title={titles[activeTab]} />
      <div className="app-layout">
        <Header
          activeTab={activeTab}
          setActiveTab={setActiveTab}
          audio={audio}
          historyCount={history.length}
          hotkey={settings.hotkey_combo}
        />
        <main className="app-content">
          {activeTab === "studio" && (
            <div className="workspace-home">
              <header className="workspace-intro">
                <p>VOICE WORKSPACE</p>
                <h1>Write without breaking your flow.</h1>
                <span>
                  Dictate locally, then Wisper safely delivers the words to the
                  input you selected.
                </span>
              </header>
              <div className="workspace-grid">
                <div className="workspace-primary">
                  <DictationStudio
                    onToggleDictation={toggle}
                    hotkey={settings.hotkey_combo}
                    isRecording={status.state_name === "Recording"}
                  />
                  <TranscriptCard
                    status={status}
                    onCopy={copy}
                    onPaste={paste}
                    copiedText={copiedText}
                    copyError={copyError}
                  />
                  <RecentDictations
                    history={history}
                    onOpenHistory={() => setActiveTab("history")}
                  />
                </div>
                <aside className="workspace-rail">
                  <section>
                    <p>THIS WORKSPACE</p>
                    <strong>{history.length}</strong>
                    <span>completed dictations</span>
                    <strong>{wordCount.toLocaleString()}</strong>
                    <span>words captured</span>
                  </section>
                  <section className="overlay-note">
                    <Keyboard />
                    <div>
                      <h2>Overlay-led dictation</h2>
                      <p>
                        Press <kbd>Ctrl + Shift + Space</kbd>. Recording,
                        processing, and delivery stay in the compact overlay.
                      </p>
                    </div>
                  </section>
                </aside>
              </div>
            </div>
          )}
          {activeTab === "history" && (
            <HistoryStream
              history={history}
              onCopy={copy}
              onPasteText={pasteText}
            />
          )}
          {activeTab === "insights" && (
            <InsightsPanel
              summary={insights}
              history={history}
              trackingEnabled={settings.app_usage_tracking}
              onSummaryChange={setInsights}
            />
          )}
          {activeTab === "vocabulary" && (
            <VocabularyManager
              customDictionary={settings.custom_dictionary}
              onUpdateDictionary={(custom_dictionary) =>
                setSettings({ ...settings, custom_dictionary })
              }
              onSave={save}
              savedSuccess={savedSuccess}
            />
          )}
          {activeTab === "settings" && (
            <SettingsPanel
              settings={settings}
              onUpdateSettings={setSettings}
              onSave={save}
              savedSuccess={savedSuccess}
            />
          )}
          {activeTab === "guide" && <AccessibilityGuide />}
        </main>
      </div>
    </div>
  );
}
