# VaultX — Project Rules

## Overview

Local-first password manager for macOS. Tauri 2.0 + React + TypeScript + Rust.

## Architecture

- **Rust backend** (`src-tauri/src/`): crypto, db (SQLCipher), commands (Tauri IPC), state
- **React frontend** (`src/`): components, stores (Zustand), hooks
- **Design system**: `docs/design-spec.md` (tokens, components, compositions)
- **Product spec**: `docs/product-spec.md` (IA, journeys, state machine)

## Key Conventions

### Encryption Layering
- SQLCipher encrypts the entire DB file (anti-file-read)
- AES-256-GCM encrypts only `password`, `hidden`, `card_number` field values
- Format: `version(1) || nonce(12) || ciphertext || tag(16)`

### Security Rules
- `master_key` wrapped in `Zeroizing<[u8; 32]>`, zeroized on lock/exit
- Lock clears: Rust state (zeroize + close DB) + React stores (reset) + reveal timers + clipboard
- No sensitive data in console.log, DOM attributes, or error messages
- Clipboard exclusively owned by Rust (no JS timers)

### State Management
- Rust: `Mutex<AppState>` registered via Tauri `.manage()`
- React: Zustand stores (`appStore`, `vaultStore`, `searchStore`, `settingsStore`)
- IPC: `invoke()` for commands, `listen()` for events (clipboard, auto-lock)

### Naming
- Rust: snake_case for commands, modules, functions
- React: PascalCase components, camelCase hooks/stores
- CSS: Tailwind utilities + `var(--token-name)` for design tokens
- Commits: Conventional Commits, English

### File Organization
```
src-tauri/src/
  crypto/        — Argon2id + AES-256-GCM
  db/            — SQLCipher connection, schema, queries, search
  commands/      — Tauri IPC command handlers
    auth.rs      — get_app_status, setup_vault, unlock, lock, heartbeat
    entries.rs   — CRUD, generate_password (random + diceware), clipboard
    search.rs    — search_entries, recent_entries
    settings.rs  — get_settings/save_settings (.vaultx-settings JSON)
    security.rs  — Touch ID (Keychain + LAContext via objc2)
    recovery.rs  — generate_recovery_kit, recover_with_key
  state.rs       — AppState (db, master_key, last_activity)
src-tauri/data/  — EFF diceware word list (embedded at compile time)

src/
  components/    — layout/ lock/ entry/ search/ settings/ ui/
  stores/        — Zustand stores (app, vault, search, settings)
  i18n/          — i18n system (en.ts, zh-CN.ts, context + hook)
  lib/           — commands.ts (API types + invoke wrappers)
  styles/        — globals.css (design tokens)
```

### Internationalization (i18n)
- Lightweight: React Context + `useTranslation()` hook, zero dependencies
- Translations: `src/i18n/en.ts` (source of truth), `src/i18n/zh-CN.ts`
- All UI text uses `t("key")` calls, never hardcoded strings
- Language setting persisted in `.vaultx-settings` via `VaultxSettings.language`
- `I18nProvider` wraps the app in `App.tsx`, reads locale from settingsStore
- Tests use `renderWithI18n()` wrapper from `src/test/test-utils.tsx`

### Design Spec Reference
Before generating UI code, check:
1. Token values: `docs/design-spec.md` §2
2. Component specs: §4
3. Interaction patterns: §5
4. Composition patterns: §6
5. Shared vocabulary: `docs/product-spec.md` §0.2

### i18n Pitfalls
- Lock/auto-lock resets stores — `settingsStore.reset()` must preserve `language` (non-sensitive)
- Never gate i18n locale on `settingsLoaded` — use `language` directly (has `?? "en"` fallback)
- Static data outside components (templates, metadata) use i18n keys, translate at render via `t()`
- DB stores translated label text — editing existing entries needs reverse lookup (`labelToKey` map)

### Type Safety
- i18n keys in static data (templates, metadata) must use `TranslationKey` type, not `string`
- Import `type TranslationKey` from `../../i18n` when defining labelKey fields
- Component local state from props: add `key` prop to force remount when prop changes

## Open Source

- **GitHub**: https://github.com/HuiW86/VaultX (migrated from wh759705-creator)
- **License**: MIT
- **Author**: Eric Wang (WeChat: 759705, [SkillNav.dev](https://skillnav.dev))
- **Dev command**: `npx tauri dev` (not `npm run tauri dev`)
- **Build**: `npx tauri build` (outputs `.dmg` for macOS, `.exe`/`.msi` for Windows)

## CI/CD

- **Release workflow**: `.github/workflows/release.yml`
- **Trigger**: push tag `v*` → builds macOS (aarch64 + x86_64) + Windows (x86_64)
- **Action**: `tauri-apps/tauri-action@v0` auto-creates GitHub Release with bundled artifacts
- macOS-only deps (`core-foundation`, `security-framework-sys`, `objc2`) behind `cfg(target_os = "macos")` in Cargo.toml

## Plans

See `docs/plans/README.md` for implementation plans and progress.
