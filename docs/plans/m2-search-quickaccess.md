# M2: Search & Quick Access

Status: active
Progress: 7/8
Date: 2026-03-20
Journey: PS:§3.2 (Daily Password Retrieval)

## Goal

Deliver the daily password retrieval journey: Cmd+K in-app search + Cmd+Shift+Space global Quick Access floating panel. After M2, users can find and copy any password in < 3 seconds from any app.

## Non-Goals (M3)

- tantivy full-text search (FTS5 sufficient for < 5000 entries)
- Quick Access context-aware suggestions (detect active app)
- Clipboard external-copy cancellation
- Sidebar collapse/expand
- Touch ID / Auto-lock / Brute-force delay
- Settings UI (M2 uses default values)

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Search engine | SQLite FTS5 (built into rusqlite bundled) | Zero extra deps; < 5000 entries doesn't need tantivy; FTS5 supports prefix queries and ranking |
| Quick Access window | Tauri WebviewWindow (separate window, not overlay) | Independent window can appear from any app; Tauri natively supports multi-window |
| Quick Access entry point | Vite multi-page build (separate HTML + React entry) | Clean isolation from main window; own store state |
| PDF generation | `printpdf` crate | Pure Rust, no FFI, simple API for text + layout |
| Context menu | Custom React component (not native menu) | Consistent styling with DS:§4.4; native menu can't be themed |

## Task List

### Task 1: Search Engine + In-App Search (Journey §3.2 main window path)

- [ ] **1.1** Rust FTS5 search module: `db/search.rs` (FTS5 virtual table, search_entries query with rank, index sync on create/update/trash) + `db/schema.rs` migration + `db/queries.rs` index calls — Journey: §3.2
- [ ] **1.2** Rust search command: `commands/search.rs` (search_entries + recent_entries) + register in `lib.rs` — Journey: §3.2
- [ ] **1.3** Frontend search: `SearchBar.tsx` + `searchStore.ts` + `EntryList.tsx` integration (Cmd+K focus, 150ms debounce, highlight matches, Escape clear) + `App.tsx` global shortcut — Journey: §3.2

**Deliverable**: Cmd+K → type → entries filter in real-time → select → view detail. Search 1000 entries < 100ms.

### Task 2: Quick Access Global Floating Panel (Journey §3.2 full flow)

- [ ] **2.1** Tauri multi-window: `tauri.conf.json` quickaccess window config + `lib.rs` global shortcut (Cmd+Shift+Space toggle) + window focus-lost auto-hide — Journey: §3.2
- [ ] **2.2** Quick Access frontend: `quick-access.html` + `quick-access.tsx` + `QuickAccess.tsx` (DS:§6.5: large search input, recent entries on empty, compact EntryCards, keyboard nav ↑↓/Enter/Shift+Enter/Cmd+Enter/Cmd+O/Escape, lock-state inline unlock, animation) + Vite multi-entry config — Journey: §3.2

**Deliverable**: From any app, Cmd+Shift+Space → floating panel → search → Enter copies password → panel closes → Toast. Full PS:§3.2 journey walkable.

### Task 3: Context Menu + Recovery Kit (Polish)

- [ ] **3.1** Context menu: `ContextMenu.tsx` (DS:§4.4, right-click entry: Copy Username/Password/URL, Edit, Move to Trash) + `EntryCard.tsx` onContextMenu + global shortcuts Cmd+Shift+C / Cmd+Shift+U — Journey: §3.2 alternative path
- [ ] **3.2** Recovery Kit: `SetupWizard.tsx` restore 3-step wizard + Rust `generate_recovery_kit` command (printpdf crate, 128-bit recovery key, encrypt master_key, store in meta) + `@tauri-apps/plugin-dialog` file save — Journey: §3.1 step 3

**Deliverable**: Right-click entries for quick actions. First-time setup offers Recovery Kit PDF download.

## Dependencies

Rust additions to Cargo.toml:
```toml
printpdf = "0.7"
```

Frontend additions:
```
@tauri-apps/plugin-dialog
```

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| FTS5 not enabled in bundled-sqlcipher | Search broken | Verify at compile time in Task 1.1; FTS5 is default-enabled |
| Quick Access multi-window Vite config | Build complexity | Vite `build.rollupOptions.input` supports multi-entry natively |
| Quick Access focus-lost hides during typing | UX issue | Only hide on focus-lost after 100ms delay (debounce blur) |
| Global shortcut conflict | No response | Fallback: open from Dock icon; configurable in M3 Settings |
| printpdf increases bundle size | +500KB | Acceptable for PDF generation; pure Rust |

## Execution Order

```
Task 1 (Search)              Task 2 (Quick Access)        Task 3 (Polish)
──────────────────          ─────────────────────         ─────────────────
1.1 FTS5 module        →    2.1 Multi-window config  →    3.1 ContextMenu
1.2 Search command      →    2.2 QuickAccess UI       →    3.2 Recovery Kit
1.3 SearchBar UI
```

Strictly sequential. Task 2 reuses Task 1's search command. Task 3 is independent but last.

## File Manifest

### New Files

| File | Purpose |
|------|---------|
| `src-tauri/src/db/search.rs` | FTS5 virtual table + search queries + index sync |
| `src-tauri/src/commands/search.rs` | search_entries, recent_entries commands |
| `src/components/search/SearchBar.tsx` | In-app search bar (Cmd+K) |
| `src/components/search/QuickAccess.tsx` | Quick Access panel content |
| `src/components/entry/ContextMenu.tsx` | Right-click context menu (DS:§4.4) |
| `src/stores/searchStore.ts` | Search state (query, results, selectedIndex) |
| `src/quick-access.html` | Quick Access window HTML entry |
| `src/quick-access.tsx` | Quick Access window React entry |

### Modified Files

| File | Change |
|------|--------|
| `src-tauri/src/db/mod.rs` | Add `pub mod search` |
| `src-tauri/src/db/schema.rs` | Add FTS5 migration |
| `src-tauri/src/db/queries.rs` | Call index sync after create/update/trash |
| `src-tauri/src/commands/mod.rs` | Add `pub mod search` |
| `src-tauri/src/commands/entries.rs` | Add `copy_field_by_type` command |
| `src-tauri/src/lib.rs` | Register search commands + Quick Access window + global shortcut |
| `src-tauri/tauri.conf.json` | Add quickaccess window definition |
| `src-tauri/Cargo.toml` | Add `printpdf` |
| `src/components/layout/EntryList.tsx` | Replace header with SearchBar |
| `src/components/entry/EntryCard.tsx` | Add `compact` prop + `onContextMenu` |
| `src/lib/commands.ts` | Add search + recovery command types |
| `src/stores/vaultStore.ts` | Integrate search results path |
| `src/App.tsx` | Add Cmd+K global shortcut |
| `src/components/lock/SetupWizard.tsx` | Restore 3-step wizard with Recovery Kit |
| `vite.config.ts` | Multi-entry build config |
| `package.json` | Add `@tauri-apps/plugin-dialog` |
