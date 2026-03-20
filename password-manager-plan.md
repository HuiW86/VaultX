# 本地密码管理器开发方案

> 项目代号：**VaultX** — 一款模仿 1Password UI 的本地优先密码管理器

---

## 一、项目定位

### 核心理念

数据 100% 本地存储，零云端依赖。用户完全掌控自己的密码数据，不依赖任何第三方服务器。UI 风格对标 1Password 8 的设计语言——简洁、现代、macOS 原生感。

### 目标用户

- 重视隐私、不想把密码交给云服务的个人用户
- 开发者和技术人员（需要管理 SSH 密钥、API Token 等）
- 有多设备同步需求但希望自主选择同步方式的用户

### 与 1Password 的差异化

| 维度 | 1Password | VaultX |
|------|-----------|--------|
| 数据存储 | 云端（强制订阅） | 纯本地文件 |
| 同步方式 | 官方云 | 用户自选（iCloud Drive / Syncthing / Git） |
| 代码开放 | 闭源 | 开源（MIT License） |
| 费用 | $2.99/月起 | 免费 |
| 技术栈 | Rust + Electron | Rust + Tauri（更轻量） |

---

## 二、技术选型

### 架构概览

```
┌──────────────────────────────────────┐
│          UI Layer (前端)              │
│   React + TypeScript + Tailwind CSS  │
│          Tauri WebView 渲染           │
├──────────────────────────────────────┤
│        Bridge Layer (桥接层)          │
│     Tauri Commands (IPC 通信)        │
├──────────────────────────────────────┤
│        Core Layer (Rust 核心)         │
│  加密引擎 │ 数据库 │ 文件 I/O │ 搜索  │
├──────────────────────────────────────┤
│        Storage Layer (存储层)         │
│   SQLite (加密) / 本地文件系统        │
└──────────────────────────────────────┘
```

### 为什么选 Tauri 而不是 Electron？

1Password 用的 Electron 打包体积大（~150MB+），内存占用高。Tauri 使用系统原生 WebView，打包体积可控制在 10-15MB，内存占用也更低。同样支持 Rust 后端，与 1Password 的 Rust Core 思路一致，但更轻量。

### 技术栈明细

| 层级 | 技术 | 说明 |
|------|------|------|
| 前端框架 | React 18 + TypeScript | 1Password 同款，生态成熟 |
| 样式方案 | Tailwind CSS + Radix UI | 快速还原 1Password 的设计系统 |
| 桌面壳 | Tauri 2.0 | Rust 驱动，轻量级桌面容器 |
| 加密库 | ring / aes-gcm (Rust) | AES-256-GCM 加密，PBKDF2/Argon2 密钥派生 |
| 本地数据库 | SQLCipher（加密 SQLite） | 数据库级透明加密 |
| 搜索引擎 | tantivy (Rust) | 全文搜索，模糊匹配 |
| 密码生成 | rand + zxcvbn-rs | 安全随机 + 强度评估 |
| TOTP | totp-rs | 双因素认证码生成 |
| 自动填充 | 浏览器扩展（WXT 框架） | 支持 Chrome / Firefox / Safari |
| 包管理 | pnpm | 前端依赖管理 |
| 构建工具 | Vite | 快速开发 + HMR |

---

## 三、功能规划

### P0 — MVP 核心功能（第 1-2 个月）

这些是上线的最低要求：

**1. 主密码与解锁**
- 首次使用设置主密码
- Argon2id 密钥派生（比 1Password 的 PBKDF2 更现代）
- Touch ID / 系统生物认证解锁（通过 Tauri 调用系统 API）
- 自动锁定（闲置超时、盖上笔记本等）

**2. 保险库 (Vault)**
- 支持多个保险库（个人、工作等）
- 条目类型：登录、银行卡、安全笔记、身份信息、SSH 密钥
- CRUD 操作 + 收藏 + 标签
- 基于 SQLCipher 的加密存储

**3. 密码生成器**
- 可配置长度、字符类型
- 支持「随机字符」和「单词组合」两种模式
- 实时强度评估（zxcvbn 算法）
- 生成历史记录

**4. 搜索**
- 全局快速搜索（模仿 1Password 的 Quick Access）
- 支持模糊匹配、按类别/标签过滤
- 全局快捷键唤出（如 Cmd+Shift+Space）

