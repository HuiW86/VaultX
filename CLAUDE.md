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
  state.rs       — AppState

src/
  components/    — layout/ lock/ entry/ search/ settings/ ui/
  stores/        — Zustand stores
  lib/           — commands.ts (API types + invoke wrappers)
  styles/        — globals.css (design tokens)
```

### Design Spec Reference
Before generating UI code, check:
1. Token values: `docs/design-spec.md` §2
2. Component specs: §4
3. Interaction patterns: §5
4. Composition patterns: §6
5. Shared vocabulary: `docs/product-spec.md` §0.2

## Plans

See `docs/plans/README.md` for implementation plans and progress.
