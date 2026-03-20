# Spec: VaultX - Local-First Password Manager

## Clarity Score: 93/100
| Dimension | Score | Notes |
|-----------|:-----:|-------|
| Business Context | 19/20 | Clear problem, defined users, measurable goals |
| Functional Clarity | 28/30 | Core features well-defined, onboarding flow clear |
| Technical Specificity | 23/25 | Tech stack locked, crypto scheme detailed |
| Scope Definition | 23/25 | Phased scope with clear out-of-scope items |

## 1. Background & Root Problem

### Surface Request
Build a local password manager that looks like 1Password 8.

### Root Problem
Mainstream password managers (1Password, LastPass, Bitwarden) force users into cloud-hosted subscription models. Privacy-conscious users and developers have no polished, modern alternative that keeps data 100% local. Existing open-source options (KeePassXC) have dated UIs and poor browser integration.

### Why Now
- 1Password forced migration to subscription model, removing standalone licenses
- Growing privacy awareness and data sovereignty concerns
- Tauri 2.0 maturity makes lightweight, native-feeling desktop apps viable with web tech
- RustCrypto ecosystem is production-ready

## 2. Objectives
- **Primary**: Build a fully functional local password manager that can replace 1Password for personal daily use
- **Secondary**: Open-source it (MIT) as a credible alternative for privacy-focused users
- **Anti-Goals**:
  - No cloud sync service (all scope) -- users choose their own sync method
  - No team/enterprise features: SSO, audit logs, custom roles, shared vault admin (all scope)
  - No mobile apps (this phase) -- macOS desktop only
  - No AI-powered autofill (this phase) -- traditional form matching first
  - No Passkey support (this phase) -- evaluate post-P2

## 3. Target Users

| Attribute | Description |
|-----------|-------------|
| Who | Privacy-conscious individuals, developers, technical users |
| Context | Daily credential management on macOS: logging into websites, storing API tokens, SSH keys, secure notes |
| Current workaround | 1Password (subscription, cloud-forced), KeePassXC (dated UI), browser built-in (no cross-browser, weak security) |
| Pain level | High -- forced cloud storage is a dealbreaker; KeePassXC UX friction causes abandonment |

## 4. Strategy Chosen

**Strategy: 1Password-parity UX with local-only architecture**

Build a Tauri 2.0 desktop app with 1Password 8's three-panel layout and design language, but backed by a local SQLCipher database. No cloud dependency. Ship macOS first, cross-platform later.

Why this over alternatives:
- "Minimal CLI tool" -- too niche, doesn't solve the UX gap vs 1Password
- "Electron clone" -- defeats the lightweight advantage; Tauri is 10x smaller
- "Mobile-first PWA" -- mobile platforms restrict crypto/clipboard APIs; desktop is the power-user base

## 5. User Stories & Acceptance Criteria

### Story 1: First-Time Setup
**As a** new user, **I want to** set up a master password and create my vault on first launch, **so that** I can start storing credentials immediately.
**Priority**: P0 | **Complexity**: L

**Acceptance Criteria:**

Happy Path:
- [ ] Given the app is launched for the first time, when the user opens VaultX, then a setup wizard is displayed with password creation form
- [ ] Given the setup wizard, when the user enters a master password meeting minimum requirements (12+ chars), then a strength meter shows real-time evaluation (zxcvbn score)
- [ ] Given a valid master password is confirmed, when the user clicks "Create Vault", then Argon2id key derivation completes in < 2s, SQLCipher database is initialized, and a default "Personal" vault is created
- [ ] Given setup is complete, when the main interface loads, then an empty-state guide prompts the user to add their first entry or import data

