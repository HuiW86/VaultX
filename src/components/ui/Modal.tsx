import { useEffect, useRef } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { X } from "lucide-react";
import { Button } from "./Button";

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title: string;
  children: React.ReactNode;
  danger?: boolean;
  confirmLabel?: string;
  cancelLabel?: string;
  onConfirm?: () => void;
  loading?: boolean;
}

export function Modal({
  open,
  onClose,
  title,
  children,
  danger = false,
  confirmLabel = "Confirm",
  cancelLabel = "Cancel",
  onConfirm,
  loading,
}: ModalProps) {
  const dialogRef = useRef<HTMLDivElement>(null);

  // Trap focus and handle Escape
  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [open, onClose]);

  // Focus first interactive element on open
  useEffect(() => {
    if (open && dialogRef.current) {
      const first = dialogRef.current.querySelector<HTMLElement>(
        "button, input, [tabindex]"
      );
      first?.focus();
    }
  }, [open]);

  return (
    <AnimatePresence>
      {open && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          {/* Overlay — danger modals don't dismiss on click */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="absolute inset-0 bg-[var(--color-bg-spotlight)]"
            onClick={danger ? undefined : onClose}
          />
          <motion.div
            ref={dialogRef}
            role="dialog"
            aria-modal="true"
            aria-label={title}
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.95 }}
            transition={{ duration: 0.3, ease: [0, 0, 0.2, 1] }}
            className="relative w-[480px] max-w-[90vw] bg-[var(--color-bg-elevated)] rounded-[var(--radius-xl)] p-[var(--spacing-xl)] shadow-[var(--shadow-lg)]"
          >
            <div className="flex items-center justify-between mb-[var(--spacing-lg)]">
              <h2 className="text-[var(--font-size-lg)] font-[var(--font-weight-semibold)]">
                {title}
              </h2>
              <button
                onClick={onClose}
                className="p-1 text-[var(--color-text-tertiary)] hover:text-[var(--color-text-primary)] rounded"
                aria-label="Close"
              >
                <X size={18} />
              </button>
            </div>

            <div className="text-[var(--font-size-md)] text-[var(--color-text-secondary)] mb-[var(--spacing-xl)]">
              {children}
            </div>

            {onConfirm && (
              <div className="flex justify-end gap-[var(--spacing-sm)]">
                <Button variant="ghost" onClick={onClose}>
                  {cancelLabel}
                </Button>
                <Button
                  variant={danger ? "danger" : "primary"}
                  onClick={onConfirm}
                  loading={loading}
                >
                  {confirmLabel}
                </Button>
              </div>
            )}
          </motion.div>
        </div>
      )}
    </AnimatePresence>
  );
}
