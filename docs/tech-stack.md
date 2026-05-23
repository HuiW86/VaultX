---
type: tech-stack
project: VaultX
version: 0.1.0
snapshot_date: 2026-05-24
status: stable
audience: [ai, human]
---

# VaultX 技术栈快照

> 依赖与版本事实清单，AI 据此决定 import 与 API。**不写选型理由（→ ADR）、不写禁用项（→ constitution）、不写架构动机（→ product-spec）。**

VaultX 是本地优先密码管理器（macOS + Windows 桌面），仓库 [HuiW86/VaultX](https://github.com/HuiW86/VaultX) (MIT)，架构为 Tauri 2 桌面壳 + React 19 前端 + Rust 后端，当前版本 `v0.1.0`（2026-03-24 首发）。

---

## 1. 运行时

| 层 | 版本 | 锚定位置 |
|---|---|---|
| Rust | edition `2021`，MSRV `1.77.2` | `src-tauri/Cargo.toml` `rust-version` |
| Node.js | `20.x` | CI `actions/setup-node@v4` |
| pnpm | `9.x` | CI `pnpm/action-setup@v4` |
| macOS 目标 | `aarch64-apple-darwin` + `x86_64-apple-darwin` | CI matrix |
| Windows 目标 | `x86_64-pc-windows-msvc` | CI matrix |

无 `.nvmrc` / `engines` —— 本地 Node/pnpm 版本需手动对齐 CI。

---

## 2. 桌面壳 — Tauri 2

| 包 | 版本 | 用途 |
|---|---|---|
| `tauri` | `2` (features: `macos-private-api`) | 主框架 |
| `tauri-build` | `2` | build script |
| `tauri-plugin-log` | `2` | 结构化日志 |
| `tauri-plugin-global-shortcut` | `2` | 全局快捷键 |
| `@tauri-apps/api` | `^2.10.1` | 前端 IPC SDK |
| `@tauri-apps/plugin-global-shortcut` | `^2.3.1` | 快捷键前端绑定 |
| `@tauri-apps/cli` | `^2.10.1` | `tauri dev/build` CLI |

**配置**（`src-tauri/tauri.conf.json`）：identifier `com.vaultx.app` · 主窗口 1100×700（min 900×600）· `titleBarStyle: Overlay` · `macOSPrivateApi: true` · `csp: null` · devUrl `http://localhost:5173` · `beforeDevCommand: pnpm dev` · `beforeBuildCommand: pnpm build`。

**Capabilities**（`src-tauri/capabilities/default.json`）：窗口 `main` + `quickaccess`；权限 `core:window:*` (show/hide/focus/close/center/visible) + `core:event:default` + `global-shortcut:*` (register/unregister)。

---

## 3. 前端框架

| 包 | 版本 |
|---|---|
| Vite | `^8.0.1` |
| `@vitejs/plugin-react` | `^6.0.1` |
| React | `^19.2.4` |
| react-dom | `^19.2.4` |
| TypeScript | `~5.9.3` |
| Tailwind CSS | `^4.2.2` |
| `@tailwindcss/vite` | `^4.2.2` |

ESM 项目（`"type": "module"`）；构建链 `tsc -b && vite build`。

---

## 4. 前端运行时依赖

| 包 | 版本 | 用途 |
|---|---|---|
| `zustand` | `^5.0.12` | 全局状态（4 store：app / vault / search / settings） |
| `framer-motion` | `^12.38.0` | 解锁/列表过渡动画 |
| `lucide-react` | `^0.577.0` | 图标系统 |
| `zxcvbn` | `^4.4.2` | 密码强度评分 |

**i18n**：自研 React Context + `useTranslation()` Hook，零运行时依赖；语种 `en` / `zh-CN`，源文件 `src/i18n/*.ts`，语言持久化于 `.vaultx-settings`。

---

## 5. Rust 后端（`src-tauri/Cargo.toml`）

| 类别 | crate | 版本 | 用途 |
|---|---|---|---|
| 加密 | `argon2` | `0.5` | KDF（主密钥派生） |
| 加密 | `aes-gcm` | `0.10` | 字段级 AEAD |
| 加密 | `zeroize` | `1` (derive) | 敏感内存清零 |
| 加密 | `rand` | `0.8` | nonce / salt |
| 加密 | `base64` | `0.22` | 编码 |
| 数据 | `rusqlite` | `0.32` | features: `bundled-sqlcipher`（整库加密） |
| IPC | `serde` / `serde_json` | `1` | 序列化 |
| 日志 | `log` + `tauri-plugin-log` | `0.4` / `2` | 结构化日志 |
| 工具 | `uuid` | `1` (v4) | 条目 ID |
| 工具 | `chrono` | `0.4` (serde) | 时间戳 |
| 工具 | `dirs` | `6` | 跨平台用户路径 |
| 系统 | `arboard` | `3` | 剪贴板（Rust 独占） |
| macOS¹ | `core-foundation` | `0.9` | Keychain 桥 |
| macOS¹ | `security-framework-sys` | `2` | Keychain 原生 API |
| macOS¹ | `objc2` | `0.5` | LAContext / Touch ID |
| Dev | `tempfile` | `3` | 测试隔离目录 |

¹ macOS 行受 `cfg(target_os = "macos")` 条件编译保护，其他平台不引入。

---

## 6. 工具链

| 工具 | 版本 | 用途 |
|---|---|---|
| Vitest | `^4.1.0` | 测试 runner |
| `@testing-library/*` | react `^16.3.2` · jest-dom `^6.9.1` · user-event `^14.6.1` | 组件测试套件 |
| `jsdom` | `^29.0.1` | 测试 DOM 环境 |
| ESLint | `^9.39.4` | flat config (`eslint.config.js`) |
| `typescript-eslint` | `^8.57.0` | TS lint |
| `eslint-plugin-react-{hooks,refresh}` | `^7.0.1` / `^0.5.2` | Hook 规则 + HMR 安全 |
| `cargo test` | — | Rust 单元测试 |

**npm scripts**：`dev` / `build` / `lint` / `preview` / `test` / `test:watch`。Tauri CLI 走 `npx tauri dev` 与 `npx tauri build`，未包装为 npm script。

---

## 7. 包管理 & Lockfile

| 层 | 管理器 | Lockfile | 提交状态 |
|---|---|---|---|
| 前端 | pnpm 9 | `pnpm-lock.yaml` | ✅ 提交 |
| Rust | Cargo | `src-tauri/Cargo.lock` | ✅ 提交（应用层项目） |

**版本锚定**：前端 `^` semver range，Rust 主版本字符串（如 `"2"` / `"0.5"`）；精确版本以 Lockfile 为准，CI 从 Lockfile 重建。

---

## 8. 构建产物 & CI

**本地命令**：`npx tauri dev`（启动 Vite + Rust 调试）/ `npx tauri build`（打包发布）。

**CI**：`.github/workflows/release.yml`，触发条件 `push tag v*`。

| Job | Runner | 目标 triple | 产物 |
|---|---|---|---|
| macos-aarch64 | `macos-latest` | `aarch64-apple-darwin` | `.dmg` |
| macos-x86_64 | `macos-latest` | `x86_64-apple-darwin` | `.dmg` |
| windows-x86_64 | `windows-latest` | `x86_64-pc-windows-msvc` | `.exe` / `.msi` |

发布动作：`tauri-apps/tauri-action@v0` → 自动建 GitHub Release（非草稿、非预发布）。

---

## 9. 升级流程

任何依赖变更须更新 `snapshot_date` 并随 commit 提交。下列变更**必须配套 ADR**：主版本 bump（Tauri / React / Tailwind 等）；**加密簇任一升级**（`argon2` / `aes-gcm` / `zeroize` / `rusqlite` —— 涉及密文兼容）；Rust MSRV 提升；新增/移除整类技术栈（如引入网络层、替换数据库引擎）。patch / minor 补丁仅更新 Lockfile + bump `snapshot_date`。