**5. 复制与自动清除**
- 一键复制用户名/密码
- 剪贴板自动清除（30 秒后）

### P1 — 体验完善（第 3-4 个月）

**6. Watchtower 安全审计**
- 弱密码检测
- 重复密码提醒
- 密码老化提醒（超过 N 天未更换）
- 集成 Have I Been Pwned API 检测泄露

**7. 浏览器扩展**
- 基于 WXT 框架开发（一套代码多浏览器）
- 自动检测登录表单并填充
- 自动保存新密码
- 与桌面 App 通过 Native Messaging 通信

**8. TOTP 双因素认证**
- 扫码或手动输入密钥
- 实时生成 6 位验证码
- 倒计时提示

**9. 导入 / 导出**
- 支持从 1Password、Bitwarden、Chrome、LastPass 导入
- CSV / JSON 导出（加密或明文可选）

### P2 — 高级功能（第 5-6 个月）

**10. 同步方案**
- 通过文件系统同步（iCloud Drive / Dropbox / Syncthing）
- 冲突检测与合并策略
- 可选：自建 WebDAV 同步

**11. SSH Agent 集成**
- SSH 密钥管理
- 作为 SSH Agent 自动提供密钥
- Git 签名集成

**12. 旅行模式**
- 标记「安全旅行」条目
- 一键隐藏非安全条目
- 模仿 1Password 的过海关保护

**13. 密码共享**
- 生成过期链接分享单条密码
- 基于非对称加密的安全传输

---

## 四、UI 设计方案

### 设计语言

模仿 1Password 8 的设计语言，核心特征：

**布局结构：三栏式**
```
┌─────────┬──────────────┬──────────────────┐
│ 侧边栏   │  条目列表      │  详情面板          │
│         │              │                  │
│ 保险库    │  搜索栏        │  标题 + 图标       │
│ 类别筛选  │  条目卡片 ×N   │  字段列表          │
│ 标签     │              │  操作按钮          │
│         │              │                  │
│ 设置     │              │                  │
│ 锁定     │              │                  │
└─────────┴──────────────┴──────────────────┘
  220px       300px          flex-1
```

**配色方案（暗色主题优先）**

| 用途 | 色值 | 说明 |
|------|------|------|
| 背景主色 | #1A1A2E | 深蓝黑，1Password 同款暗色调 |
| 侧边栏 | #16163A | 略深于主背景 |
| 卡片/面板 | #222244 | 微妙的层次感 |
| 强调色 | #0066FF | 1Password 标志性蓝 |
| 次强调色 | #4CC9F0 | 辅助高亮 |
| 文字主色 | #E8E8F0 | 柔和白 |
| 文字次色 | #8888AA | 灰紫 |
| 成功 | #10B981 | 强密码指示 |
| 警告 | #F59E0B | 一般密码 |
| 危险 | #EF4444 | 弱密码/泄露 |

**字体**

- 标题：SF Pro Display / Geist（macOS 原生感）
- 正文：SF Pro Text / Geist
- 代码/密码：JetBrains Mono / SF Mono

**关键交互**

- Quick Access：全局快捷键 `Cmd+Shift+Space` 唤出浮动搜索面板
- 侧边栏可折叠
- 密码字段默认隐藏，hover 显示「眼睛」图标
- 复制后显示 toast 提示并标记复制时间
- 流畅的过渡动画（framer-motion）

### 核心页面清单

| 页面 | 功能 |
|------|------|
| 锁屏 | 主密码输入 / Touch ID / 首次设置引导 |
| 主界面 | 三栏布局，保险库浏览与管理 |
| Quick Access | 全局浮窗搜索 |
| 密码生成器 | 独立面板或嵌入编辑表单 |
| Watchtower | 安全仪表盘（饼图 + 问题列表） |
| 设置 | 外观/安全/同步/快捷键/导入导出 |
| 条目编辑 | 表单式编辑，类别切换 |

---

## 五、安全架构

### 加密方案

```
主密码
  │
  ▼
Argon2id (salt, m=64MB, t=3, p=4)
  │
  ▼
Master Key (256-bit)
  │
  ├──► SQLCipher 数据库加密密钥
  │
  └──► 各条目字段单独加密
       AES-256-GCM (随机 IV/Nonce)
```

