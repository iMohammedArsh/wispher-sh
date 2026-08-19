import {
  BarChart3,
  Clock3,
  Gauge,
  LockKeyhole,
  Sparkles,
  TrendingUp,
} from "lucide-react";
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { DictationRecord, InsightsSummary } from "../types";

interface InsightsPanelProps {
  summary: InsightsSummary | null;
  history: DictationRecord[];
  trackingEnabled: boolean;
  onSummaryChange: (summary: InsightsSummary) => void;
}

const titleForCategory: Record<string, string> = {
  coding: "Coding",
  email: "Email",
  messages: "Messages",
  documents: "Documents",
  browser: "Browser",
  other: "Other",
};

export function InsightsPanel({
  summary,
  history,
  trackingEnabled,
  onSummaryChange,
}: InsightsPanelProps) {
  const [heard, setHeard] = useState("");
  const [approved, setApproved] = useState("");
  const [ruleError, setRuleError] = useState<string | null>(null);
  const addRule = async () => {
    try {
      const next = await invoke<InsightsSummary>("add_correction_rule", {
        source: heard,
        replacement: approved,
      });
      onSummaryChange(next);
      setHeard("");
      setApproved("");
      setRuleError(null);
    } catch (error) {
      setRuleError(String(error));
    }
  };
  const totalWords = history.reduce(
    (total, record) =>
      total + record.text.trim().split(/\s+/).filter(Boolean).length,
    0,
  );
  const categories = Object.entries(summary?.category_words ?? {}).sort(
    ([, a], [, b]) => b - a,
  );
  const largestCategory = Math.max(1, ...categories.map(([, words]) => words));
  const remaining = Math.max(0, 2_000 - (summary?.tracked_words ?? 0));

  return (
    <section className="glass-panel rounded-3xl p-6 border border-slate-800/80 shadow-2xl space-y-6">
      <header className="flex flex-col md:flex-row md:items-start md:justify-between gap-4 border-b border-slate-800/80 pb-5">
        <div>
          <p className="section-eyebrow">PRIVATE, ON THIS DEVICE</p>
          <h3 className="text-xl font-bold text-slate-100 flex items-center gap-2">
            <BarChart3 className="w-5 h-5 text-emerald-400" /> Insights
          </h3>
          <p className="mt-2 text-sm text-slate-500">
            Usage and learning signals calculated from your local dictation
            history. No recordings or screen content are stored.
          </p>
        </div>
        <div className="inline-flex items-center gap-2 rounded-full border border-emerald-800/40 bg-emerald-950/30 px-3 py-1.5 text-xs font-semibold text-emerald-300">
          <LockKeyhole className="w-3.5 h-3.5" /> Local data
        </div>
      </header>

      <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4">
        <Metric
          icon={<Sparkles />}
          label="Total words"
          value={totalWords.toLocaleString()}
          note="completed dictations"
        />
        <Metric
          icon={<Gauge />}
          label="Words per minute"
          value={summary?.average_wpm ? String(summary.average_wpm) : "—"}
          note={
            summary?.average_wpm
              ? "from timed dictations"
              : "starting to measure"
          }
        />
        <Metric
          icon={<TrendingUp />}
          label="Current streak"
          value={`${summary?.current_streak_days ?? 0} days`}
          note={`longest: ${summary?.longest_streak_days ?? 0} days`}
        />
        <Metric
          icon={<Clock3 />}
          label="Local fixes"
          value={String(summary?.correction_rules.length ?? 0)}
          note="approved reusable rules"
        />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-[1.1fr_.9fr] gap-5">
        <section className="rounded-2xl border border-slate-800/80 bg-slate-950/40 p-5">
          <div className="flex items-center justify-between gap-3">
            <div>
              <h4 className="font-bold text-slate-100">Your usage</h4>
              <p className="mt-1 text-xs text-slate-500">
                How your local dictation is grouped.
              </p>
            </div>
            {!trackingEnabled && (
              <span className="text-xs text-slate-500">App grouping off</span>
            )}
          </div>
          {!trackingEnabled ? (
            <p className="mt-5 rounded-xl border border-dashed border-slate-700 p-4 text-sm text-slate-500">
              Enable “Local app-category insights” in Settings to group
              dictation by app without saving titles or content.
            </p>
          ) : categories.length === 0 ? (
            <p className="mt-5 text-sm text-slate-500">
              Complete a new dictation to begin building local activity data.
            </p>
          ) : (
            <div className="mt-5 space-y-3">
              {categories.map(([category, words]) => (
                <div key={category}>
                  <div className="mb-1.5 flex justify-between text-xs">
                    <span className="font-semibold text-slate-300">
                      {titleForCategory[category] ?? category}
                    </span>
                    <span className="text-slate-500">
                      {words.toLocaleString()} words
                    </span>
                  </div>
                  <div className="h-2 overflow-hidden rounded-full bg-slate-800">
                    <div
                      className="h-full rounded-full bg-emerald-500"
                      style={{
                        width: `${Math.max(4, (words / largestCategory) * 100)}%`,
                      }}
                    />
                  </div>
                </div>
              ))}
            </div>
          )}
        </section>

        <section className="rounded-2xl border border-slate-800/80 bg-slate-950/40 p-5">
          <h4 className="font-bold text-slate-100">Your voice</h4>
          {summary?.profile_unlocked ? (
            <p className="mt-2 text-sm text-slate-500">
              Your local profile is ready. Wisper can now surface recurring
              vocabulary and correction patterns as you approve them.
            </p>
          ) : (
            <>
              <p className="mt-2 text-sm text-slate-500">
                Wisper waits for enough real data before drawing any conclusion
                about your writing habits.
              </p>
              <div className="mt-5 h-2 overflow-hidden rounded-full bg-slate-800">
                <div
                  className="h-full rounded-full bg-emerald-500"
                  style={{
                    width: `${Math.min(100, ((summary?.tracked_words ?? 0) / 2_000) * 100)}%`,
                  }}
                />
              </div>
              <p className="mt-2 text-xs text-slate-500">
                {remaining.toLocaleString()} more tracked words until the
                private profile unlocks.
              </p>
            </>
          )}
        </section>
      </div>

      <section className="rounded-2xl border border-slate-800/80 bg-slate-950/40 p-5">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h4 className="font-bold text-slate-100">
              Teach Wisper a correction
            </h4>
            <p className="mt-1 text-xs text-slate-500">
              Only an explicit rule is reused in future dictations. Example:
              heard “us art” → use “Alt”.
            </p>
          </div>
          <span className="text-xs text-slate-500">
            {summary?.correction_rules.length ?? 0} saved
          </span>
        </div>
        <div className="mt-4 grid grid-cols-1 md:grid-cols-[1fr_auto_1fr_auto] gap-2">
          <input
            value={heard}
            onChange={(event) => setHeard(event.target.value)}
            placeholder="Heard"
            className="rounded-xl border border-slate-700 bg-slate-900 px-3 py-2 text-sm text-slate-100"
          />
          <span className="self-center text-center text-slate-500">→</span>
          <input
            value={approved}
            onChange={(event) => setApproved(event.target.value)}
            placeholder="Use instead"
            className="rounded-xl border border-slate-700 bg-slate-900 px-3 py-2 text-sm text-slate-100"
          />
          <button
            type="button"
            onClick={addRule}
            className="rounded-xl bg-emerald-700 px-3 py-2 text-sm font-semibold text-white hover:bg-emerald-600"
          >
            Learn
          </button>
        </div>
        {ruleError && <p className="mt-2 text-xs text-red-400">{ruleError}</p>}
      </section>
    </section>
  );
}

function Metric({
  icon,
  label,
  value,
  note,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  note: string;
}) {
  return (
    <article className="rounded-2xl border border-slate-800/80 bg-slate-950/40 p-4">
      <div className="flex items-center gap-2 text-emerald-400">
        {icon}
        <span className="text-xs font-semibold uppercase tracking-wider text-slate-500">
          {label}
        </span>
      </div>
      <strong className="mt-4 block text-2xl text-slate-100">{value}</strong>
      <span className="mt-1 block text-xs text-slate-500">{note}</span>
    </article>
  );
}
