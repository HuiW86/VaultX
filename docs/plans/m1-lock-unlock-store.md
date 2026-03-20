# M1: Lock, Unlock & Store

Status: complete
Progress: 16/16
Date: 2026-03-20
Journey: PS:§3.1 simplified (skip Recovery Kit and search — M2/M3 补齐)

## Goal

Deliver the first-time experience journey (simplified): setup master password → unlock → create entry → view & copy password (Aha moment). After M1, VaultX is a functional (minimal) password manager.

## Non-Goals (M2/M3)

- Quick Access floating panel (M2)
- Password generator UI panel (M3, M1 uses built-in default generation)
- Touch ID (M3)
- Auto-lock / brute-force delay (M3, M1 only manual lock via Cmd+L, but interfaces pre-designed for M3 extension)
- Multiple vaults (M3, M1 only default "Personal")
- Tags (M3)
- Search (M2, M1 browse only)
- Import/Export (M3)
- Recovery Kit PDF (M2)
- Clipboard external-copy cancellation (M2, needs clipboard monitoring)
- Context menu / right-click (M2)
- Sidebar collapse/expand (M3)

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| SQLCipher | `rusqlite` + `bundled-sqlcipher` | Sync API simpler; bundled compiles without system deps; Tauri SQL plugin doesn't support SQLCipher |
| Crypto | RustCrypto (`argon2` + `aes-gcm` + `zeroize`) | Pure Rust, no FFI; `aes-gcm` audited by NCC Group; OWASP recommended params |
| Argon2id params | m=19MiB, t=2, p=1 | OWASP 2025 minimum secure; unlock < 1s |
| Frontend state | Zustand | Lightweight, TS-friendly, no boilerplate |
| Titlebar | Native `TitleBarStyle::Overlay` + `traffic_light_position` | Official API, zero extra deps |
| CSS | Tailwind CSS 4 | Defined in plan doc |
| Package manager | pnpm | Defined in plan doc |
| Salt storage | `.vaultx-meta` file (plaintext JSON) with KDF version/params, atomic write (write temp → rename) | Salt cannot be inside encrypted DB; includes KDF params for forward compatibility |
| Encryption layering | SQLCipher encrypts entire DB (anti-file-read). AES-256-GCM only on field_type = password/hidden/card_number. Format: `version(1) \|\| nonce(12) \|\| ciphertext \|\| tag(16)` | Defense in depth without over-encrypting (title/username only protected by SQLCipher) |
| Clipboard ownership | Rust exclusively owns copy/timer/clear/exit-clear. React only calls `invoke('copy_to_clipboard')` + displays Toast via Rust event. No JS-side timer. | Single source of truth prevents double timers and inconsistent state |
| Lock behavior | lock() = Rust: zeroize master_key + close DB. React: reset vaultStore (clear entries/selectedEntry) + cancel all reveal timers + cancel clipboard countdown display | Ensures no plaintext remains in any memory layer after lock |
| unlock return type | `Result<UnlockResult, UnlockError>` where UnlockError = `{ kind: wrong_password \| db_corrupted \| rate_limited, message, retry_after_ms? }` | Pre-designed for M3 brute-force delay without interface changes |
| Delete semantics | Soft delete only (trashed=1). API: `trash_entry`. UI: "Move to Trash". Sidebar Trash section viewable. No permanent delete or restore in M1. | Matches product-spec expectation; prevents accidental data loss |
| zxcvbn performance | 150ms debounce on StrengthMeter calculation | Prevents input lag on long passwords |

## Encryption Architecture

```
Master Password
  │
  ▼
Argon2id (salt from .vaultx-meta, m=19MiB, t=2, p=1)
  │
  ▼
Master Key (256-bit) ──► held in Rust AppState (memory only)
  │                       zeroize on lock/exit
  │
  ├──► SQLCipher PRAGMA key
  │    Encrypts: entire DB file (all tables, all columns)
  │    Protects against: file-level read (disk theft, backup leak)
  │
  └──► AES-256-GCM per-field encryption
       Encrypts: fields WHERE field_type IN (password, hidden, card_number)
       Does NOT encrypt: title, username, url, text, otp
       Storage format: version(1) || nonce(12) || ciphertext || tag(16)
       Protects against: DB key leak (attacker gets SQLCipher key
                         but still can't read passwords without master key)
```

