# VaultX

Local-first password manager for macOS. Your passwords never leave your machine.

本地优先的 macOS 密码管理器。你的密码永远不会离开你的设备。

## Why VaultX? / 为什么选择 VaultX？

Mainstream password managers (1Password, LastPass) force cloud subscriptions. VaultX keeps everything local with a polished, modern UI inspired by 1Password 8.

主流密码管理器（1Password、LastPass）强制使用云订阅。VaultX 将所有数据保留在本地，同时提供媲美 1Password 8 的精致现代界面。

## Features / 功能特性

- **Local-only storage / 纯本地存储** — SQLCipher encrypted database, no cloud dependency / SQLCipher 加密数据库，无云端依赖
- **Strong encryption / 强加密** — Argon2id key derivation + AES-256-GCM field-level encryption / Argon2id 密钥派生 + AES-256-GCM 字段级加密
- **1Password-style UI / 1Password 风格界面** — Three-panel layout with dark theme / 三栏布局，暗色主题
- **Touch ID unlock / 触控 ID 解锁** — macOS biometric authentication via Keychain / 通过钥匙串实现 macOS 生物识别认证
- **Password generator / 密码生成器** — Random and diceware (passphrase) modes with strength meter / 随机和 Diceware（助记短语）模式，带强度指示
- **Auto-lock / 自动锁定** — Configurable timeout with sleep detection / 可配置超时时间，支持睡眠检测
- **Clipboard security / 剪贴板安全** — Auto-clear after configurable delay, Rust-owned (no JS timers) / 可配置延迟后自动清除，Rust 原生控制
- **Search / 搜索** — Instant fuzzy search across all entries / 全条目即时模糊搜索
- **Recovery kit / 恢复工具包** — 128-bit recovery key for master password reset / 128 位恢复密钥，用于重置主密码
- **Brute-force protection / 暴力破解防护** — Exponential backoff on failed attempts / 失败尝试指数退避
- **i18n / 国际化** — English and Chinese (Simplified) supported / 支持英文和简体中文

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

## Author / 作者

Built by [@wh759705-creator](https://github.com/wh759705-creator), creator of [SkillNav.dev](https://skillnav.dev) — a curated navigation platform for AI Agent tools, featuring 100+ quality Skills and 3900+ MCP tools for developers.

由 [@wh759705-creator](https://github.com/wh759705-creator) 开发，同时也是 [SkillNav.dev](https://skillnav.dev) 的作者 — 一个面向开发者的 AI 智能体工具导航平台，精选 100+ 优质 Skills 和 3900+ MCP 工具。

## License

[MIT](LICENSE)
