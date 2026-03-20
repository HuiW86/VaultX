import { useState, useCallback, type FormEvent } from "react";
import { motion } from "framer-motion";
import { Lock } from "lucide-react";
import { Button } from "../ui/Button";
import { useAppStore } from "../../stores/appStore";

export function LockScreen() {
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [shake, setShake] = useState(false);
  const error = useAppStore((s) => s.error);
  const unlock = useAppStore((s) => s.unlock);
  const clearError = useAppStore((s) => s.clearError);

  const handleSubmit = useCallback(
    async (e: FormEvent) => {
      e.preventDefault();
      if (!password || loading) return;
      setLoading(true);
      try {
        await unlock(password);
      } catch {
        setShake(true);
        setTimeout(() => setShake(false), 500);
      } finally {
        setLoading(false);
      }
    },
    [password, loading, unlock]
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

          <Button type="submit" variant="primary" size="lg" loading={loading} className="w-full">
            Unlock
          </Button>
        </motion.form>
      </div>
    </div>
  );
}