## .vaultx-meta File Format

```json
{
  "version": 1,
  "kdf": {
    "algorithm": "argon2id",
    "params": { "m_cost": 19456, "t_cost": 2, "p_cost": 1 },
    "salt": "<base64>"
  },
  "created_at": "2026-03-20T08:00:00Z",
  "db_path": "vault.db"
}
```

- Atomic write: write to `.vaultx-meta.tmp` then rename
- `get_app_status` checks: both files exist → `locked`; neither → `first_run`; inconsistent → `corrupted` (show ErrorState)
- KDF params stored here enable future parameter upgrades without breaking existing vaults

## Design Review Additions

### EmptyState Copy (per scenario)

| Scenario | Title | Description | CTA |
|----------|-------|-------------|-----|
| Empty EntryList (first use) | "Your vault is empty" | "Add your first login to get started" | "+ Add Item" |
| Empty EntryList (after filter) | "No items in this category" | "Try selecting a different category" | — |
| Empty Trash | "Trash is empty" | "Items you delete will appear here" | — |
| DetailPanel no selection | — | "Select an item to view details" | — |

### Category Selector (new entry)

When user clicks "+", DetailPanel shows an inline card selector: 5 horizontal cards using CategoryBadge icons + labels (Login / Card / Note / Identity / SSH Key). Click a card → load the corresponding form template. Cards use `--color-bg-hover` on hover, `--color-primary-bg` when selected. Gap: `--spacing-sm`.

### Loading States for Auth Actions

- SetupWizard "Start" button: on click → button shows spinner + "Creating vault..." + disabled. Completes → auto-transition.
- LockScreen "Unlock" button: on click → button shows spinner + disabled. If wrong password → shake + error. If correct → transition.

### Trash Entry Visual

Entries in Trash view: `opacity: 0.5`, not editable (click shows detail in read-only, no Edit button). Sidebar Trash count badge uses `--color-text-tertiary`.

### ARIA / Accessibility Baseline

All UI components in Task 2.3/3.5 must include:
- `PasswordField`: `aria-label="Password (hidden)"` when masked, `aria-label="Password (visible)"` when revealed
- `Toast`: `role="status"` + `aria-live="polite"` for non-critical, `aria-live="assertive"` for errors
- `Modal`: `role="dialog"` + `aria-modal="true"` + `aria-labelledby` pointing to title
- `Sidebar`: `role="navigation"` + `aria-label="Sidebar"`
- `EntryList`: `role="listbox"` + each `EntryCard` as `role="option"` + `aria-selected`
- `StrengthMeter`: `role="meter"` + `aria-valuenow` + `aria-valuemin="0"` + `aria-valuemax="4"` + `aria-label="Password strength"`

### Discard Changes Modal Copy

When user cancels entry edit with unsaved changes:
- Title: "Discard changes?"
- Body: "You have unsaved changes that will be lost."
- Cancel button: "Keep Editing" (secondary)
- Confirm button: "Discard" (danger)

## Task List

### Task 1: Project Skeleton + Crypto + Database (Foundation)

- [ ] **1.1** Initialize Tauri 2 + React + TypeScript project (`src-tauri/`, `src/`, `package.json`, `Cargo.toml`, `tauri.conf.json`, `tailwind.config.ts`, `vite.config.ts`, `globals.css` with DS:§2 tokens)
- [ ] **1.2** Rust crypto module: `crypto/key_derivation.rs` (Argon2id derive + verify + salt gen) + `crypto/encryption.rs` (AES-256-GCM encrypt/decrypt, version prefix, only for sensitive field_types) + unit tests (~6 tests: roundtrip, wrong key, corrupt data, empty data, nonce uniqueness, salt uniqueness)
- [ ] **1.3** Rust database module: `db/connection.rs` (SQLCipher init + open, busy timeout 5s, meta file atomic write/read/consistency check) + `db/schema.rs` (migrations: vaults, entries, fields, password_history, meta) + `db/queries.rs` (CRUD) + `state.rs` (AppState with Mutex) + unit tests (~10 tests: init, open correct/wrong key, meta consistency, CRUD roundtrips, soft delete)

