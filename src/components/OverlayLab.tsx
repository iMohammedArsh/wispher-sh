import { useEffect, useMemo, useState, type CSSProperties } from "react";
import {
  AudioLines,
  Check,
  CircleAlert,
  Copy,
  LoaderCircle,
  RotateCcw,
  Sparkles,
  X,
} from "lucide-react";

type PreviewState = "recording" | "processing" | "success" | "error";
type PreviewStyle = "aurora" | "ink" | "frost";
type PreviewSize = "compact" | "balanced" | "bold";
type PreviewPosition = "top" | "bottom";
type PreviewBackdrop = "light" | "dark" | "color";

interface OverlayDraft {
  state: PreviewState;
  style: PreviewStyle;
  size: PreviewSize;
  position: PreviewPosition;
  backdrop: PreviewBackdrop;
  accent: string;
  showTimer: boolean;
  showActions: boolean;
}

const storageKey = "hams-overlay-lab-draft";
const defaultDraft: OverlayDraft = {
  state: "recording",
  style: "aurora",
  size: "balanced",
  position: "top",
  backdrop: "dark",
  accent: "mint",
  showTimer: true,
  showActions: true,
};

const accents = [
  { id: "mint", label: "Mint", color: "#75e6ad" },
  { id: "violet", label: "Violet", color: "#a995ff" },
  { id: "coral", label: "Coral", color: "#ff987d" },
  { id: "blue", label: "Blue", color: "#78b8ff" },
];

function loadDraft(): OverlayDraft {
  try {
    const saved = window.localStorage.getItem(storageKey);
    return saved ? { ...defaultDraft, ...JSON.parse(saved) } : defaultDraft;
  } catch {
    return defaultDraft;
  }
}

