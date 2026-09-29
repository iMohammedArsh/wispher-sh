import React from "react";
import { Keyboard, Mic } from "lucide-react";
import { platformName } from "../platform";

interface DictationStudioProps {
  onToggleDictation: () => void;
  hotkey: string;
  isRecording: boolean;
}

export const DictationStudio: React.FC<DictationStudioProps> = ({
  onToggleDictation,
  hotkey,
  isRecording,
}) => {
  return (
    <section className="dictation-studio">
      <div className="studio-topline">
        <div>
          <p className="section-eyebrow">Ready when you are</p>
          <h1>
            Speak naturally.
            <br />
            Stay in the moment.
          </h1>
        </div>
        <span className="mode-chip">Local transcription</span>
      </div>
      <div
        className={`record-surface quiet-record-surface ${isRecording ? "recording" : ""}`}
      >
        <button
          className="record-button"
          onClick={onToggleDictation}
          aria-label={isRecording ? "Finish dictation" : "Start dictation"}
        >
          <Mic />
        </button>
        <strong>{isRecording ? "Listening now" : "Start a dictation"}</strong>
        <p>
          {isRecording
            ? "Use the overlay controls to finish or discard."
            : `Use Hams from any selected input on ${platformName}.`}
        </p>
        <span className="key-hint">
          <Keyboard /> {hotkey}
        </span>
      </div>
    </section>
  );
};
