import { useEffect, useRef, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { useAppStore } from "./stores/appStore";
import { useSearchStore } from "./stores/searchStore";
import { LockScreen } from "./components/lock/LockScreen";
import { SetupWizard } from "./components/lock/SetupWizard";
import { ThreePanel } from "./components/layout/ThreePanel";
import { ErrorState } from "./components/ui/ErrorState";
import { ToastProvider } from "./components/ui/Toast";
import { api } from "./lib/commands";

function App() {
  const status = useAppStore((s) => s.status);
  const corruptReason = useAppStore((s) => s.corruptReason);
  const init = useAppStore((s) => s.init);
  const lock = useAppStore((s) => s.lock);
  const searchInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    init();
  }, [init]);

  // Listen for auto-lock event from Rust backend
  useEffect(() => {
    const unlisten = listen("app:locked", async () => {
      useSearchStore.getState().reset();
      const { useSettingsStore } = await import("./stores/settingsStore");
      useSettingsStore.getState().reset();
      useAppStore.setState({ status: "locked", error: null, retryAfterMs: null });
    });
    return () => { unlisten.then((f) => f()); };
  }, []);

  // Activity heartbeat for auto-lock timer (throttled to every 60s)
  const lastHeartbeat = useRef(0);
  const sendHeartbeat = useCallback(() => {
    if (status !== "unlocked") return;
    const now = Date.now();
    if (now - lastHeartbeat.current < 60_000) return;
    lastHeartbeat.current = now;
    api.heartbeat().catch(() => {});
  }, [status]);

  useEffect(() => {
    if (status !== "unlocked") return;
    const events = ["mousemove", "keydown", "click", "scroll"] as const;
    events.forEach((e) => document.addEventListener(e, sendHeartbeat));
    // Send initial heartbeat on unlock
    sendHeartbeat();
    return () => {
      events.forEach((e) => document.removeEventListener(e, sendHeartbeat));
    };
  }, [status, sendHeartbeat]);

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
