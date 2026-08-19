import React, { useState } from "react";
import {
  BookOpen,
  Plus,
  X,
  CheckCircle2,
  Sparkles,
  Tag,
  FolderPlus,
} from "lucide-react";
import { PresetVocabulary } from "../types";

interface VocabularyManagerProps {
  customDictionary: string;
  onUpdateDictionary: (dict: string) => void;
  onSave: () => void;
  savedSuccess: boolean;
}

export const VocabularyManager: React.FC<VocabularyManagerProps> = ({
  customDictionary,
  onUpdateDictionary,
  onSave,
  savedSuccess,
}) => {
  const [newWordInput, setNewWordInput] = useState("");

  const words = customDictionary
    .split(",")
    .map((w) => w.trim())
    .filter(Boolean);

  const handleAddWord = (wordToAdd?: string) => {
    const term = (wordToAdd || newWordInput).trim();
    if (!term) return;
    if (words.some((w) => w.toLowerCase() === term.toLowerCase())) {
      setNewWordInput("");
      return;
    }
    const updated = [...words, term].join(", ");
    onUpdateDictionary(updated);
    setNewWordInput("");
  };

  const handleRemoveWord = (wordToRemove: string) => {
    const updated = words.filter((w) => w !== wordToRemove).join(", ");
    onUpdateDictionary(updated);
  };

  // Preset Vocabulary Collections
  const presets: PresetVocabulary[] = [
    {
      id: "web-dev",
      name: "Fullstack Web Dev",
      description: "React, Vite, Next.js, TypeScript, Tailwind, GraphQL, REST",
      words: ["Tauri", "React", "TypeScript", "Vite", "Tailwind", "GraphQL", "Node.js", "Zustand", "Prisma"],
    },
    {
      id: "rust-sys",
      name: "Systems & Rust",
      description: "Cargo, Tokio, Serde, CPAL, Enigo, Arboard, Mutex, Arc",
      words: ["Rust", "Cargo", "Tokio", "Serde", "cpal", "rdev", "enigo", "arboard", "Mutex", "Arc"],
    },
    {
      id: "ai-ml",
      name: "AI & Machine Learning",
      description: "Gemini, Whisper, LLM, PyTorch, Embeddings, Transcribe",
      words: ["Gemini", "Whisper", "LLM", "PyTorch", "Transformers", "Embeddings", "FastAPI", "Ollama"],
    },
    {
      id: "business",
      name: "Product & Agile",
      description: "Sprint, Backlog, PRD, ROI, KPI, OKR, Roadmap",
      words: ["Sprint", "Backlog", "PRD", "ROI", "KPI", "OKR", "Roadmap", "Deployment"],
    },
  ];

  const handleApplyPreset = (presetWords: string[]) => {
    const merged = Array.from(new Set([...words, ...presetWords])).join(", ");
    onUpdateDictionary(merged);
  };

  return (
    <div className="glass-panel rounded-3xl p-6 border border-slate-800/80 shadow-2xl space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 border-b border-slate-800/80 pb-4">
        <div>
          <h3 className="text-base font-bold text-slate-100 flex items-center gap-2">
            <BookOpen className="w-4 h-4 text-purple-400" />
            Custom Dictionary & Domain Vocabulary
          </h3>
          <p className="text-xs text-slate-400">
            Guide Gemini AI to accurately spell custom project jargon, acronyms, and proper nouns.
          </p>
        </div>

        <button
          onClick={onSave}
          className="flex items-center justify-center gap-2 px-5 py-2.5 rounded-xl bg-purple-600 hover:bg-purple-500 text-white font-semibold text-xs transition-all shadow-md shadow-purple-950 cursor-pointer"
        >
          <CheckCircle2 className="w-4 h-4" />
          {savedSuccess ? "Saved Vocabulary!" : "Save Vocabulary"}
        </button>
      </div>

      {/* Add Single Word Input */}
      <div className="flex gap-2">
        <div className="relative flex-1">
          <Tag className="w-4 h-4 text-slate-400 absolute left-3.5 top-3" />
          <input
            type="text"
            value={newWordInput}
            onChange={(e) => setNewWordInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                handleAddWord();
              }
            }}
            placeholder="Type a new word/acronym and press Enter (e.g. OpenAI, Kubernetes, Postgres)..."
            className="w-full bg-slate-950/70 border border-slate-800 rounded-xl pl-10 pr-4 py-2.5 text-xs text-slate-200 focus:outline-none focus:border-purple-500 font-mono"
          />
        </div>
        <button
          onClick={() => handleAddWord()}
          className="flex items-center gap-1.5 px-4 py-2.5 bg-purple-950 hover:bg-purple-900 border border-purple-800/60 text-purple-300 rounded-xl text-xs font-semibold cursor-pointer transition-colors"
        >
          <Plus className="w-4 h-4" />
          Add Term
        </button>
      </div>

      {/* Active Vocabulary Tag Cloud */}
      <div className="space-y-3">
        <div className="flex items-center justify-between text-xs text-slate-400">
          <span className="font-semibold uppercase tracking-wider text-[11px] text-slate-400">
            Active Dictionary Words ({words.length})
          </span>
          <span className="text-[11px] text-slate-500 font-mono">
            Click 'X' to remove a term
          </span>
        </div>

        <div className="bg-slate-950/70 border border-slate-800/80 rounded-2xl p-4 min-h-[120px] flex flex-wrap gap-2 items-start">
          {words.length === 0 ? (
            <p className="text-slate-500 text-xs italic">
              No custom terms added yet. Add terms above or select a preset pack below.
            </p>
          ) : (
            words.map((word, idx) => (
              <span
                key={idx}
                className="inline-flex items-center gap-1.5 px-3 py-1 bg-purple-950/50 hover:bg-purple-900/60 border border-purple-800/50 rounded-xl text-xs text-purple-200 font-mono transition-colors shadow-sm"
              >
                <span>{word}</span>
                <button
                  onClick={() => handleRemoveWord(word)}
                  className="text-purple-400 hover:text-red-400 transition-colors cursor-pointer"
                  title={`Remove ${word}`}
                >
                  <X className="w-3.5 h-3.5" />
                </button>
              </span>
            ))
          )}
        </div>
      </div>

      {/* Preset Packs Section */}
      <div className="border-t border-slate-800/80 pt-4 space-y-3">
        <h4 className="text-xs font-bold text-slate-300 uppercase tracking-wider flex items-center gap-1.5">
          <FolderPlus className="w-3.5 h-3.5 text-indigo-400" />
          Quick Add Preset Packs
        </h4>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          {presets.map((pack) => (
            <div
              key={pack.id}
              className="bg-slate-950/50 border border-slate-800/80 hover:border-slate-700 rounded-2xl p-4 flex flex-col justify-between gap-3 transition-all"
            >
              <div>
                <h5 className="text-xs font-bold text-slate-200 flex items-center gap-1.5">
                  <Sparkles className="w-3.5 h-3.5 text-purple-400" />
                  {pack.name}
                </h5>
                <p className="text-[11px] text-slate-400 mt-0.5 leading-relaxed">
                  {pack.description}
                </p>
              </div>

              <button
                onClick={() => handleApplyPreset(pack.words)}
                className="self-start flex items-center gap-1.5 px-3 py-1.5 bg-indigo-950/60 hover:bg-indigo-900/80 border border-indigo-800/50 rounded-xl text-xs font-semibold text-indigo-300 transition-colors cursor-pointer"
              >
                <Plus className="w-3.5 h-3.5" />
                Import Preset Terms
              </button>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
