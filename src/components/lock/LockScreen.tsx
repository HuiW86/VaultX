import { useState, useCallback, useEffect, useRef, type FormEvent } from "react";
import { motion } from "framer-motion";
import { Fingerprint, Lock } from "lucide-react";
import { Button } from "../ui/Button";
import { useAppStore } from "../../stores/appStore";
import { api } from "../../lib/commands";

export function LockScreen() {
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [shake, setShake] = useState(false);
  const [countdown, setCountdown] = useState(0);
  const [touchIdEnabled, setTouchIdEnabled] = useState(false);
  const [biometricLoading, setBiometricLoading] = useState(false);
  const [showRecovery, setShowRecovery] = useState(false);
  const [recoveryKey, setRecoveryKey] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmNew, setConfirmNew] = useState("");
  const [recoveryLoading, setRecoveryLoading] = useState(false);
  const [recoveryError, setRecoveryError] = useState("");
  const countdownRef = useRef<ReturnType<typeof setInterval>>();
  const error = useAppStore((s) => s.error);
  const retryAfterMs = useAppStore((s) => s.retryAfterMs);
  const unlock = useAppStore((s) => s.unlock);
  const clearError = useAppStore((s) => s.clearError);

  // Check if Touch ID is enabled in settings
  useEffect(() => {
    api.getSettings().then((s) => setTouchIdEnabled(s.touch_id_enabled)).catch(() => {});
  }, []);

  // Auto-trigger Touch ID on mount if enabled
  useEffect(() => {
    if (!touchIdEnabled) return;
    handleBiometricUnlock();
  }, [touchIdEnabled]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleBiometricUnlock = useCallback(async () => {
    if (biometricLoading) return;
    setBiometricLoading(true);
    try {
      await api.unlockBiometric();
      useAppStore.setState({ status: "unlocked", error: null });
    } catch {
      // Touch ID failed or cancelled — user can use password
    } finally {
      setBiometricLoading(false);
    }
  }, [biometricLoading]);

  // Countdown timer for rate limiting
  useEffect(() => {
    if (countdown <= 0) {
      clearInterval(countdownRef.current);
      return;
    }
    countdownRef.current = setInterval(() => {
      setCountdown((c) => {
        if (c <= 1) {
          clearInterval(countdownRef.current);
          return 0;
        }
        return c - 1;
      });
    }, 1000);
    return () => clearInterval(countdownRef.current);
  }, [countdown > 0]); // only re-run when transitioning from 0 to >0

  const handleSubmit = useCallback(
    async (e: FormEvent) => {
      e.preventDefault();
      if (!password || loading || countdown > 0) return;
      setLoading(true);
      try {
        await unlock(password);
      } catch (err: any) {
        setShake(true);
        setTimeout(() => setShake(false), 500);
        // Check for rate_limited response with retry_after_ms (from store or error)
        const retryMs = err?.retry_after_ms ?? retryAfterMs;
        if (retryMs && retryMs > 0) {
          setCountdown(Math.ceil(retryMs / 1000));
        }
      } finally {
        setLoading(false);
      }
    },
    [password, loading, unlock, countdown]
  );

  return (
    <div className="h-full flex flex-col items-center justify-center bg-[var(--color-bg-app)]">
      <div className="flex flex-col items-center w-80">
        <Lock
          size={48}
          className="text-[var(--color-primary)] mb-[var(--spacing-lg)]"
        />
        <h1 className="text-[var(--font-size-display)] font-[var(--font-weight-bold)] text-[var(--color-text-primary)] mb-[var(--spacing-3xl)]"
            style={{ fontFamily: "var(--font-display)" }}>
          VaultX
        </h1>

        <motion.form
          onSubmit={handleSubmit}
          animate={shake ? { x: [0, -10, 10, -10, 10, 0] } : {}}
          transition={{ duration: 0.4 }}
          className="w-full flex flex-col gap-[var(--spacing-lg)]"
        >
          <input
            type="password"
            value={password}
            onChange={(e) => {
              setPassword(e.target.value);
              if (error) clearError();
            }}
            placeholder="Master password"
            autoFocus
            className="w-full h-10 px-[var(--spacing-md)] bg-[var(--color-bg-input)] border border-[var(--color-border)] rounded-[var(--radius-md)] text-[var(--font-size-md)] text-[var(--color-text-primary)] placeholder:text-[var(--color-text-tertiary)] outline-none focus:border-[var(--color-primary)]"
            aria-label="Master password"
          />

          {error && (
            <p className="text-[var(--font-size-xs)] text-[var(--color-error)] text-center">
              {error}
            </p>
          )}

          {countdown > 0 && (
            <p className="text-[var(--font-size-xs)] text-[var(--color-text-tertiary)] text-center">
              Try again in {countdown}s
            </p>
          )}

          <Button
            type="submit"
            variant="primary"
            size="lg"
            loading={loading}
            disabled={countdown > 0}
            className="w-full"
          >
            {countdown > 0 ? `Wait ${countdown}s` : "Unlock"}
          </Button>
        </motion.form>

        {touchIdEnabled && (
          <button
            onClick={handleBiometricUnlock}
            disabled={biometricLoading}
            className="mt-[var(--spacing-lg)] flex items-center gap-[var(--spacing-sm)] text-[var(--color-text-secondary)] hover:text-[var(--color-primary)] transition-colors cursor-pointer disabled:opacity-40"
          >
            <Fingerprint size={20} />
            <span className="text-[var(--font-size-sm)]">
              {biometricLoading ? "Authenticating..." : "Use Touch ID"}
            </span>
          </button>
        )}

        <button
          onClick={() => setShowRecovery(!showRecovery)}
          className="mt-[var(--spacing-lg)] text-[var(--font-size-xs)] text-[var(--color-text-tertiary)] hover:text-[var(--color-text-secondary)] cursor-pointer"
        >
          Forgot password?
        </button>

        {showRecovery && (
          <div className="mt-[var(--spacing-md)] w-full flex flex-col gap-[var(--spacing-md)] p-[var(--spacing-md)] border border-[var(--color-border)] rounded-[var(--radius-lg)] bg-[var(--color-bg-elevated)]">
            <p className="text-[var(--font-size-xs)] text-[var(--color-text-secondary)]">
              Enter your recovery key and set a new master password.
            </p>
            <input
              type="text"
              value={recoveryKey}
              onChange={(e) => { setRecoveryKey(e.target.value); setRecoveryError(""); }}
              placeholder="Recovery key (e.g. ABCD-EFGH-...)"
              className="w-full h-10 px-[var(--spacing-md)] bg-[var(--color-bg-input)] border border-[var(--color-border)] rounded-[var(--radius-md)] text-[var(--font-size-sm)] text-[var(--color-text-primary)] placeholder:text-[var(--color-text-tertiary)] outline-none focus:border-[var(--color-primary)]"
              style={{ fontFamily: "var(--font-mono)" }}
            />
            <input
              type="password"
              value={newPassword}
              onChange={(e) => { setNewPassword(e.target.value); setRecoveryError(""); }}
              placeholder="New master password"
              className="w-full h-10 px-[var(--spacing-md)] bg-[var(--color-bg-input)] border border-[var(--color-border)] rounded-[var(--radius-md)] text-[var(--font-size-md)] text-[var(--color-text-primary)] placeholder:text-[var(--color-text-tertiary)] outline-none focus:border-[var(--color-primary)]"
            />
            <input
              type="password"
              value={confirmNew}
              onChange={(e) => { setConfirmNew(e.target.value); setRecoveryError(""); }}
              placeholder="Confirm new password"
              className="w-full h-10 px-[var(--spacing-md)] bg-[var(--color-bg-input)] border border-[var(--color-border)] rounded-[var(--radius-md)] text-[var(--font-size-md)] text-[var(--color-text-primary)] placeholder:text-[var(--color-text-tertiary)] outline-none focus:border-[var(--color-primary)]"
            />
            {recoveryError && (
              <p className="text-[var(--font-size-xs)] text-[var(--color-error)]">{recoveryError}</p>
            )}
            <Button
              variant="primary"
              size="md"
              loading={recoveryLoading}
              onClick={async () => {
                if (!recoveryKey.trim()) { setRecoveryError("Recovery key is required"); return; }
                if (newPassword.length < 8) { setRecoveryError("Password must be at least 8 characters"); return; }
                if (newPassword !== confirmNew) { setRecoveryError("Passwords don't match"); return; }
                setRecoveryLoading(true);
                try {
                  await api.recoverWithKey(recoveryKey.trim(), newPassword);
                  useAppStore.setState({ status: "unlocked", error: null });
                } catch (e) {
                  setRecoveryError(typeof e === "string" ? e : "Recovery failed. Check your recovery key.");
                } finally {
                  setRecoveryLoading(false);
                }
              }}
              className="w-full"
            >
              Reset Password
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
