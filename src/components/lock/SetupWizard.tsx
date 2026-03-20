import { useState, useCallback, type FormEvent } from "react";
import { Shield } from "lucide-react";
import { Button } from "../ui/Button";
import { StrengthMeter } from "../ui/StrengthMeter";
import { useAppStore } from "../../stores/appStore";

export function SetupWizard() {
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [loading, setLoading] = useState(false);
  const [errors, setErrors] = useState<{ password?: string; confirm?: string }>({});
  const setup = useAppStore((s) => s.setup);
  const globalError = useAppStore((s) => s.error);

  const validate = useCallback(() => {
    const errs: typeof errors = {};
    if (password.length < 8) errs.password = "Password must be at least 8 characters";
    if (password !== confirm) errs.confirm = "Passwords don't match";
    setErrors(errs);
    return Object.keys(errs).length === 0;
  }, [password, confirm]);

  const handleSubmit = useCallback(
    async (e: FormEvent) => {
      e.preventDefault();
      if (!validate() || loading) return;
      setLoading(true);
      try {
        await setup(password);
      } catch {
        // Error shown via globalError
      } finally {
        setLoading(false);
      }
    },
    [password, validate, loading, setup]
  );

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

        <form onSubmit={handleSubmit} className="w-full flex flex-col gap-[var(--spacing-lg)]">
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
