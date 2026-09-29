import React from "react";
import {
  BookOpen,
  CircleHelp,
  Clock3,
  ChartNoAxesCombined,
  Keyboard,
  Mic,
  Settings,
  SquarePen,
} from "lucide-react";
import { ActiveTab, AudioDiagnostics } from "../types";

interface HeaderProps {
  activeTab: ActiveTab;
  setActiveTab: (tab: ActiveTab) => void;
  audio: AudioDiagnostics;
  historyCount: number;
  hotkey: string;
}

const items: { id: ActiveTab; label: string; icon: React.ElementType }[] = [
  { id: "studio", label: "Dictation", icon: SquarePen },
  { id: "history", label: "History", icon: Clock3 },
  { id: "insights", label: "Insights", icon: ChartNoAxesCombined },
  { id: "vocabulary", label: "Vocabulary", icon: BookOpen },
  { id: "settings", label: "Settings", icon: Settings },
];

export const Header: React.FC<HeaderProps> = ({
  activeTab,
  setActiveTab,
  audio,
  historyCount,
  hotkey,
}) => {
  const navigation = (compact = false) => (
    <nav
      className={compact ? "compact-nav" : "side-nav"}
      aria-label={compact ? "Compact navigation" : undefined}
    >
      {items.map(({ id, label, icon: Icon }) => (
        <button
          key={id}
          className={activeTab === id ? "active" : ""}
          onClick={() => setActiveTab(id)}
          aria-current={activeTab === id ? "page" : undefined}
        >
          <Icon aria-hidden="true" />
          <span>{label}</span>
          {id === "history" && historyCount > 0 && <em>{historyCount}</em>}
        </button>
      ))}
    </nav>
  );

  return (
    <>
      <aside className="app-sidebar" aria-label="Main navigation">
        <div className="workspace-name">
          <span className="brand-soundmark" aria-hidden="true">
            <i />
            <i />
            <i />
            <i />
            <i />
          </span>
          <div>
            hams
            <span>Private voice workspace</span>
          </div>
        </div>
        {navigation()}
        <div className="sidebar-status">
          <div className="mic-state">
            <Mic aria-hidden="true" />
            <span>
              {audio.native_sample_rate
                ? "Microphone ready"
                : "Checking microphone"}
            </span>
          </div>
          <button onClick={() => setActiveTab("guide")}>
            <CircleHelp aria-hidden="true" /> Help & setup
          </button>
          <p>
            <Keyboard aria-hidden="true" /> Hold <kbd>{hotkey}</kbd>
          </p>
        </div>
      </aside>
      {navigation(true)}
    </>
  );
};