**Deliverable**: `cargo test` passes (~16 tests), `pnpm tauri dev` shows empty window.

### Task 2: First-Time Setup → Unlock → Empty Main UI (Journey §3.1 steps 1-2)

- [ ] **2.1** Rust auth commands: `commands/auth.rs` — `setup_vault`, `unlock` (returns `Result<UnlockResult, UnlockError>`), `lock` (zeroize + close DB), `get_app_status` (checks meta/db consistency) + register in `lib.rs` + unit tests (~5 tests: setup creates DB+meta+vault, unlock correct/wrong, lock clears state, interrupted setup cleanup)
- [ ] **2.2** Frontend stores + types: `appStore.ts` (status: first_run/locked/unlocked/corrupted, init/setup/unlock/lock with vaultStore.reset on lock) + `vaultStore.ts` (skeleton with reset method) + `lib/commands.ts` (TypeScript types for all commands)
- [ ] **2.3** Lock screen components: `LockScreen.tsx` (DS:§6.1) + `SetupWizard.tsx` (DS:§6.1.1, 2 steps: password → start) + `PasswordField.tsx` + `StrengthMeter.tsx` (with 150ms zxcvbn debounce) + `Button.tsx` + `Input.tsx` + `ErrorState.tsx` (DS:§4.3)
- [ ] **2.4** Three-panel layout shell: `ThreePanel.tsx` (DS:§3.1) + `Sidebar.tsx` (single vault, categories, Trash, lock button) + `EntryList.tsx` (empty state) + `DetailPanel.tsx` (no-selection state) + `App.tsx` (status-based routing, corrupted → ErrorState)

**Deliverable**: User can set master password → sees empty three-panel layout. Close & reopen → unlock → same layout. `Cmd+L` locks (clears all stores + zeroizes). State machine `first_run → unlocked` and `locked → unlocked` works. Wrong password shows structured error.

### Task 3: Create Entry → View → Copy Password (Journey §3.1 steps 3-4, Aha)

- [ ] **3.1** Rust entry commands: `commands/entries.rs` — `create_entry` (encrypts sensitive fields), `get_entry` (decrypts), `list_entries` (summaries only), `update_entry` (re-encrypt + password_history), `trash_entry` (soft delete), `generate_password` (default 20-char), `copy_to_clipboard` (Rust-owned timer, emit event to frontend for Toast, on_exit hook clear) + unit tests (~3 tests: create+get roundtrip, sensitive field encryption verification, trash)
- [ ] **3.2** Frontend data layer: `vaultStore.ts` (entries, selectedEntryId, selectedEntry, categoryFilter, trashFilter, fetchEntries, selectEntry, createEntry, updateEntry, trashEntry, reset) + listen to Rust clipboard events for Toast
- [ ] **3.3** Entry list + sidebar wiring: `EntryList.tsx` (+ button, EntryCard list, selected state, keyboard ↑↓, Cmd+N) + `EntryCard.tsx` (DS:§4.6) + `Sidebar.tsx` update (category counts, click filter, Trash viewable)
- [ ] **3.4a** Entry detail + edit (Login + Note): `EntryDetail.tsx` (DS:§6.2 view mode) + `EntryForm.tsx` (template pattern, Login + Note templates per PS:§2.4, auto-generate password for new Login, title required validation, Cmd+S save, Escape cancel with discard confirm Modal)
- [ ] **3.4b** Entry detail + edit (Card + Identity): Card template (card_number formatting: 4-digit groups in edit, last-4 masked in view, CVV/PIN masked) + Identity template (name/email/phone/address)
- [ ] **3.4c** Entry detail + edit (SSH Key): SSH template (private key masked multiline, public key from private key derivation via `ssh-key` crate, fingerprint auto-calculation). Add `ssh-key` to Cargo.toml dependencies.
- [ ] **3.5** UI component library: `CopyButton.tsx` (DS:§4.5) + `Toast.tsx` (DS:§4.2 with countdown, driven by Rust events) + `Modal.tsx` (DS:§4.1, danger variant, overlay-dismiss disabled for danger) + `EmptyState.tsx` (DS:§4.3) + `ErrorState.tsx` (if not done in 2.3) + `Skeleton.tsx` (DS:§4.3) + `useClipboard.ts` hook (invoke only, no JS timer)