Validation:
- [ ] Given a weak password (zxcvbn score < 2), when the user attempts to proceed, then a warning is shown (but not blocked -- user's choice)
- [ ] Given mismatched password confirmation, when the user clicks "Create Vault", then an inline error is shown without clearing the first field

Edge Cases:
- [ ] Given the app is force-quit during setup, when re-launched, then setup resumes from the beginning (no corrupt partial DB)
- [ ] Given a database file already exists at the default path, when setup runs, then the user is prompted to unlock existing vault or create a new one

---

### Story 2: Unlock Vault
**As a** returning user, **I want to** unlock my vault quickly with master password or Touch ID, **so that** I can access credentials without friction.
**Priority**: P0 | **Complexity**: M

**Acceptance Criteria:**

Happy Path:
- [ ] Given the app is locked, when the user enters the correct master password and presses Enter, then the vault unlocks in < 1s and the main interface is displayed
- [ ] Given Touch ID is enabled, when the user authenticates via Touch ID, then the vault unlocks in < 2s
- [ ] Given the vault is unlocked, when the user is idle for the configured timeout (default: 8 hours), then the vault auto-locks

Validation:
- [ ] Given an incorrect master password, when the user submits, then an error message is shown with remaining attempts before delay
- [ ] Given 5 consecutive failed attempts, when the user tries again, then a progressive delay is enforced (5s, 15s, 30s, 60s)

Edge Cases:
- [ ] Given Touch ID hardware is unavailable (e.g., external keyboard, no sensor), when the lock screen loads, then Touch ID option is hidden and password input is focused
- [ ] Given the laptop lid is closed and reopened, when the auto-lock policy triggers, then the vault locks and the lock screen is displayed immediately

---

### Story 3: CRUD Vault Entries
**As a** user, **I want to** create, view, edit, and delete password entries in my vault, **so that** I can manage all my credentials in one place.
**Priority**: P0 | **Complexity**: L

**Acceptance Criteria:**

Happy Path:
- [ ] Given the user is on the main interface, when they click "+ New Item", then a category selector appears (Login, Card, Note, Identity, SSH Key)
- [ ] Given "Login" is selected, when the form loads, then fields include: title, username, password (with generate button), URL, notes, tags
- [ ] Given all required fields are filled, when the user clicks "Save", then the entry is encrypted (AES-256-GCM) and persisted to SQLCipher in < 200ms
- [ ] Given an existing entry, when the user clicks "Edit", then the form is pre-filled with decrypted values; saving re-encrypts and updates `updated_at`
- [ ] Given an entry, when the user clicks "Delete", then a confirmation dialog appears; confirming moves it to Trash (soft delete, `trashed=1`)

Validation:
- [ ] Given a new entry with no title, when the user clicks "Save", then an inline error highlights the title field
- [ ] Given duplicate URL detection, when saving a Login with a URL matching an existing entry, then a non-blocking notice is shown

Edge Cases:
- [ ] Given an entry with 50+ custom fields, when viewing the detail panel, then all fields render without layout overflow (scrollable)
- [ ] Given the user deletes the last entry in a vault, when the list is empty, then an empty-state illustration and "Add your first item" CTA is shown

---

### Story 4: Password Generator
**As a** user, **I want to** generate strong, customizable passwords, **so that** I can create unique credentials for every account.
**Priority**: P0 | **Complexity**: M

**Acceptance Criteria:**

Happy Path:
- [ ] Given the password generator is opened, when default settings load, then a 20-character random password is generated with uppercase, lowercase, digits, and symbols
- [ ] Given the user adjusts length (8-128) or toggles character types, when any setting changes, then a new password is generated in < 50ms with real-time strength score
- [ ] Given "Word-based" mode is selected, when generating, then 4-6 random words are joined with a configurable separator (-, ., space) and optional digit/symbol padding
- [ ] Given a generated password, when the user clicks "Use Password", then it fills the password field in the current entry form

Validation:
- [ ] Given all character types are toggled off, when the user attempts to generate, then at least one type is forced on with a tooltip explaining why

Edge Cases:
- [ ] Given the generator is opened standalone (not from entry form), when "Copy" is clicked, then the password is copied and a 30s clipboard auto-clear timer starts
- [ ] Given generation history, when the user views history, then last 20 generated passwords are listed (encrypted in DB) with timestamps

---

### Story 5: Search & Quick Access
**As a** user, **I want to** quickly find any credential via global search or Quick Access hotkey, **so that** I don't waste time browsing through vaults manually.
**Priority**: P0 | **Complexity**: M

**Acceptance Criteria:**

Happy Path:
- [ ] Given the main interface, when the user types in the search bar or presses `Cmd+K`, then results filter in real-time as the user types, with < 100ms latency for up to 5,000 entries
- [ ] Given the global hotkey `Cmd+Shift+Space` is pressed from any app, when VaultX is running (locked or unlocked), then a floating Quick Access panel appears
- [ ] Given search results, when the user clicks an entry, then the detail panel shows and the password is one-click copyable

Validation:
- [ ] Given the vault is locked when Quick Access is triggered, when the panel appears, then the master password / Touch ID prompt is shown inline before results

Edge Cases:
- [ ] Given a query with no matches, when the search completes, then an empty-state message suggests checking spelling or broadening the search
- [ ] Given special characters in search query (e.g., `@`, `.`), when searching, then they are treated as literal characters, not query operators

---

### Story 6: Copy & Clipboard Auto-Clear
**As a** user, **I want to** copy credentials to clipboard with automatic clearing, **so that** sensitive data doesn't linger in clipboard history.
**Priority**: P0 | **Complexity**: S

**Acceptance Criteria:**

Happy Path:
- [ ] Given an entry's detail view, when the user clicks the copy icon next to any field (username, password, URL), then the value is copied to system clipboard and a toast confirms "Copied, clearing in 30s"
- [ ] Given a value was copied, when 30 seconds elapse, then the clipboard is cleared (replaced with empty string)
- [ ] Given a value was copied, when the user copies something else (inside or outside VaultX), then the auto-clear timer is cancelled

Validation:
- [ ] Given clipboard access is denied by OS, when copy is attempted, then a fallback toast says "Unable to access clipboard"

Edge Cases:
- [ ] Given the app is quit before the 30s timer, when VaultX exits, then the clipboard is immediately cleared on shutdown

---

### Story 7: Multiple Vaults
**As a** user, **I want to** organize credentials into separate vaults (Personal, Work, etc.), **so that** I can logically separate different contexts.
**Priority**: P0 | **Complexity**: M

**Acceptance Criteria:**

Happy Path:
- [ ] Given the sidebar, when the user clicks "+ New Vault", then a dialog prompts for vault name and optional icon
- [ ] Given multiple vaults exist, when the user selects a vault in the sidebar, then the entry list filters to show only that vault's entries
- [ ] Given the "All Vaults" view, when selected, then entries from all vaults are merged and displayed with vault badges

Validation:
- [ ] Given a vault name that already exists, when creating, then an inline error prevents duplicate names

Edge Cases:
- [ ] Given a vault with 0 entries, when selected, then an empty-state is shown with "Add Item" CTA
- [ ] Given the user deletes a vault, when confirmed, then all entries in it are moved to Trash (not permanently deleted)

## 6. Feature Breakdown & Dependencies

| ID | Feature | Priority | Complexity | Dependencies | Stories |
|----|---------|----------|------------|--------------|---------|
| F1 | Crypto Engine (Argon2id + AES-256-GCM) | P0 | L | None | S1, S2, S3 |
| F2 | SQLCipher Database Layer | P0 | M | F1 | S1, S3 |
| F3 | Master Password Setup & Unlock | P0 | L | F1, F2 | S1, S2 |
| F4 | Vault & Entry CRUD | P0 | L | F2, F3 | S3, S7 |
| F5 | Password Generator | P0 | M | F1 | S4 |
| F6 | Search Engine (tantivy) | P0 | M | F2, F4 | S5 |
| F7 | Quick Access (global hotkey) | P0 | M | F3, F6 | S5 |
| F8 | Clipboard Manager | P0 | S | None | S6 |
| F9 | Touch ID Integration | P0 | M | F3 | S2 |
| F10 | Three-Panel UI Layout | P0 | L | F4 | S3, S5, S7 |
| F11 | Watchtower Security Audit | P1 | L | F4 | -- |
| F12 | Browser Extension (WXT) | P1 | XL | F3, F4 | -- |
| F13 | TOTP Generator | P1 | M | F4 | -- |
| F14 | Import / Export | P1 | L | F4 | -- |
| F15 | File-based Sync | P2 | L | F2 | -- |
| F16 | SSH Agent Integration | P2 | L | F4 | -- |

### Dependency Graph

```
F1 (Crypto) ──→ F2 (SQLCipher) ──→ F4 (CRUD) ──→ F6 (Search) ──→ F7 (Quick Access)
    │                │                  │               │
    │                └──→ F3 (Unlock) ──┘               └──→ F10 (UI Layout)
    │                      │
    │                      └──→ F9 (Touch ID)
    │
    └──→ F5 (Password Gen)

F8 (Clipboard) ── standalone

F4 ──→ F11 (Watchtower)
F4 ──→ F12 (Browser Ext)
F4 ──→ F13 (TOTP)
F4 ──→ F14 (Import/Export)
F2 ──→ F15 (Sync)
F4 ──→ F16 (SSH Agent)
```

## 7. Data Model

### Entity: Vault
| Field | Type | Constraints | Required | Default |
|-------|------|-------------|:--------:|---------|
| id | TEXT (UUID) | PK | Yes | gen_uuid_v4() |
| name | TEXT | UNIQUE, NOT NULL | Yes | -- |
| icon | TEXT | -- | No | NULL |
| sort_order | INTEGER | -- | No | 0 |
| created_at | INTEGER | UNIX timestamp | Yes | -- |
| updated_at | INTEGER | UNIX timestamp | Yes | -- |

### Entity: Entry
| Field | Type | Constraints | Required | Default |
|-------|------|-------------|:--------:|---------|
| id | TEXT (UUID) | PK | Yes | gen_uuid_v4() |
| vault_id | TEXT | FK → vaults.id | Yes | -- |
| category | TEXT | CHECK(login,card,note,identity,ssh_key) | Yes | -- |
| title | TEXT | NOT NULL, encrypted | Yes | -- |
| subtitle | TEXT | encrypted | No | NULL |
| icon_url | TEXT | -- | No | NULL |
| favorite | INTEGER | 0 or 1 | No | 0 |
| trashed | INTEGER | 0 or 1 | No | 0 |
| created_at | INTEGER | UNIX timestamp | Yes | -- |
| updated_at | INTEGER | UNIX timestamp | Yes | -- |

### Entity: Field
| Field | Type | Constraints | Required | Default |
|-------|------|-------------|:--------:|---------|
| id | TEXT (UUID) | PK | Yes | gen_uuid_v4() |
| entry_id | TEXT | FK → entries.id, ON DELETE CASCADE | Yes | -- |
| field_type | TEXT | CHECK(username,password,url,otp,text,hidden,card_number,...) | Yes | -- |
| label | TEXT | NOT NULL | Yes | -- |
| value | BLOB | AES-256-GCM encrypted | Yes | -- |
| sort_order | INTEGER | -- | No | 0 |

### Entity: Tag
| Field | Type | Constraints | Required | Default |
|-------|------|-------------|:--------:|---------|
| id | TEXT (UUID) | PK | Yes | gen_uuid_v4() |
| name | TEXT | UNIQUE, NOT NULL | Yes | -- |

### Entity: EntryTag (junction)
| Field | Type | Constraints | Required | Default |
|-------|------|-------------|:--------:|---------|
| entry_id | TEXT | FK → entries.id, ON DELETE CASCADE | Yes | -- |
| tag_id | TEXT | FK → tags.id, ON DELETE CASCADE | Yes | -- |
| -- | -- | PK(entry_id, tag_id) | -- | -- |

### Entity: PasswordHistory
| Field | Type | Constraints | Required | Default |
|-------|------|-------------|:--------:|---------|
| id | TEXT (UUID) | PK | Yes | gen_uuid_v4() |
| entry_id | TEXT | FK → entries.id, ON DELETE CASCADE | Yes | -- |
| value | BLOB | AES-256-GCM encrypted | Yes | -- |
| created_at | INTEGER | UNIX timestamp | Yes | -- |

## 8. API Surface

VaultX uses Tauri IPC Commands (not HTTP APIs). All commands are invoked from the React frontend via `invoke()`.

| Command | Description | Auth Required | Request | Response |
|---------|-------------|:------------:|---------|----------|
| `setup_vault` | First-time master password setup | No | `{ password: string }` | `{ success: bool }` |
| `unlock` | Unlock vault with master password | No | `{ password: string }` | `{ success: bool, error?: string }` |
| `unlock_biometric` | Unlock via Touch ID | No | `{}` | `{ success: bool }` |
| `lock` | Lock the vault | Yes | `{}` | `{}` |
| `list_vaults` | List all vaults | Yes | `{}` | `Vault[]` |
| `create_vault` | Create a new vault | Yes | `{ name, icon? }` | `Vault` |
| `delete_vault` | Soft-delete a vault | Yes | `{ vault_id }` | `{}` |
| `list_entries` | List entries (with filters) | Yes | `{ vault_id?, category?, tag?, trashed? }` | `EntrySummary[]` |
| `get_entry` | Get full entry with decrypted fields | Yes | `{ entry_id }` | `EntryDetail` |
| `create_entry` | Create new entry | Yes | `{ vault_id, category, title, fields[] }` | `Entry` |
| `update_entry` | Update entry | Yes | `{ entry_id, ...changes }` | `Entry` |
| `delete_entry` | Move to trash / permanent delete | Yes | `{ entry_id, permanent? }` | `{}` |
| `toggle_favorite` | Toggle favorite status | Yes | `{ entry_id }` | `{}` |
| `search` | Full-text search across entries | Yes | `{ query, vault_id?, category? }` | `EntrySummary[]` |
| `generate_password` | Generate a random password | No | `{ length, uppercase, lowercase, digits, symbols, mode }` | `{ password, strength }` |
| `copy_to_clipboard` | Copy value with auto-clear | Yes | `{ value, clear_after_ms? }` | `{}` |
| `get_password_history` | Get password change history | Yes | `{ entry_id }` | `PasswordHistory[]` |

## 9. UI/UX Notes

### Layout
- Three-panel: Sidebar (220px) | Entry List (300px) | Detail Panel (flex-1)
- Sidebar collapsible to icon-only mode (48px)
- Dark theme by default (1Password 8 color palette)

### States
- **Loading**: Skeleton placeholders for entry list and detail panel
- **Empty**: Illustrated empty-states with CTAs ("Add your first login", "Create a vault")
- **Error**: Inline error messages for form validation; toast for system errors (DB, crypto failures)
- **Locked**: Full-screen lock overlay with password input + Touch ID button

### Responsiveness
- Minimum window size: 900 x 600px
- Below 1100px: detail panel becomes a slide-over sheet
- Below 900px: sidebar collapses to icon-only

### Accessibility
- Full keyboard navigation (Tab, Arrow keys, Enter, Escape)
- ARIA labels on all interactive elements
- High-contrast text (WCAG AA on dark background)
- Screen reader support for password strength announcements

### Animations (framer-motion)
- Panel transitions: 200ms ease-out
- Toast enter/exit: 150ms
- Password reveal: 100ms fade
- Quick Access open/close: 150ms scale + fade

### Design References
- Follow product-spec and design-spec templates if created via `/product-design`
- Color tokens, typography, and spacing defined in plan doc Section IV

## 10. Risks & Concerns

### Engineering
- **SQLCipher + Tauri bundling complexity**: SQLCipher requires native compilation; Tauri's build pipeline may need custom configuration. Mitigation: prototype the build chain in Phase 1 before committing to features.
- **Argon2id tuning vs. unlock speed**: Memory-hard KDF with m=64MB target may be slow on older machines. Mitigation: benchmark on minimum supported hardware, allow configurable parameters.
- **Global hotkey conflicts**: `Cmd+Shift+Space` may conflict with Spotlight or other tools. Mitigation: make hotkey configurable.
- **tantivy index size**: Full-text index on encrypted data requires decrypted in-memory index. Mitigation: index titles/URLs only (not password values), rebuild on unlock.

### User Experience
- **No cloud = no account recovery**: If the user forgets their master password, data is permanently lost. Mitigation: clear warning during setup, optional recovery key export (PDF).
- **Onboarding friction**: First-time setup is more complex than cloud services (no email signup). Mitigation: streamlined wizard, smart defaults, import from 1Password/Chrome on first run.
- **1Password muscle memory**: Users expect exact 1Password shortcuts and interactions. Mitigation: match key shortcuts and UI patterns closely.

### Business
- **Adoption vs. 1Password network effects**: 1Password has browser extension store presence, team features, enterprise sales. Mitigation: differentiate on privacy story; target niche audience first.
- **Maintenance burden**: Open-source project requires ongoing security patches for crypto dependencies. Mitigation: automated dependency monitoring (Dependabot), minimal dependency surface.

## 11. Out of Scope

- **Mobile apps (iOS/Android)** -- macOS desktop only for this phase
- **Team/enterprise features** -- SSO, audit logs, admin console, custom roles, shared vault management
- **Cloud sync service** -- no server-side component; sync via user's own file system tools
- **AI-powered autofill** -- use traditional form field matching in browser extension
- **Passkey credential management** -- evaluate after P2 based on ecosystem maturity
- **Windows/Linux** -- Tauri supports cross-platform, but UX polish targets macOS first; other platforms are future work
- **Password sharing via links** -- P2 feature, not in MVP

## 12. Open Questions

- [ ] Should Secret Key (128-bit, like 1Password's Account Key) be mandatory or optional? -- Owner: product decision
- [ ] SQLCipher vs. libsql with encryption: which has better Tauri integration? -- Owner: engineering spike
- [ ] Touch ID credential storage: use macOS Keychain or Tauri's built-in secure storage? -- Owner: engineering spike
- [ ] License: MIT or GPLv3? MIT is more permissive but allows proprietary forks -- Owner: product decision
- [ ] Auto-update mechanism: Tauri's built-in updater or manual download? -- Owner: engineering

## 13. Success Metrics

| Metric | Current | Target | How to Measure |
|--------|---------|--------|----------------|
| Unlock time (password) | N/A | < 1s | Benchmark: time from Enter to main UI render |
| Unlock time (Touch ID) | N/A | < 2s | Benchmark: time from fingerprint to main UI render |
| Search latency (1k entries) | N/A | < 100ms | Benchmark: keystroke to results render |
| Search latency (5k entries) | N/A | < 200ms | Benchmark: keystroke to results render |
| Entry save time | N/A | < 200ms | Benchmark: click Save to confirmation toast |
| App bundle size | N/A | < 15MB | Measure .dmg output |
| Memory usage (idle) | N/A | < 80MB | Activity Monitor after unlock with 1k entries |
| Daily personal use | N/A | Fully replaces current password manager | Self-dogfooding log |
| Password gen strength | N/A | zxcvbn score 4 on default settings | Automated test |
