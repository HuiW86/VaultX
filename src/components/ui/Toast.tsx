import { useEffect, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { Check, AlertCircle, Info } from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
import { useTranslation } from "../../i18n";

interface ToastItem {
  id: number;
  message: string;
  type: "success" | "error" | "info";
  countdown?: number;
}

let nextId = 0;

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const { t } = useTranslation();

  const addToast = useCallback(
    (message: string, type: ToastItem["type"] = "info", countdown?: number) => {
      const id = nextId++;
      setToasts((prev) => [...prev, { id, message, type, countdown }]);
      const duration = type === "error" ? 4000 : 2000;
      setTimeout(() => {
        setToasts((prev) => prev.filter((toast) => toast.id !== id));
      }, duration);
    },
    []
  );

  useEffect(() => {
    const unlistenCopied = listen<number>("clipboard:copied", (event) => {
      const ms = event.payload;
      addToast(t("toast.copied_clearing", { seconds: Math.round(ms / 1000) }), "success", ms);
    });
    const unlistenCleared = listen("clipboard:cleared", () => {
      addToast(t("toast.clipboard_cleared"), "info");
    });

    // Expose addToast globally for imperative use
    (window as any).__vaultx_toast = addToast;

    return () => {
      unlistenCopied.then((f) => f());
      unlistenCleared.then((f) => f());
    };
  }, [addToast]);

  const icons = {
    success: <Check size={14} className="text-[var(--color-success)]" />,
    error: <AlertCircle size={14} className="text-[var(--color-error)]" />,
    info: <Info size={14} className="text-[var(--color-text-secondary)]" />,
  };

  return (
    <>
      {children}
      <div className="fixed bottom-12 left-1/2 -translate-x-1/2 z-50 flex flex-col gap-2">
        <AnimatePresence>
          {toasts.map((toast) => (
            <motion.div
              key={toast.id}
              role="status"
              aria-live="polite"
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: 20 }}
              transition={{ duration: 0.2 }}
              className="flex items-center gap-[var(--spacing-sm)] px-[var(--spacing-md)] py-[var(--spacing-sm)] bg-[var(--color-bg-elevated)] border border-[var(--color-border)] rounded-[var(--radius-md)] shadow-[var(--shadow-md)] text-[var(--font-size-sm)] text-[var(--color-text-primary)] max-w-[360px]"
            >
              {icons[toast.type]}
              <span>{toast.message}</span>
            </motion.div>
          ))}
        </AnimatePresence>
      </div>
    </>
  );
}

export function useToast() {
  return {
    toast: (message: string, type: ToastItem["type"] = "info") => {
      (window as any).__vaultx_toast?.(message, type);
    },
  };
}