**Deliverable**: User can create a Login entry → see it in list → click to view details → click copy on password field → see Toast "Copied, clearing in 30s". All 5 category types work. Trash viewable. Full Aha moment walkable.

### Task 4: Tests (all layers)

- [ ] **4.1** Rust integration tests: auth flow (setup → lock → unlock → lock), entry flow (create → list → get → update → trash), edge cases (interrupted setup, wrong password, corrupt meta, DB busy timeout)
- [ ] **4.2** React component tests: LockScreen (render, wrong password shake, transition), SetupWizard (step flow, validation), EntryForm (Login template fields, title required, save/cancel), lock flow (vaultStore reset, no residual data)

**Deliverable**: ~37 tests total. `cargo test` + `pnpm test` both green.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| `rusqlite` + `bundled-sqlcipher` macOS compile failure | Blocks Task 1 | Verify compilation first in 1.3; fallback: `bundled-sqlcipher-vendored-openssl` |
| Argon2id m=19MiB slow on low-memory machines | Unlock > 1s | Benchmark on target hardware; lower to m=15MiB if needed (still above OWASP minimum) |
| Meta/DB file inconsistency | App stuck in corrupted state | `get_app_status` detects and shows ErrorState with instructions |
| Clipboard not cleared on app exit within 30s | Security gap | Tauri `on_exit` hook clears clipboard immediately |
| Tailwind CSS 4 + CSS variable tokens coexistence | Style conflicts | Define `:root` vars in `globals.css`, map to Tailwind via `theme.extend.colors` |
| DB file locked by external process | Operations fail silently | rusqlite busy timeout 5s + ErrorState with retry |
| `ssh-key` crate adds dependency for SSH type | Cargo.toml bloat | Only used in 3.4c, can be feature-gated if needed |

## Execution Order

```
Task 1 (Foundation)       Task 2 (Lock/Unlock)        Task 3 (CRUD + Aha)           Task 4 (Tests)
──────────────────       ─────────────────────        ──────────────────────        ──────────────
1.1 Project init    →    2.1 auth commands       →    3.1 entry commands       →    4.1 Rust tests
1.2 crypto module   →    2.2 frontend stores     →    3.2 vaultStore           →    4.2 React tests
1.3 db module       →    2.3 LockScreen + Setup  →    3.3 EntryList + Sidebar
                          2.4 ThreePanel shell    →    3.4a Login + Note
                                                       3.4b Card + Identity
                                                       3.4c SSH Key
                                                       3.5 UI components
```

Strictly sequential — no parallelism across tasks.

## File Manifest

### Rust (`src-tauri/src/`)

| File | Purpose |
|------|---------|
| `main.rs` | Tauri entry, window config (titlebar, min size), on_exit clipboard clear |
| `lib.rs` | Command registration via `generate_handler![]` |
| `state.rs` | `AppState { db, master_key (Zeroizing), meta_path }` wrapped in `Mutex` |
| `crypto/mod.rs` | Module exports |
| `crypto/key_derivation.rs` | Argon2id key derivation + salt generation + verify |
| `crypto/encryption.rs` | AES-256-GCM encrypt/decrypt (version-prefixed format) |
| `db/mod.rs` | Module exports |
| `db/connection.rs` | SQLCipher connection + meta file management (atomic write, consistency check, busy timeout) |
| `db/schema.rs` | Table creation SQL (migrations) |
| `db/queries.rs` | CRUD for vaults, entries, fields, password_history |
| `commands/mod.rs` | Module exports |
| `commands/auth.rs` | setup_vault, unlock (structured errors), lock (full cleanup), get_app_status |
| `commands/entries.rs` | create_entry, get_entry, list_entries, update_entry, trash_entry, generate_password, copy_to_clipboard (Rust-owned timer + event emit) |

