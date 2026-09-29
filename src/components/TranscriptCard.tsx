import React, { useState } from "react";
import {
  Sparkles,
  Clipboard,
  Check,
  Send,
  Edit2,
  Save,
  X,
  MessageSquare,
  AlertCircle,
} from "lucide-react";
import { AppStatus } from "../types";
import { holdHotkeyLabel } from "../platform";

interface TranscriptCardProps {
  status: AppStatus;
  onCopy: (text: string) => void;
  onPaste: () => void;
  copiedText: boolean;
  copyError: string | null;
}

export const TranscriptCard: React.FC<TranscriptCardProps> = ({
  status,
  onCopy,
  onPaste,
  copiedText,
  copyError,
}) => {
  const [isEditing, setIsEditing] = useState(false);
  const [editedText, setEditedText] = useState("");

  const rawText = status.last_dictation || "";
  const wordCount = rawText ? rawText.trim().split(/\s+/).length : 0;
  const charCount = rawText.length;

  const handleStartEdit = () => {
    setEditedText(rawText);
    setIsEditing(true);
  };

  const handleSaveEdit = () => {
    onCopy(editedText);
    setIsEditing(false);
  };

  return (
    <section className="glass-panel transcript-card">
      {/* Header Bar */}
      <div className="transcript-header">
        <div className="transcript-title">
          <div className="transcript-icon">
            <Sparkles />
          </div>
          <div>
            <h3>Latest transcript</h3>
            <p>Ready to copy or send to your active app.</p>
          </div>
        </div>

        {rawText && (
          <div className="transcript-actions">
            {!isEditing ? (
              <>
                <button
                  onClick={handleStartEdit}
                  className="secondary-action"
                  title="Edit dictation text"
                >
                  <Edit2 />
                  Edit
                </button>

                <button
                  onClick={onPaste}
                  className="secondary-action"
                  title="Paste directly into current focused window"
                >
                  <Send />
                  Paste Again
                </button>

                <button
                  onClick={() => onCopy(rawText)}
                  className={`primary-action ${copiedText ? "is-complete" : ""}`}
                >
                  {copiedText ? (
                    <Check />
                  ) : (
                    <Clipboard />
                  )}
                  {copiedText ? "Copied!" : "Copy Text"}
                </button>
              </>
            ) : (
              <>
                <button
                  onClick={() => setIsEditing(false)}
                  className="secondary-action"
                >
                  <X />
                  Cancel
                </button>
                <button
                  onClick={handleSaveEdit}
                  className="primary-action"
                >
                  <Save />
                  Save & Copy
                </button>
              </>
            )}
          </div>
        )}
      </div>

      {/* Main Text Content Window */}
      <div className="transcript-body">
        {status.error_message ? (
          <div className="transcript-error">
            <AlertCircle />
            <p>
              Dictation failed or was interrupted: {status.error_message}. Please check audio input level and try dictating again.
            </p>
          </div>
        ) : isEditing ? (
          <textarea
            value={editedText}
            onChange={(e) => setEditedText(e.target.value)}
            rows={4}
            className="transcript-editor"
            autoFocus
          />
        ) : rawText ? (
          <p className="transcript-text">
            {rawText}
          </p>
        ) : (
          <div className="transcript-empty">
            <MessageSquare />
            <p>
              No dictation recorded in this session yet.
              <br />
              Hold <strong>{holdHotkeyLabel}</strong> anywhere to speak.
            </p>
          </div>
        )}
      </div>

      {/* Delivery Message & Stats Footer */}
      <footer className="transcript-footer">
        <div>
          {status.delivery_message ? (
            <span className="delivery-status">
              <Check />
              {status.delivery_message}
            </span>
          ) : (
            <span className="delivery-placeholder">
              Delivery: Auto Paste into focused app window
            </span>
          )}
        </div>

        {rawText && (
          <div className="transcript-stats">
            <span>{wordCount} words</span>
            <span>•</span>
            <span>{charCount} chars</span>
          </div>
        )}
      </footer>

      {copyError && (
        <div className="copy-error">
          {copyError}
        </div>
      )}
    </section>
  );
};
