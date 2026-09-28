import { useState, useCallback, type FormEvent } from "react";
import { Shield, Download, Check } from "lucide-react";
import { Button } from "../ui/Button";
import { StrengthMeter } from "../ui/StrengthMeter";
import { clearFirstKitPending, useAppStore } from "../../stores/appStore";
import { api } from "../../lib/commands";
import { downloadRecoveryKit } from "../../lib/recoveryKit";
import { useTranslation } from "../../i18n";

type Step = "password" | "recovery" | "done";

/**
 * First run: create the vault, then show its first recovery kit (contract
 * C11). The vault is unlocked after creation, but the app stays in
 * `setup_recovery` (this wizard, no main window) until the kit has been
 * downloaded; the kit page has no way forward before that.
 */
export function SetupWizard() {
  const { t } = useTranslation();
  // Mounted in `setup_recovery` (e.g. unlock after a restart with the first
  // kit still unsaved): go straight to the kit step.
  const [step, setStep] = useState<Step>(() =>
    useAppStore.getState().status === "setup_recovery" ? "recovery" : "password"
  );
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [loading, setLoading] = useState(false);
  const [errors, setErrors] = useState<{ password?: string; confirm?: string }>({});
  const [recoveryKey, setRecoveryKey] = useState("");
  const [fileContent, setFileContent] = useState("");
  const [downloaded, setDownloaded] = useState(false);
  const [kitLoading, setKitLoading] = useState(false);
  const [kitError, setKitError] = useState("");
  const setup = useAppStore((s) => s.setup);
  const finishSetup = useAppStore((s) => s.finishSetup);
  const globalError = useAppStore((s) => s.error);

  const validate = useCallback(() => {
    const errs: typeof errors = {};
    if (password.length < 8) errs.password = t("setup.password_min_length");
    if (password !== confirm) errs.confirm = t("setup.passwords_mismatch");
    setErrors(errs);
    return Object.keys(errs).length === 0;
  }, [password, confirm]);

  // Only ever triggered by an explicit action, so a kit is never generated
  // twice behind the user's back (each generation invalidates the previous).
  const generateKit = useCallback(async () => {
    setKitLoading(true);
    setKitError("");
    try {
      const kit = await api.generateRecoveryKit();
      setRecoveryKey(kit.recovery_key);
      setFileContent(kit.file_content);
      setDownloaded(false);
    } catch {
      setKitError(t("setup.recovery_generate_failed"));
    } finally {
      setKitLoading(false);
    }
  }, [t]);

  const handleCreateVault = useCallback(
    async (e: FormEvent) => {
      e.preventDefault();
      if (!validate() || loading) return;
      setLoading(true);
      try {
        await setup(password);
      } catch {
        // Error shown via globalError
        setLoading(false);
        return;
      }
      setLoading(false);
      setStep("recovery");
      await generateKit();
    },
    [password, validate, loading, setup, generateKit]
  );

  const handleDownload = useCallback(() => {
    downloadRecoveryKit(fileContent);
    setDownloaded(true);
    clearFirstKitPending();
  }, [fileContent]);

  if (step === "recovery") {
    return (
      <div className="h-full flex flex-col items-center justify-center bg-[var(--color-bg-app)]">
        <div className="flex flex-col items-center w-96">
          <Download
            size={48}
            className="text-[var(--color-warning)] mb-[var(--spacing-lg)]"
          />
          <h1 className="text-[var(--font-size-h1)] font-[var(--font-weight-bold)] text-[var(--color-text-primary)] mb-[var(--spacing-sm)]">
            {t("setup.recovery_title")}
          </h1>
          <p className="text-[var(--font-size-sm)] text-[var(--color-text-secondary)] mb-[var(--spacing-xl)] text-center">
            {t("setup.recovery_desc")}
          </p>

          {!recoveryKey ? (
            <div className="w-full flex flex-col gap-[var(--spacing-md)]">
              {kitError && (
                <p role="alert" className="text-[var(--font-size-xs)] text-[var(--color-error)] text-center">
                  {kitError}
                </p>
              )}
              <Button
                variant="primary"
                size="lg"
                loading={kitLoading}
                disabled={kitLoading}
                onClick={generateKit}
                className="w-full"
              >
                {kitLoading ? t("setup.recovery_generating") : t("setup.recovery_generate")}
              </Button>
            </div>
          ) : (
          <>
          {/* Recovery key display */}
          <div className="w-full bg-[var(--color-bg-elevated)] border border-[var(--color-border)] rounded-[var(--radius-lg)] p-[var(--spacing-lg)] mb-[var(--spacing-lg)]">
            <p className="text-[var(--font-size-xs)] text-[var(--color-text-tertiary)] mb-[var(--spacing-sm)]">
              {t("setup.recovery_key_label")}
            </p>
            <p
              data-testid="setup-recovery-key"
              className="text-[var(--font-size-lg)] text-[var(--color-text-primary)] font-[var(--font-weight-semibold)] select-all text-center tracking-wider"
              style={{ fontFamily: "var(--font-mono)" }}
            >
              {recoveryKey}
            </p>
          </div>

          <div className="w-full flex flex-col gap-[var(--spacing-md)]">
            <Button
              variant={downloaded ? "ghost" : "primary"}
              size="lg"
              onClick={handleDownload}
              className="w-full"
            >
              {downloaded ? (
                <>
                  <Check size={16} />
                  {t("setup.recovery_downloaded")}
                </>
              ) : (
                <>
                  <Download size={16} />
                  {t("setup.recovery_download")}
                </>
              )}
            </Button>

            <Button
              variant="primary"
              size="lg"
              disabled={!downloaded}
              onClick={() => setStep("done")}
              className="w-full"
            >
              {t("setup.recovery_saved")}
            </Button>
          </div>

          {!downloaded && (
            <p className="text-[var(--font-size-xs)] text-[var(--color-text-tertiary)] mt-[var(--spacing-md)] text-center">
              {t("setup.recovery_must_download")}
            </p>
          )}
          </>
          )}
        </div>
      </div>
    );
  }

  if (step === "done") {
    // Already unlocked from setup, just show success briefly
    return (
      <div className="h-full flex flex-col items-center justify-center bg-[var(--color-bg-app)]">
        <div className="flex flex-col items-center w-80">
          <Check
            size={48}
            className="text-[var(--color-success)] mb-[var(--spacing-lg)]"
          />
          <h1 className="text-[var(--font-size-h1)] font-[var(--font-weight-bold)] text-[var(--color-text-primary)] mb-[var(--spacing-sm)]">
            {t("setup.all_set")}
          </h1>
          <p className="text-[var(--font-size-sm)] text-[var(--color-text-secondary)] mb-[var(--spacing-xl)] text-center">
            {t("setup.all_set_desc")}
          </p>
          <Button
            variant="primary"
            size="lg"
            onClick={finishSetup}
            className="w-full"
          >
            {t("setup.get_started")}
          </Button>
        </div>
      </div>
    );
  }

  // Step: password
  return (
    <div className="h-full flex flex-col items-center justify-center bg-[var(--color-bg-app)]">
      <div className="flex flex-col items-center w-80">
        <Shield
          size={48}
          className="text-[var(--color-primary)] mb-[var(--spacing-lg)]"
        />
        <h1
          className="text-[var(--font-size-display)] font-[var(--font-weight-bold)] text-[var(--color-text-primary)] mb-[var(--spacing-sm)]"
          style={{ fontFamily: "var(--font-display)" }}
        >
          {t("setup.welcome")}
        </h1>
        <p className="text-[var(--font-size-sm)] text-[var(--color-text-secondary)] mb-[var(--spacing-3xl)] text-center">
          {t("setup.welcome_desc")}
        </p>

        <form onSubmit={handleCreateVault} className="w-full flex flex-col gap-[var(--spacing-lg)]">
          <div className="flex flex-col gap-[var(--spacing-xs)]">
            <label className="text-[var(--font-size-xs)] font-[var(--font-weight-medium)] text-[var(--color-text-secondary)]">
              {t("setup.master_password")}
            </label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              autoFocus
              className="w-full h-10 px-[var(--spacing-md)] bg-[var(--color-bg-input)] border border-[var(--color-border)] rounded-[var(--radius-md)] text-[var(--font-size-md)] text-[var(--color-text-primary)] outline-none focus:border-[var(--color-primary)]"
              aria-label={t("setup.master_password")}
            />
            <StrengthMeter password={password} />
            {errors.password && (
              <p className="text-[var(--font-size-xs)] text-[var(--color-error)]">
                {errors.password}
              </p>
            )}
          </div>

          <div className="flex flex-col gap-[var(--spacing-xs)]">
            <label className="text-[var(--font-size-xs)] font-[var(--font-weight-medium)] text-[var(--color-text-secondary)]">
              {t("setup.confirm_password")}
            </label>
            <input
              type="password"
              value={confirm}
              onChange={(e) => setConfirm(e.target.value)}
              className="w-full h-10 px-[var(--spacing-md)] bg-[var(--color-bg-input)] border border-[var(--color-border)] rounded-[var(--radius-md)] text-[var(--font-size-md)] text-[var(--color-text-primary)] outline-none focus:border-[var(--color-primary)]"
              aria-label={t("setup.confirm_password")}
            />
            {errors.confirm && (
              <p className="text-[var(--font-size-xs)] text-[var(--color-error)]">
                {errors.confirm}
              </p>
            )}
          </div>

          {globalError && (
            <p className="text-[var(--font-size-xs)] text-[var(--color-error)] text-center">
              {globalError}
            </p>
          )}

          <Button type="submit" variant="primary" size="lg" loading={loading} className="w-full">
            {loading ? t("setup.creating") : t("setup.create_vault")}
          </Button>
        </form>
      </div>
    </div>
  );
}