### React (`src/`)

| File | Purpose |
|------|---------|
| `main.tsx` | React entry |
| `App.tsx` | Root component, status-based view switching (first_run/locked/unlocked/corrupted) |
| `stores/appStore.ts` | App state + lock triggers vaultStore.reset |
| `stores/vaultStore.ts` | Entry data + CRUD + reset (security clear) |
| `lib/commands.ts` | TypeScript type defs for all Tauri commands + error types |
| `hooks/useClipboard.ts` | invoke copy_to_clipboard + listen Rust events for Toast |
| `hooks/useInvoke.ts` | Tauri invoke wrapper with error handling |
| `styles/globals.css` | Tailwind entry + CSS variable tokens (DS:§2) |
| `components/layout/ThreePanel.tsx` | Three-panel grid container |
| `components/layout/Sidebar.tsx` | Vault, categories, Trash, lock button |
| `components/layout/EntryList.tsx` | Entry card list + add button |
| `components/layout/DetailPanel.tsx` | Detail/edit view container |
| `components/lock/LockScreen.tsx` | Unlock UI (DS:§6.1) |
| `components/lock/SetupWizard.tsx` | First-time setup (DS:§6.1.1) |
| `components/entry/EntryCard.tsx` | List item (DS:§4.6) |
| `components/entry/EntryDetail.tsx` | View mode (DS:§6.2) |
| `components/entry/EntryForm.tsx` | Edit mode (DS:§6.3), template pattern for 5 category types |
| `components/ui/Button.tsx` | Button variants (DS:§4.7) |
| `components/ui/Input.tsx` | Input with states (DS:§4.7) |
| `components/ui/PasswordField.tsx` | Mask/reveal/copy (DS:§4.5) |
| `components/ui/StrengthMeter.tsx` | Password strength bar with 150ms debounce (DS:§4.5) |
| `components/ui/CopyButton.tsx` | Copy + checkmark feedback (DS:§4.5) |
| `components/ui/Toast.tsx` | Bottom-center toast, Rust event driven (DS:§4.2) |
| `components/ui/Modal.tsx` | Dialog, danger variant, overlay-dismiss control (DS:§4.1) |
| `components/ui/Skeleton.tsx` | Loading placeholder (DS:§4.3) |
| `components/ui/EmptyState.tsx` | No-data state (DS:§4.3) |
| `components/ui/ErrorState.tsx` | Error + retry (DS:§4.3) |

## Review Decisions Log

| # | Issue | Decision | Rationale |
|---|-------|----------|-----------|
| 1 | Lock only cosmetic | **1A**: Full lock cleanup (Rust zeroize + React store reset + cancel timers) | Security: no plaintext in any memory after lock |
| 2 | Encryption layers undefined | **2A**: AES-GCM only for password/hidden/card_number fields | Balance: defense-in-depth without over-encrypting |
| 3 | Clipboard dual ownership | **3A**: Rust exclusively owns clipboard logic | Single source of truth prevents double timers |
| 4 | Meta file brittle | **4A**: Strengthen with KDF params + atomic write + consistency check | Forward compatibility + crash safety |
| 5 | Task 3.4 scope underestimated | **5B**: Split into 3.4a (Login+Note) + 3.4b (Card+Identity) + 3.4c (SSH) | Decomposed scope, each sub-task independently deliverable |
| 6 | Delete semantics unclear | **6A**: Soft delete + viewable Trash | Matches product-spec, prevents accidental data loss |
| 7 | Auth interface forward compat | **7A**: Pre-design UnlockError struct with rate_limited variant | Zero rework when M3 adds brute-force delay |
| 8 | ErrorState missing | **8A**: Add ErrorState component + error flows in key views | Security app must handle errors visibly, not silently |
| 9 | Test coverage weak | **9A**: Rust full coverage (~24) + React key paths (~13) = ~37 tests | Security-critical app needs thorough testing |
| 10 | zxcvbn input lag | **10A**: 150ms debounce on StrengthMeter | Prevents typing stutter on long passwords |
