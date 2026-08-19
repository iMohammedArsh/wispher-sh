import { ArrowUpRight, Clock3, History } from "lucide-react";
import { DictationRecord } from "../types";

interface RecentDictationsProps {
  history: DictationRecord[];
  onOpenHistory: () => void;
}

export function RecentDictations({
  history,
  onOpenHistory,
}: RecentDictationsProps) {
  const entries = [...history]
    .sort((a, b) => b.created_at_ms - a.created_at_ms)
    .slice(0, 4);

  return (
    <section className="recent-dictations" aria-labelledby="recent-title">
      <header className="recent-dictations-header">
        <div>
          <p className="section-eyebrow">Recent activity</p>
          <h2 id="recent-title">Your dictation history</h2>
        </div>
        <button type="button" onClick={onOpenHistory}>
          Open history <ArrowUpRight aria-hidden="true" />
        </button>
      </header>
      {entries.length === 0 ? (
        <div className="recent-empty">
          <History aria-hidden="true" />
          <p>Your completed dictations will appear here.</p>
        </div>
      ) : (
        <div className="recent-list">
          {entries.map((entry) => {
            const date = new Date(entry.created_at_ms);
            const words = entry.text.trim().split(/\s+/).filter(Boolean).length;
            return (
              <article className="recent-entry" key={entry.id}>
                <time dateTime={date.toISOString()}>
                  <Clock3 aria-hidden="true" />
                  <span>
                    {date.toLocaleDateString([], {
                      month: "short",
                      day: "numeric",
                    })}
                  </span>
                  <strong>
                    {date.toLocaleTimeString([], {
                      hour: "2-digit",
                      minute: "2-digit",
                    })}
                  </strong>
                </time>
                <p>{entry.text}</p>
                <span>{words} words</span>
              </article>
            );
          })}
        </div>
      )}
    </section>
  );
}
