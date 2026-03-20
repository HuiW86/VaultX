import { useState, useCallback, type FormEvent } from "react";
import { Shield, Download, Check } from "lucide-react";
import { Button } from "../ui/Button";
import { StrengthMeter } from "../ui/StrengthMeter";
import { useAppStore } from "../../stores/appStore";
import { api } from "../../lib/commands";

type Step = "password" | "recovery" | "done";

export function SetupWizard() {
  const [step, setStep] = useState<Step>("password");
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [loading, setLoading] = useState(false);
  const [errors, setErrors] = useState<{ password?: string; confirm?: string }>({});
  const [recoveryKey, setRecoveryKey] = useState("");
  const [fileContent, setFileContent] = useState("");
  const [downloaded, setDownloaded] = useState(false);
  const setup = useAppStore((s) => s.setup);
  const globalError = useAppStore((s) => s.error);

  const validate = useCallback(() => {
    const errs: typeof errors = {};
    if (password.length < 8) errs.password = "Password must be at least 8 characters";
    if (password !== confirm) errs.confirm = "Passwords don't match";
    setErrors(errs);
    return Object.keys(errs).length === 0;
  }, [password, confirm]);

  const handleCreateVault = useCallback(
    async (e: FormEvent) => {
      e.preventDefault();
      if (!validate() || loading) return;
      setLoading(true);
      try {
        await setup(password);
        // Generate recovery kit
        const kit = await api.generateRecoveryKit();
        setRecoveryKey(kit.recovery_key);
        setFileContent(kit.file_content);
        setStep("recovery");
      } catch {
        // Error shown via globalError
      } finally {
        setLoading(false);
      }
    },
    [password, validate, loading, setup]
  );

  const handleDownload = useCallback(() => {
    const blob = new Blob([fileContent], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "VaultX-Recovery-Kit.txt";
    a.click();
    URL.revokeObjectURL(url);
    setDownloaded(true);
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
            Save Your Recovery Kit
          </h1>
          <p className="text-[var(--font-size-sm)] text-[var(--color-text-secondary)] mb-[var(--spacing-xl)] text-center">
            If you forget your master password, this is your only way back in.
            Download and store it somewhere safe.
          </p>

          {/* Recovery key display */}
          <div className="w-full bg-[var(--color-bg-elevated)] border border-[var(--color-border)] rounded-[var(--radius-lg)] p-[var(--spacing-lg)] mb-[var(--spacing-lg)]">
            <p className="text-[var(--font-size-xs)] text-[var(--color-text-tertiary)] mb-[var(--spacing-sm)]">
              Your Recovery Key
            </p>
            <p
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
                  Downloaded — Download Again
                </>
              ) : (
                <>
                  <Download size={16} />
                  Download Recovery Kit
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
              I've Saved My Recovery Kit
            </Button>
          </div>

          {!downloaded && (
            <p className="text-[var(--font-size-xs)] text-[var(--color-text-tertiary)] mt-[var(--spacing-md)] text-center">
              You must download the recovery kit before continuing
            </p>
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
            You're All Set
          </h1>
          <p className="text-[var(--font-size-sm)] text-[var(--color-text-secondary)] mb-[var(--spacing-xl)] text-center">
            Your vault is ready. Start adding your passwords.
          </p>
          <Button
            variant="primary"
            size="lg"
            onClick={() => useAppStore.setState({ status: "unlocked" })}
            className="w-full"
          >
            Get Started
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
          Welcome to VaultX
        </h1>
        <p className="text-[var(--font-size-sm)] text-[var(--color-text-secondary)] mb-[var(--spacing-3xl)] text-center">
          Create a master password to protect your vault.
          <br />
          Choose something strong — this is your only key.
        </p>

        <form onSubmit={handleCreateVault} className="w-full flex flex-col gap-[var(--spacing-lg)]">
          <div className="flex flex-col gap-[var(--spacing-xs)]">
            <label className="text-[var(--font-size-xs)] font-[var(--font-weight-medium)] text-[var(--color-text-secondary)]">
              Master Password
            </label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              autoFocus
              className="w-full h-10 px-[var(--spacing-md)] bg-[var(--color-bg-input)] border border-[var(--color-border)] rounded-[var(--radius-md)] text-[var(--font-size-md)] text-[var(--color-text-primary)] outline-none focus:border-[var(--color-primary)]"
              aria-label="Master password"
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
              Confirm Password
            </label>
            <input
              type="password"
              value={confirm}
              onChange={(e) => setConfirm(e.target.value)}
              className="w-full h-10 px-[var(--spacing-md)] bg-[var(--color-bg-input)] border border-[var(--color-border)] rounded-[var(--radius-md)] text-[var(--font-size-md)] text-[var(--color-text-primary)] outline-none focus:border-[var(--color-primary)]"
              aria-label="Confirm master password"
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
            {loading ? "Creating vault..." : "Create Vault"}
          </Button>
        </form>
      </div>
    </div>
  );
}