**双层加密**：SQLCipher 提供数据库级透明加密（防止文件被直接读取），同时每个敏感字段（密码、卡号等）再用 AES-256-GCM 单独加密（防止数据库密钥泄露后一次性暴露所有数据）。

### 安全措施清单

- 内存中密码使用后立即 zeroize（Rust `zeroize` crate）
- 剪贴板自动清除
- 防暴力破解：连续失败后增加延迟
- 敏感数据不写入日志
- 自动锁定策略
- 可选：Secret Key 机制（类似 1Password 的 Account Key）

---

## 六、项目结构

```
vaultx/
├── src-tauri/                 # Rust 后端
│   ├── src/
│   │   ├── main.rs            # Tauri 入口
│   │   ├── crypto/            # 加密模块
│   │   │   ├── mod.rs
│   │   │   ├── argon2.rs      # 密钥派生
│   │   │   ├── aes_gcm.rs     # 对称加密
│   │   │   └── password_gen.rs # 密码生成
│   │   ├── db/                # 数据库模块
│   │   │   ├── mod.rs
│   │   │   ├── schema.rs      # 表结构
│   │   │   └── queries.rs     # CRUD 操作
│   │   ├── vault/             # 保险库逻辑
│   │   │   ├── mod.rs
│   │   │   ├── entry.rs       # 条目模型
│   │   │   └── watchtower.rs  # 安全审计
│   │   ├── search/            # 搜索引擎
│   │   ├── commands.rs        # Tauri IPC 命令
│   │   └── state.rs           # 应用状态管理
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── src/                       # React 前端
│   ├── components/
│   │   ├── layout/            # 布局组件
│   │   │   ├── Sidebar.tsx
│   │   │   ├── EntryList.tsx
│   │   │   └── DetailPanel.tsx
│   │   ├── vault/             # 保险库组件
│   │   │   ├── EntryCard.tsx
│   │   │   ├── EntryForm.tsx
│   │   │   └── EntryDetail.tsx
│   │   ├── security/          # 安全组件
│   │   │   ├── LockScreen.tsx
│   │   │   ├── PasswordGenerator.tsx
│   │   │   └── Watchtower.tsx
│   │   ├── search/
│   │   │   └── QuickAccess.tsx
│   │   └── ui/                # 基础 UI 组件
│   │       ├── Button.tsx
│   │       ├── Input.tsx
│   │       └── Toast.tsx
│   ├── hooks/                 # 自定义 Hooks
│   │   ├── useVault.ts
│   │   ├── useCrypto.ts
│   │   └── useClipboard.ts
│   ├── stores/                # Zustand 状态管理
│   │   ├── vaultStore.ts
│   │   └── settingsStore.ts
│   ├── styles/                # 全局样式
│   │   └── globals.css
│   ├── App.tsx
│   └── main.tsx
│
├── extensions/                # 浏览器扩展
│   ├── src/
│   │   ├── background.ts
│   │   ├── content.ts
│   │   └── popup/
│   └── wxt.config.ts
│
├── package.json
├── tsconfig.json
├── tailwind.config.ts
├── vite.config.ts
└── README.md
```

---

## 七、数据模型

### 数据库 Schema（SQLCipher）

```sql
-- 保险库
CREATE TABLE vaults (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    icon        TEXT,
    sort_order  INTEGER DEFAULT 0,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

-- 条目
CREATE TABLE entries (
    id          TEXT PRIMARY KEY,
    vault_id    TEXT NOT NULL REFERENCES vaults(id),
    category    TEXT NOT NULL, -- login | card | note | identity | ssh_key
    title       TEXT NOT NULL, -- 加密
    subtitle    TEXT,          -- 加密（用户名/卡号后四位等）
    icon_url    TEXT,
    favorite    INTEGER DEFAULT 0,
    trashed     INTEGER DEFAULT 0,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

-- 字段（每个条目可有多个字段）
CREATE TABLE fields (
    id          TEXT PRIMARY KEY,
    entry_id    TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    field_type  TEXT NOT NULL, -- username | password | url | otp | text | hidden | card_number | ...
    label       TEXT NOT NULL,
    value       BLOB NOT NULL, -- AES-256-GCM 加密
    sort_order  INTEGER DEFAULT 0
);

-- 标签
CREATE TABLE tags (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE
);

-- 条目-标签关联
CREATE TABLE entry_tags (
    entry_id    TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    tag_id      TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (entry_id, tag_id)
);

-- 密码历史
CREATE TABLE password_history (
    id          TEXT PRIMARY KEY,
    entry_id    TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    value       BLOB NOT NULL, -- 加密
    created_at  INTEGER NOT NULL
);
```

