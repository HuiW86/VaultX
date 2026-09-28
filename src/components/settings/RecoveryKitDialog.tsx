import { useCallback, useEffect, useState } from "react";
import { Check, Download } from "lucide-react";
import { Modal } from "../ui/Modal";
import { Button } from "../ui/Button";
import { api } from "../../lib/commands";
import { downloadRecoveryKit } from "../../lib/recoveryKit";
import { useAppStore } from "../../stores/appStore";
import { useTranslation } from "../../i18n";

interface RecoveryKitDialogProps {
  open: boolean;
  onClose: () => void;
}

/**
 * Regenerate the recovery kit (contract C11): confirm that the previous kit
 * stops working, call `generate_recovery_kit`, then show the new key once
 * with the same download as the setup wizard.
 */
export function RecoveryKitDialog({ open, onClose }: RecoveryKitDialogProps) {
  const { t } = useTranslation();
  const refreshRecoveryKitStatus = useAppStore((s) => s.refreshRecoveryKitStatus);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [recoveryKey, setRecoveryKey] = useState("");
  const [fileContent, setFileContent] = useState("");
  const [downloaded, setDownloaded] = useState(false);
  const [confirmingClose, setConfirmingClose] = useState(false);

  // Drop the key text from component state whenever the dialog closes.
  useEffect(() => {
    if (open) return;
    setRecoveryKey("");
    setFileContent("");
    setDownloaded(false);
    setConfirmingClose(false);
    setError("");
    setLoading(false);
  }, [open]);

  const handleGenerate = useCallback(async () => {
    if (loading) return;
    setLoading(true);
    setError("");
    try {
      const kit = await api.generateRecoveryKit();
      setRecoveryKey(kit.recovery_key);
      setFileContent(kit.file_content);
    } catch {
      setError(t("recovery_kit.failed"));
    } finally {
      setLoading(false);
      await refreshRecoveryKitStatus();
    }
  }, [loading, refreshRecoveryKitStatus, t]);

  const handleDownload = useCallback(() => {
    downloadRecoveryKit(fileContent);
    setDownloaded(true);
  }, [fileContent]);

  // Once a new kit exists the previous one no longer works, so every close
  // path (X, overlay, Escape) asks for confirmation until the kit is saved.
  const guardedClose = useCallback(() => {
    if (downloaded) onClose();
    else setConfirmingClose(true);
  }, [downloaded, onClose]);
  const cancelClose = useCallback(() => setConfirmingClose(false), []);
  // Once "generate" is clicked the backend may commit the new kit (and
  // invalidate the old one) at any moment, so no close path may drop the
  // dialog before the new key is shown (H3).
  const closeUnlessGenerating = useCallback(() => {
    if (!loading) onClose();
  }, [loading, onClose]);

  if (!recoveryKey) {
    return (
      <Modal
        open={open}
        onClose={closeUnlessGenerating}
        title={t("recovery_kit.confirm_title")}
        confirmLabel={t("recovery_kit.confirm")}
        onConfirm={handleGenerate}
        loading={loading}
        dismissible={!loading}
      >
        <p>{t("recovery_kit.confirm_desc")}</p>
        {error && (
          <p role="alert" className="mt-[var(--spacing-sm)] text-[var(--font-size-xs)] text-[var(--color-error)]">
            {error}
          </p>
        )}
      </Modal>
    );
  }

  if (confirmingClose) {
    // Close paths here only go back to the key; closing needs the explicit button.
    return (
      <Modal open={open} onClose={cancelClose} title={t("recovery_kit.close_confirm_title")}>
        <p role="alert" className="mb-[var(--spacing-md)]">
          {t("recovery_kit.close_confirm_desc")}
        </p>
        <div className="flex justify-end gap-[var(--spacing-sm)]">
          <Button variant="ghost" onClick={onClose}>
            {t("recovery_kit.close_anyway")}
          </Button>
          <Button variant="primary" onClick={cancelClose}>
            {t("recovery_kit.close_back")}
          </Button>
        </div>
      </Modal>
    );
  }

  return (
    <Modal open={open} onClose={guardedClose} title={t("recovery_kit.new_title")}>
      <p className="mb-[var(--spacing-md)]">{t("recovery_kit.new_desc")}</p>
      <div className="w-full bg-[var(--color-bg-panel)] border border-[var(--color-border)] rounded-[var(--radius-lg)] p-[var(--spacing-md)] mb-[var(--spacing-md)]">
        <p className="text-[var(--font-size-xs)] text-[var(--color-text-tertiary)] mb-[var(--spacing-xs)]">
          {t("setup.recovery_key_label")}
        </p>
        <p
          data-testid="recovery-key"
          className="text-[var(--font-size-lg)] text-[var(--color-text-primary)] font-[var(--font-weight-semibold)] select-all text-center tracking-wider"
          style={{ fontFamily: "var(--font-mono)" }}
        >
          {recoveryKey}
        </p>
      </div>
      <div className="flex justify-end gap-[var(--spacing-sm)]">
        <Button variant={downloaded ? "ghost" : "primary"} onClick={handleDownload}>
          {downloaded ? <Check size={16} /> : <Download size={16} />}
          {downloaded ? t("setup.recovery_downloaded") : t("setup.recovery_download")}
        </Button>
        <Button variant="primary" disabled={!downloaded} onClick={onClose}>
          {t("recovery_kit.done")}
        </Button>
      </div>
    </Modal>
  );
}
