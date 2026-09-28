import { useEffect, useState } from "react";
import { AlertTriangle } from "lucide-react";
import { Button } from "../ui/Button";
import { RecoveryKitDialog } from "../settings/RecoveryKitDialog";
import { useAppStore } from "../../stores/appStore";
import { useTranslation } from "../../i18n";

/**
 * Post-unlock notice shown while the vault has no usable recovery kit, e.g.
 * after a recovery (single-use kit, C5) or a v1 -> v2 migration (contract C11).
 */
export function RecoveryKitBanner() {
  const { t } = useTranslation();
  const recoveryKit = useAppStore((s) => s.recoveryKit);
  const refreshRecoveryKitStatus = useAppStore((s) => s.refreshRecoveryKitStatus);
  const [dialogOpen, setDialogOpen] = useState(false);

  useEffect(() => {
    refreshRecoveryKitStatus();
  }, [refreshRecoveryKitStatus]);

  const missing = recoveryKit !== null && !recoveryKit.present && recoveryKit.can_generate;
  if (!missing && !dialogOpen) return null;

  return (
    <>
      {missing && (
        <div
          role="status"
          className="mx-[var(--spacing-lg)] mb-[var(--spacing-md)] flex items-center gap-[var(--spacing-sm)] rounded-[var(--radius-md)] border border-[var(--color-warning)] px-[var(--spacing-md)] py-[var(--spacing-sm)] text-[var(--font-size-sm)] text-[var(--color-text-primary)]"
        >
          <AlertTriangle size={16} className="shrink-0 text-[var(--color-warning)]" />
          <span className="flex-1">{t("recovery_kit.banner")}</span>
          <Button size="sm" variant="secondary" onClick={() => setDialogOpen(true)}>
            {t("recovery_kit.banner_action")}
          </Button>
        </div>
      )}
      <RecoveryKitDialog open={dialogOpen} onClose={() => setDialogOpen(false)} />
    </>
  );
}
