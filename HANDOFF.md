# HANDOFF
<!-- /checkpoint at 2026-03-20 -->

## Active Plan
M2: Search & Quick Access — `docs/plans/m2-search-quickaccess.md`（7/8, 87%）
M3: Security & Polish — `docs/plans/m3-security-polish.md`（0/11, 0%）

## Session Tasks
- [x] /spec → docs/spec.md
- [x] /product-design → docs/product-spec.md + docs/design-spec.md
- [x] 1Password 竞品调研 → 更新 specs
- [x] M1 方案 + eng-review + design-review
- [x] M1 实现完成（16/16 tasks, 50 tests）
- [x] M2 方案
- [x] M2 Task 1 搜索引擎 + SearchBar + Cmd+K
- [x] M2 Task 2 Quick Access 浮窗（Cmd+Shift+Space, 多窗口）
- [x] M2 Task 3.1 ContextMenu 组件
- [ ] M2 Task 3.2 Recovery Kit（延后到 M3）
- [ ] M3 Task 1 Settings UI
- [ ] M3 Task 2 Auto-lock + Brute-force
- [ ] M3 Task 3 Touch ID
- [ ] M3 Task 4 Password Generator 面板
- [ ] M3 Task 5 Recovery Kit

## Key Files
- `CLAUDE.md` — 项目约定和架构规则
- `docs/plans/m3-security-polish.md` — 下一步实施方案
- `src-tauri/src/lib.rs` — Tauri 入口（命令注册 + Quick Access 窗口 + 全局快捷键）
- `src-tauri/src/state.rs` — AppState（M3 需扩展 failed_attempts + last_activity）
- `src/App.tsx` — 前端根组件（M3 需加 auto-lock 事件监听）

## Decisions Needed
- macOS sleep/lock 事件检测方案：`security-framework` vs `objc2` 直接桥接
