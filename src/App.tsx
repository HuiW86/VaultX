import { useEffect, useRef, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { useAppStore } from "./stores/appStore";
import { useSearchStore } from "./stores/searchStore";
import { useSettingsStore } from "./stores/settingsStore";
import { LockScreen } from "./components/lock/LockScreen";
import { SetupWizard } from "./components/lock/SetupWizard";
import { ThreePanel } from "./components/layout/ThreePanel";
import { ErrorState } from "./components/ui/ErrorState";
import { ToastProvider } from "./components/ui/Toast";
import { I18nProvider, useTranslation } from "./i18n";
import type { Locale } from "./i18n";
import { api } from "./lib/commands";

function App() {
  const status = useAppStore((s) => s.status);
  const corruptReason = useAppStore((s) => s.corruptReason);
  const init = useAppStore((s) => s.init);
  const lock = useAppStore((s) => s.lock);
  const language = useSettingsStore((s) => s.settings.language) as Locale;
  const settingsLoaded = useSettingsStore((s) => s.loaded);
  const loadSettings = useSettingsStore((s) => s.load);
  const searchInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    init();
    loadSettings();
  }, [init, loadSettings]);

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
    sendHeartbeat();
    return () => {
      events.forEach((e) => document.removeEventListener(e, sendHeartbeat));
    };
  }, [status, sendHeartbeat]);

  // Global keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "l" && (e.metaKey || e.ctrlKey) && status === "unlocked") {
        e.preventDefault();
        useSearchStore.getState().reset();
        lock();
      }
      if (e.key === "k" && (e.metaKey || e.ctrlKey) && status === "unlocked") {
        e.preventDefault();
        searchInputRef.current?.focus();
      }
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [status, lock]);

  return (
    <I18nProvider locale={settingsLoaded ? language : "en"}>
      <ToastProvider>
        <AppContent status={status} corruptReason={corruptReason} searchInputRef={searchInputRef} />
      </ToastProvider>
    </I18nProvider>
  );
}

function AppContent({ status, corruptReason, searchInputRef }: {
  status: string;
  corruptReason: string | null;
  searchInputRef: React.RefObject<HTMLInputElement | null>;
}) {
  const { t } = useTranslation();

  switch (status) {
    case "loading":
      return (
        <div className="h-full flex items-center justify-center bg-[var(--color-bg-app)]">
          <p className="text-[var(--color-text-tertiary)]">{t("app.loading")}</p>
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
            title={t("app.vault_corrupted")}
            message={corruptReason || t("app.vault_corrupted_desc")}
          />
        </div>
      );
    default:
      return null;
  }
}

export default App;
