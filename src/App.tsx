import { useEffect, useRef } from "react";
import { useAppStore } from "./stores/appStore";
import { useSearchStore } from "./stores/searchStore";
import { LockScreen } from "./components/lock/LockScreen";
import { SetupWizard } from "./components/lock/SetupWizard";
import { ThreePanel } from "./components/layout/ThreePanel";
import { ErrorState } from "./components/ui/ErrorState";
import { ToastProvider } from "./components/ui/Toast";

function App() {
  const status = useAppStore((s) => s.status);
  const corruptReason = useAppStore((s) => s.corruptReason);
  const init = useAppStore((s) => s.init);
  const lock = useAppStore((s) => s.lock);
  const searchInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    init();
  }, [init]);

  // Global keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // Cmd+L to lock
      if (e.key === "l" && (e.metaKey || e.ctrlKey) && status === "unlocked") {
        e.preventDefault();
        useSearchStore.getState().reset();
        lock();
      }
      // Cmd+K to focus search
      if (e.key === "k" && (e.metaKey || e.ctrlKey) && status === "unlocked") {
        e.preventDefault();
        searchInputRef.current?.focus();
      }
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [status, lock]);

  const content = (() => {
    switch (status) {
      case "loading":
        return (
          <div className="h-full flex items-center justify-center bg-[var(--color-bg-app)]">
            <p className="text-[var(--color-text-tertiary)]">Loading...</p>
          </div>
        );
      case "first_run":
        return <SetupWizard />;
      case "locked":
        return <LockScreen />;
      case "unlocked":
        return <ThreePanel searchInputRef={searchInputRef} />;
      case "corrupted":
        return (
          <div className="h-full bg-[var(--color-bg-app)]">
            <ErrorState
              title="Vault corrupted"
              message={corruptReason || "Database files are inconsistent. Please check your data directory."}
            />
          </div>
        );
    }
  })();

  return <ToastProvider>{content}</ToastProvider>;
}

export default App;
