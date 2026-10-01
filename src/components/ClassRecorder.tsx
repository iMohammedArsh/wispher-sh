import { useEffect, useMemo, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  AudioLines,
  BookOpenCheck,
  ChevronRight,
  CircleStop,
  Clock3,
  FileText,
  LoaderCircle,
  Mic2,
} from "lucide-react";
import { ClassRecordingStatus, ClassSession } from "../types";
import { isTauriRuntime } from "../tauri";

const idleRecording: ClassRecordingStatus = {
  is_recording: false,
  started_at_ms: null,
  title: null,
};

function formatDuration(seconds: number) {
  const total = Math.max(0, Math.floor(seconds));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const remaining = total % 60;
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${String(remaining).padStart(2, "0")}`
    : `${minutes}:${String(remaining).padStart(2, "0")}`;
}

export function ClassRecorder() {
  const [sessions, setSessions] = useState<ClassSession[]>([]);
  const [recording, setRecording] =
    useState<ClassRecordingStatus>(idleRecording);
  const [title, setTitle] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isStopping, setIsStopping] = useState(false);
  const [now, setNow] = useState(Date.now());

  useEffect(() => {
    if (!isTauriRuntime) return;
    invoke<ClassSession[]>("get_class_sessions")
      .then((items) => {
        setSessions(items);
        setSelectedId((current) => current ?? items[0]?.id ?? null);
      })
      .catch((reason) => setError(String(reason)));
    invoke<ClassRecordingStatus>("get_class_recording_status")
      .then(setRecording)
      .catch(() => undefined);
    const unlistenSessions = listen<ClassSession[]>(
      "class-sessions-changed",
      (event) => {
        setSessions(event.payload);
        setSelectedId((current) => current ?? event.payload[0]?.id ?? null);
      },
    );
    const unlistenRecording = listen<ClassRecordingStatus>(
      "class-recording-changed",
      (event) => setRecording(event.payload),
    );
    return () => {
      unlistenSessions.then((unlisten) => unlisten());
      unlistenRecording.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    if (!recording.is_recording) return;
    setNow(Date.now());
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [recording.is_recording]);

  const elapsed = recording.started_at_ms
    ? (now - recording.started_at_ms) / 1000
    : 0;
  const selected = useMemo(
    () => sessions.find((session) => session.id === selectedId) ?? sessions[0],
    [sessions, selectedId],
  );

  const start = async () => {
    try {
      const next = await invoke<ClassRecordingStatus>("start_class_recording", {
        title,
      });
      setRecording(next);
      setError(null);
    } catch (reason) {
      setError(String(reason));
    }
  };

  const stop = async () => {
    if (isStopping) return;
    setIsStopping(true);
    try {
      const session = await invoke<ClassSession>("stop_class_recording");
      setRecording(idleRecording);
      setSelectedId(session.id);
      setTitle("");
      setError(null);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setIsStopping(false);
    }
  };

  return (
    <div className="class-workspace">
      <header className="class-hero">
        <div>
          <p className="section-eyebrow">CLASS RECORDER</p>
          <h1>Listen now. Study later.</h1>
          <span>
            Save the entire class audio, then turn it into a transcript and
            clear study notes.
          </span>
        </div>
        <div className={`class-capture ${recording.is_recording ? "is-live" : ""}`}>
          <div className="class-capture-copy">
            <span>{recording.is_recording ? "RECORDING CLASS" : "NEW RECORDING"}</span>
            <strong>
              {recording.is_recording
                ? recording.title || "Untitled class"
                : "Capture a lecture"}
            </strong>
            {recording.is_recording ? (
              <time>{formatDuration(elapsed)}</time>
            ) : (
              <input
                value={title}
                onChange={(event) => setTitle(event.target.value)}
                placeholder="Class name, e.g. Biology 101"
                maxLength={100}
                aria-label="Class name"
              />
            )}
          </div>
          <button
            type="button"
            className="class-record-button"
            onClick={recording.is_recording ? stop : start}
            disabled={isStopping}
          >
            {isStopping ? <LoaderCircle className="class-spinner" /> : recording.is_recording ? <CircleStop /> : <Mic2 />}
            {isStopping
              ? "Saving recording…"
              : recording.is_recording
                ? "Finish & make notes"
                : "Start recording"}
          </button>
        </div>
        {error && <div className="class-error">{error}</div>}
      </header>

      <div className="class-library">
        <aside className="class-session-list">
          <div className="class-list-heading">
            <span>YOUR CLASSES</span>
            <em>{sessions.length}</em>
          </div>
          {sessions.length === 0 ? (
            <div className="class-empty">
              <AudioLines />
              <strong>No classes yet</strong>
              <span>Your saved recordings and notes will appear here.</span>
            </div>
          ) : (
            sessions.map((session) => (
              <button
                type="button"
                key={session.id}
                className={selected?.id === session.id ? "selected" : ""}
                onClick={() => setSelectedId(session.id)}
              >
                <span className="class-list-icon"><BookOpenCheck /></span>
                <span>
                  <strong>{session.title}</strong>
                  <small>
                    {new Date(session.created_at_ms).toLocaleDateString(undefined, {
                      month: "short",
                      day: "numeric",
                    })}
                    {" · "}
                    {formatDuration(session.duration_secs)}
                  </small>
                </span>
                {session.status === "processing" ? (
                  <LoaderCircle className="class-spinner" />
                ) : (
                  <ChevronRight />
                )}
              </button>
            ))
          )}
        </aside>

        <section className="class-detail">
          {!selected ? (
            <div className="class-detail-empty">
              <BookOpenCheck />
              <h2>Your class notebook</h2>
              <p>Start a recording to build your first set of notes.</p>
            </div>
          ) : (
            <>
              <div className="class-detail-heading">
                <div>
                  <p className="section-eyebrow">SAVED CLASS</p>
                  <h2>{selected.title}</h2>
                  <span><Clock3 /> {formatDuration(selected.duration_secs)} of audio</span>
                </div>
                <span className={`class-state ${selected.status}`}>
                  {selected.status === "processing" ? "Making notes…" : selected.status}
                </span>
              </div>

              {isTauriRuntime && (
                <div className="class-player">
                  <div><AudioLines /><span>Full class audio</span></div>
                  <audio
                    key={selected.audio_path}
                    controls
                    preload="metadata"
                    src={convertFileSrc(selected.audio_path)}
                  />
                </div>
              )}

              {selected.error_message && (
                <div className="class-error">{selected.error_message}</div>
              )}

              {selected.status === "processing" ? (
                <div className="class-processing">
                  <LoaderCircle className="class-spinner" />
                  <strong>Creating your study material</strong>
                  <span>The audio is already saved. You can leave this page while Hams transcribes it.</span>
                </div>
              ) : (
                <>
                  <article className="class-notes">
                    <div><BookOpenCheck /><h3>Study notes</h3></div>
                    <pre>{selected.notes || "Notes could not be generated for this class."}</pre>
                  </article>
                  <details className="class-transcript">
                    <summary><FileText /> Entire class transcript</summary>
                    <p>{selected.transcript || "No transcript is available."}</p>
                  </details>
                </>
              )}
            </>
          )}
        </section>
      </div>
    </div>
  );
}
