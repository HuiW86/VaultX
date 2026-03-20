# M3: Security & Polish

Status: complete
Progress: 11/11
Date: 2026-03-20
Journey: PS:§3.2 acceleration + PS:§5.3 auto-lock + PS:§3.1 step 3 recovery

## Goal

Complete MVP security layer (Touch ID, auto-lock, brute-force delay) and experience layer (Settings UI, password generator panel, Recovery Kit). After M3, VaultX is a daily-driveable password manager.

## Non-Goals (Backlog)

- Multiple vaults (default "Personal" sufficient for MVP)
- Tags
- Import/Export (manual entry for MVP, P1 feature)
- Sidebar collapse/expand
- Quick Access context-aware suggestions
- Clipboard external-copy cancellation
- Browser extension

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Touch ID implementation | `security-framework` crate (macOS LocalAuthentication) | Native API, stores master_key in Keychain with biometric protection |
| Recovery Kit format | Plain text .txt file (not PDF) | Zero extra deps; printable; contains base32 recovery key |
| Settings persistence | `.vaultx-settings` JSON file | Separate from encrypted DB; readable before unlock for theme/auto-lock config |
| Password word list | EFF diceware (embedded ~50KB) | Offline, no network; standard word list |
| Auto-lock mechanism | Background thread polling every 30s | Simple, reliable; macOS sleep/lock via NSNotification |

## Task List

### Task 1: Settings UI + Persistence (Infrastructure)

- [x] **1.1** Rust settings commands: `commands/settings.rs` (get_settings/save_settings, `.vaultx-settings` JSON) + settingsStore.ts — Journey: all
- [x] **1.2** SettingsPanel.tsx (DS:§6.4): General/Security/Appearance/Data/About sections, immediate-apply switches/selects — Journey: all

**Deliverable**: Sidebar Settings gear opens settings panel, all config persists.

### Task 2: Auto-lock + Brute-force Delay (Security) — Journey: PS:§5.3

- [x] **2.1** Rust state expansion: failed_attempts (persisted) + last_activity (memory) + auto-lock timer thread (30s poll) + macOS sleep/lock event → immediate lock — Journey: §5.3
- [x] **2.2** Brute-force delay in unlock: check failed_attempts (5→5s, 6→15s, 7→30s, 8+→60s), return UnlockError.rate_limited + LockScreen countdown UI — Journey: §5.4
- [x] **2.3** Frontend auto-lock integration: App.tsx listens "app:locked" event → transition to LockScreen, activity heartbeat on user interaction — Journey: §5.3

### Task 3: Touch ID (Biometric Auth) — Journey: PS:§3.2

- [x] **3.1** Rust Touch ID: `commands/security.rs` (is_touch_id_available, setup_touch_id → Keychain store, disable_touch_id, unlock_biometric) — Journey: §3.2
- [x] **3.2** Frontend Touch ID: LockScreen Touch ID button + Settings toggle + auto-trigger on mount — Journey: §3.2

### Task 4: Password Generator Panel — Journey: PS:§3.3

- [x] **4.1** PasswordGenerator.tsx: expandable panel (length slider 8-128, charset checkboxes, random/words mode, word separator, live preview + StrengthMeter, Use/Refresh/Copy buttons, history last 20) — Journey: §3.3
- [x] **4.2** Rust generate_password upgrade: accept params (length, charset, mode, separator), embed EFF diceware word list, return { password, strength } — Journey: §3.3

### Task 5: Recovery Kit — Journey: PS:§3.1 step 3

- [x] **5.1** Rust recovery: generate 128-bit recovery key, AES-GCM encrypt master_key with it, store in meta, generate .txt content (base32 key + instructions) — Journey: §3.1
- [x] **5.2** Frontend recovery: SetupWizard 3-step (password → recovery kit download → start) + LockScreen "Forgot password?" → enter recovery key → reset password — Journey: §3.1

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| macOS LocalAuthentication API complexity | Touch ID fails | security-framework crate wraps it; fallback: password-only |
| Keychain master_key storage security | Attack surface | kSecAccessControlBiometryCurrentSet limits to current fingerprints |
| macOS sleep event detection | Needs ObjC bridge | objc2 crate or security-framework NSNotification support |
| EFF word list adds 50KB to binary | Acceptable | Embedded at compile time, no runtime cost |

## Execution Order

```
Task 1 (Settings)    → Task 2 (Auto-lock)  → Task 3 (Touch ID)  → Task 4 (PwGen) → Task 5 (Recovery)
1.1 settingsStore       2.1 state+timer        3.1 Rust Touch ID     4.1 UI panel     5.1 Rust gen
1.2 SettingsPanel       2.2 brute-force        3.2 Frontend          4.2 Rust upgrade  5.2 Frontend
                        2.3 frontend lock
```

Strictly sequential. Task 2 depends on Task 1 settings. Task 3 depends on Task 2 state.
