import { useState } from "react";
import { Sidebar } from "./Sidebar";
import { EntryList } from "./EntryList";
import { DetailPanel } from "./DetailPanel";
import { SettingsPanel } from "../settings/SettingsPanel";

interface ThreePanelProps {
  searchInputRef?: React.RefObject<HTMLInputElement | null>;
}

export function ThreePanel({ searchInputRef }: ThreePanelProps) {
  const [showSettings, setShowSettings] = useState(false);

  const handleNewEntry = (category: string) => {
    (window as any).__vaultx_newEntry?.(category);
  };

  return (
    <div className="h-full grid" style={{ gridTemplateColumns: "220px 300px 1fr" }}>
      <aside
        role="navigation"
        aria-label="Sidebar"
        className="bg-[var(--color-bg-sidebar)] border-r border-[var(--color-border)] overflow-y-auto"
        style={{ paddingTop: 48 }}
      >
        <Sidebar onOpenSettings={() => setShowSettings(true)} />
      </aside>

      <div className="bg-[var(--color-bg-panel)] border-r border-[var(--color-border)] flex flex-col overflow-hidden"
           style={{ paddingTop: 16 }}>
        <EntryList
          onNewEntry={() => handleNewEntry("login")}
          searchInputRef={searchInputRef}
        />
      </div>

      <main className="bg-[var(--color-bg-panel)] overflow-y-auto" style={{ paddingTop: 16 }}>
        {showSettings ? (
          <SettingsPanel onClose={() => setShowSettings(false)} />
        ) : (
          <DetailPanel />
        )}
      </main>
    </div>
  );
}