---

## 八、开发路线图

### Phase 1：基础框架（第 1-2 周）

- 初始化 Tauri + React + TypeScript 项目
- 搭建 Rust 加密模块（Argon2id + AES-256-GCM）
- 实现 SQLCipher 数据库初始化
- 完成锁屏 UI 与主密码验证流程

### Phase 2：核心 CRUD（第 3-4 周）

- 实现保险库创建与管理
- 条目的增删改查
- 前端三栏布局
- Tauri IPC 通信打通

### Phase 3：体验打磨（第 5-6 周）

- 密码生成器（含强度评估）
- 全局搜索 + Quick Access
- 复制 + 剪贴板清除
- Touch ID 集成
- 自动锁定

### Phase 4：安全审计（第 7-8 周）

- Watchtower 仪表盘
- HIBP 泄露检测（k-Anonymity API）
- 弱密码/重复密码检测
- TOTP 支持

### Phase 5：生态扩展（第 9-12 周）

- 浏览器扩展（Chrome + Firefox）
- 导入/导出功能
- 文件同步方案
- SSH Agent 集成

---

## 九、开发环境搭建

### 前置依赖

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Node.js (推荐 v20+)
brew install node

# pnpm
npm install -g pnpm

# Tauri CLI
cargo install tauri-cli

# SQLCipher
brew install sqlcipher
```

### 项目初始化

```bash
# 创建 Tauri 项目
pnpm create tauri-app vaultx --template react-ts

# 安装前端依赖
cd vaultx
pnpm add @radix-ui/react-dialog @radix-ui/react-dropdown-menu
pnpm add zustand framer-motion
pnpm add -D tailwindcss postcss autoprefixer
npx tailwindcss init -p

# 安装 Rust 依赖 (在 src-tauri/Cargo.toml 中添加)
# aes-gcm, argon2, sqlx, tantivy, totp-rs, zeroize, rand
```

### Rust 核心依赖（Cargo.toml）

```toml
[dependencies]
tauri = { version = "2", features = ["macos-private-api"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
aes-gcm = "0.10"
argon2 = "0.5"
rand = "0.8"
zeroize = { version = "1", features = ["derive"] }
sqlx = { version = "0.7", features = ["runtime-tokio", "sqlite"] }
tantivy = "0.22"
totp-rs = "5"
uuid = { version = "1", features = ["v4"] }
tokio = { version = "1", features = ["full"] }
```

---

## 十、参考资源

### 设计参考
- 1Password 8 Mac 界面截图与设计语言
- Radix UI 组件库文档
- Tailwind CSS 暗色主题方案

### 技术参考
- Tauri 2.0 官方文档：https://v2.tauri.app
- 1Password 安全白皮书（加密方案参考）
- RustCrypto 项目（aes-gcm, argon2 实现）
- SQLCipher 文档

### 竞品代码参考（均开源）
- Bitwarden（C# + Angular，参考功能逻辑）
- KeePassXC（C++ + Qt，参考本地存储方案）
- Padloc（TypeScript，参考 UI 交互设计）

---

## 十一、风险与应对

| 风险 | 影响 | 应对策略 |
|------|------|----------|
| Tauri WebView 兼容性 | 不同系统渲染差异 | 优先适配 macOS，后续逐步覆盖 |
| 加密实现漏洞 | 安全性受损 | 使用成熟的 RustCrypto 库，不自己造轮子 |
| 浏览器扩展审核 | 上架延迟 | 先支持手动安装，并行提交审核 |
| SQLCipher 性能 | 大量条目时变慢 | 索引优化 + 分页加载 |
| 文件同步冲突 | 数据丢失 | 基于时间戳的 last-write-wins + 冲突备份 |