export function OverlayLab() {
  const [draft, setDraft] = useState<OverlayDraft>(loadDraft);
  const [copied, setCopied] = useState(false);
  const accent = accents.find((item) => item.id === draft.accent) ?? accents[0];

  useEffect(() => {
    window.localStorage.setItem(storageKey, JSON.stringify(draft));
  }, [draft]);

  const description = useMemo(
    () =>
      `${draft.style} style · ${draft.size} size · ${accent.label.toLowerCase()} accent · ${draft.position}`,
    [draft, accent.label],
  );

  const update = <Key extends keyof OverlayDraft>(
    key: Key,
    value: OverlayDraft[Key],
  ) => setDraft((current) => ({ ...current, [key]: value }));

  const copyDraft = async () => {
    await navigator.clipboard.writeText(
      `Hams overlay: ${description}; timer ${draft.showTimer ? "on" : "off"}; actions ${draft.showActions ? "on" : "off"}.`,
    );
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1800);
  };

  return (
    <div className="overlay-lab">
      <header className="overlay-lab-heading">
        <div>
          <p>DESIGN BEFORE DEVELOPMENT</p>
          <h1>Overlay Lab</h1>
          <span>
            Shape the experience here. Nothing on this page controls the real
            microphone or native overlay.
          </span>
        </div>
        <div className="overlay-lab-heading-actions">
          <button type="button" onClick={() => setDraft(defaultDraft)}>
            <RotateCcw /> Reset
          </button>
          <button type="button" className="primary" onClick={copyDraft}>
            {copied ? <Check /> : <Copy />} {copied ? "Copied" : "Copy choices"}
          </button>
        </div>
      </header>

      <section className="overlay-lab-layout">
        <div className="overlay-lab-preview-card">
          <div className="overlay-lab-card-title">
            <div>
              <span>LIVE MOCKUP</span>
              <strong>{description}</strong>
            </div>
            <em>Draft saves automatically</em>
          </div>
          <div
            className={`overlay-stage backdrop-${draft.backdrop} position-${draft.position}`}
            style={{ "--lab-accent": accent.color } as CSSProperties}
          >
            <div className="overlay-stage-window">
              <span />
              <span />
              <span />
              <div className="overlay-stage-line long" />
              <div className="overlay-stage-line" />
              <div className="overlay-stage-card" />
            </div>
            <div
              className={`lab-island style-${draft.style} size-${draft.size} state-${draft.state}`}
            >
              {draft.state === "recording" && (
                <>
                  {draft.showActions && (
                    <button type="button" aria-label="Discard preview">
                      <X />
                    </button>
                  )}
                  <div className="lab-island-mark"><AudioLines /></div>
                  <div className="lab-island-copy">
                    <strong><i /> Listening</strong>
                    {draft.showTimer && <span>00:18</span>}
                  </div>
                  <div className="lab-wave" aria-hidden="true">
                    {[8, 17, 25, 13, 29, 20, 10, 23, 15].map((height, index) => (
                      <i key={index} style={{ height }} />
                    ))}
                  </div>
                  {draft.showActions && (
                    <button type="button" className="confirm" aria-label="Finish preview">
                      <Check />
                    </button>
                  )}
                </>
              )}
              {draft.state === "processing" && (
                <>
                  <div className="lab-island-mark"><Sparkles /></div>
                  <div className="lab-island-copy grow">
                    <strong>Making it readable</strong>
                    <span>Transcribing locally</span>
                  </div>
                  <LoaderCircle className="lab-spin" />
                </>
              )}
              {draft.state === "success" && (
                <>
                  <div className="lab-island-mark success"><Check /></div>
                  <div className="lab-island-copy grow">
                    <strong>Text delivered</strong>
                    <span>Ready for your next thought</span>
                  </div>
                </>
              )}
              {draft.state === "error" && (
                <>
                  <div className="lab-island-mark error"><CircleAlert /></div>
                  <div className="lab-island-copy grow">
                    <strong>Couldn’t hear that</strong>
                    <span>Check the microphone and try again</span>
                  </div>
                </>
              )}
            </div>
          </div>
        </div>

        <aside className="overlay-lab-controls">
          <LabChoice
            label="Preview state"
            value={draft.state}
            choices={[
              ["recording", "Recording"],
              ["processing", "Processing"],
              ["success", "Success"],
              ["error", "Error"],
            ]}
            onChange={(value) => update("state", value as PreviewState)}
          />
          <LabChoice
            label="Visual direction"
            value={draft.style}
            choices={[
              ["aurora", "Aurora"],
              ["ink", "Ink"],
              ["frost", "Frost"],
            ]}
            onChange={(value) => update("style", value as PreviewStyle)}
          />
          <LabChoice
            label="Size"
            value={draft.size}
            choices={[
              ["compact", "Compact"],
              ["balanced", "Balanced"],
              ["bold", "Bold"],
            ]}
            onChange={(value) => update("size", value as PreviewSize)}
          />
          <div className="lab-control-group">
            <span>Accent</span>
            <div className="lab-swatches">
              {accents.map((item) => (
                <button
                  key={item.id}
                  type="button"
                  className={draft.accent === item.id ? "selected" : ""}
                  onClick={() => update("accent", item.id)}
                  aria-label={item.label}
                  title={item.label}
                >
                  <i style={{ background: item.color }} />
                </button>
              ))}
            </div>
          </div>
          <LabChoice
            label="Test background"
            value={draft.backdrop}
            choices={[
              ["light", "Light app"],
              ["dark", "Dark app"],
              ["color", "Wallpaper"],
            ]}
            onChange={(value) => update("backdrop", value as PreviewBackdrop)}
          />
          <LabChoice
            label="Position"
            value={draft.position}
            choices={[["top", "Top"], ["bottom", "Bottom"]]}
            onChange={(value) => update("position", value as PreviewPosition)}
          />
          <div className="lab-control-group lab-switches">
            <span>Details</span>
            <label>
              <input
                type="checkbox"
                checked={draft.showTimer}
                onChange={(event) => update("showTimer", event.target.checked)}
              />
              Recording timer
            </label>
            <label>
              <input
                type="checkbox"
                checked={draft.showActions}
                onChange={(event) => update("showActions", event.target.checked)}
              />
              Cancel and finish buttons
            </label>
          </div>
        </aside>
      </section>
    </div>
  );
}

function LabChoice({
  label,
  value,
  choices,
  onChange,
}: {
  label: string;
  value: string;
  choices: string[][];
  onChange: (value: string) => void;
}) {
  return (
    <div className="lab-control-group">
      <span>{label}</span>
      <div className="lab-segmented">
        {choices.map(([id, choiceLabel]) => (
          <button
            key={id}
            type="button"
            className={value === id ? "selected" : ""}
            onClick={() => onChange(id)}
          >
            {choiceLabel}
          </button>
        ))}
      </div>
    </div>
  );
}
