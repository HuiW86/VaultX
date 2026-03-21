# VaultX

Local-first password manager for macOS. Your passwords never leave your machine.

## Why VaultX?

Mainstream password managers (1Password, LastPass) force cloud subscriptions. VaultX keeps everything local with a polished, modern UI inspired by 1Password 8.

## Features

- **Local-only storage** — SQLCipher encrypted database, no cloud dependency
- **Strong encryption** — Argon2id key derivation + AES-256-GCM field-level encryption
- **1Password-style UI** — Three-panel layout with dark theme
- **Touch ID unlock** — macOS biometric authentication via Keychain
- **Password generator** — Random and diceware (passphrase) modes with strength meter
- **Auto-lock** — Configurable timeout with sleep detection
- **Clipboard security** — Auto-clear after configurable delay, Rust-owned (no JS timers)
- **Search** — Instant fuzzy search across all entries
- **Recovery kit** — 128-bit recovery key for master password reset
- **Brute-force protection** — Exponential backoff on failed attempts
- **i18n** — English and Chinese (Simplified) supported

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop framework | [Tauri 2.0](https://v2.tauri.app/) |
| Frontend | React 18 + TypeScript + Tailwind CSS 4 |
| Backend | Rust |
| Database | SQLCipher (SQLite with full-database encryption) |
| Crypto | Argon2id (KDF) + AES-256-GCM (field encryption) |
| State management | Zustand (frontend) + Mutex\<AppState\> (Rust) |

## Prerequisites

- macOS (Apple Silicon or Intel)
- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) >= 18
- [pnpm](https://pnpm.io/)

## Getting Started

```bash
# Clone the repo
git clone https://github.com/wh759705-creator/VaultX.git
cd VaultX

# Install dependencies
pnpm install

# Run in development mode
npx tauri dev

# Build for production
npx tauri build
```

## Architecture

```
src-tauri/src/
  crypto/        — Argon2id + AES-256-GCM
  db/            — SQLCipher connection, schema, queries, search
  commands/      — Tauri IPC command handlers
  state.rs       — AppState (db, master_key, last_activity)

src/
  components/    — React UI (layout, lock, entry, search, settings)
  stores/        — Zustand stores (app, vault, search, settings)
  i18n/          — Internationalization (en, zh-CN)
  lib/           — API types + invoke wrappers
  styles/        — Design tokens (CSS custom properties)
```

## Security Model

1. **Database encryption**: SQLCipher encrypts the entire DB file at rest
2. **Field encryption**: Sensitive fields (`password`, `hidden`, `card_number`) are additionally encrypted with AES-256-GCM
3. **Key derivation**: Master password → Argon2id → 256-bit key
4. **Memory safety**: Master key wrapped in `Zeroizing<[u8; 32]>`, zeroized on lock/exit
5. **Lock clears everything**: Rust state (zeroize + close DB) + React stores (reset) + clipboard

## License

[MIT](LICENSE)
