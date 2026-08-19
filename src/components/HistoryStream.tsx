import React, { useState } from "react";
import {
  History as HistoryIcon,
  Search,
  Clipboard,
  Check,
  Send,
  Pin,
  Clock,
  Zap,
} from "lucide-react";
import { DictationRecord } from "../types";

interface HistoryStreamProps {
  history: DictationRecord[];
  onCopy: (text: string) => void;
  onPasteText?: (text: string) => void;
}

export const HistoryStream: React.FC<HistoryStreamProps> = ({
  history,
  onCopy,
  onPasteText,
}) => {
  const [searchQuery, setSearchQuery] = useState("");
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [pinnedIds, setPinnedIds] = useState<Set<string>>(new Set());

  const handleCopyRecord = (id: string, text: string) => {
    onCopy(text);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 2000);
  };

  const togglePin = (id: string) => {
    setPinnedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const filteredHistory = history.filter((item) =>
    item.text.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const sortedHistory = [...filteredHistory].sort((a, b) => {
    const aPinned = pinnedIds.has(a.id) ? 1 : 0;
    const bPinned = pinnedIds.has(b.id) ? 1 : 0;
    if (aPinned !== bPinned) return bPinned - aPinned;
    return b.created_at_ms - a.created_at_ms;
  });

  // Calculate statistics
  const totalSnippets = history.length;
  const totalWords = history.reduce((acc, item) => {
    return acc + item.text.trim().split(/\s+/).filter(Boolean).length;
  }, 0);
  // Avg typing speed assumed 40 wpm -> time saved
  const minutesSaved = (totalWords / 40).toFixed(1);

  return (
    <div className="glass-panel rounded-3xl p-6 border border-slate-800/80 shadow-2xl space-y-6">
      {/* Header & Stats Banner */}
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 border-b border-slate-800/80 pb-4">
        <div>
          <h3 className="text-base font-bold text-slate-100 flex items-center gap-2">
            <HistoryIcon className="w-4 h-4 text-purple-400" />
            Dictation History Stream
          </h3>
          <p className="text-xs text-slate-400">
            Local dictation log saved on disk. Re-use, copy or re-paste previous transcriptions.
          </p>
        </div>

        {/* Stats Metrics Pills */}
        <div className="flex items-center gap-2 font-mono text-xs">
          <div className="bg-purple-950/40 border border-purple-800/40 px-3 py-1.5 rounded-xl text-purple-300">
            <span className="font-bold">{totalSnippets}</span> snippets
          </div>
          <div className="bg-indigo-950/40 border border-indigo-800/40 px-3 py-1.5 rounded-xl text-indigo-300">
            <span className="font-bold">{totalWords}</span> words
          </div>
          <div className="bg-emerald-950/40 border border-emerald-800/40 px-3 py-1.5 rounded-xl text-emerald-300 flex items-center gap-1">
            <Zap className="w-3 h-3 text-emerald-400" />
            <span className="font-bold">{minutesSaved}m</span> saved
          </div>
        </div>
      </div>

      {/* Search Input Bar */}
      <div className="relative">
        <Search className="w-4 h-4 text-slate-400 absolute left-3.5 top-3" />
        <input
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="Search past dictations..."
          className="w-full bg-slate-950/70 border border-slate-800 rounded-xl pl-10 pr-4 py-2.5 text-xs text-slate-200 focus:outline-none focus:border-purple-500 font-mono"
        />
        {searchQuery && (
          <button
            onClick={() => setSearchQuery("")}
            className="absolute right-3 top-2.5 text-xs text-slate-500 hover:text-slate-300"
          >
            Clear
          </button>
        )}
      </div>

      {/* History Timeline Stream */}
      {sortedHistory.length === 0 ? (
        <div className="py-12 text-center text-slate-500 space-y-2">
          <Clock className="w-10 h-10 mx-auto text-slate-700 stroke-1" />
          <p className="text-xs">
            {searchQuery ? "No dictations match your search query." : "No saved dictations yet."}
          </p>
        </div>
      ) : (
        <div className="space-y-3 max-h-[440px] overflow-y-auto pr-1">
          {sortedHistory.map((record) => {
            const isPinned = pinnedIds.has(record.id);
            const isCopied = copiedId === record.id;
            const dateStr = new Date(record.created_at_ms).toLocaleTimeString([], {
              hour: "2-digit",
              minute: "2-digit",
              second: "2-digit",
            });

            return (
              <div
                key={record.id}
                className={`group relative rounded-2xl p-4 border transition-all ${
                  isPinned
                    ? "bg-purple-950/30 border-purple-700/50 shadow-md"
                    : "bg-slate-950/50 border-slate-800/70 hover:border-slate-700"
                }`}
              >
                <div className="flex items-start justify-between gap-4">
                  <p className="flex-1 font-mono text-xs text-slate-200 leading-relaxed whitespace-pre-wrap">
                    {record.text}
                  </p>

                  <div className="flex items-center gap-1.5 opacity-95 md:opacity-0 group-hover:opacity-100 transition-opacity">
                    {/* Pin Button */}
                    <button
                      onClick={() => togglePin(record.id)}
                      className={`p-1.5 rounded-lg border transition-colors cursor-pointer ${
                        isPinned
                          ? "bg-purple-600 text-white border-purple-500"
                          : "bg-slate-900 border-slate-800 text-slate-400 hover:text-slate-200"
                      }`}
                      title={isPinned ? "Unpin snippet" : "Pin snippet to top"}
                    >
                      <Pin className="w-3.5 h-3.5" />
                    </button>

                    {/* Re-paste Button */}
                    {onPasteText && (
                      <button
                        onClick={() => onPasteText(record.text)}
                        className="p-1.5 rounded-lg bg-indigo-950/60 hover:bg-indigo-900 border border-indigo-800/60 text-indigo-300 transition-colors cursor-pointer"
                        title="Re-paste directly into target window"
                      >
                        <Send className="w-3.5 h-3.5" />
                      </button>
                    )}

                    {/* Copy Button */}
                    <button
                      onClick={() => handleCopyRecord(record.id, record.text)}
                      className={`flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
                        isCopied
                          ? "bg-emerald-600 text-white"
                          : "bg-purple-950/60 hover:bg-purple-900 text-purple-300 border border-purple-800/60"
                      }`}
                    >
                      {isCopied ? <Check className="w-3.5 h-3.5" /> : <Clipboard className="w-3.5 h-3.5" />}
                      {isCopied ? "Copied" : "Copy"}
                    </button>
                  </div>
                </div>

                <div className="mt-2.5 flex items-center justify-between text-[10px] text-slate-500 font-mono pt-2 border-t border-slate-800/40">
                  <span className="flex items-center gap-1">
                    <Clock className="w-3 h-3 text-slate-600" />
                    {dateStr}
                  </span>
                  <span>{record.text.trim().split(/\s+/).length} words</span>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
