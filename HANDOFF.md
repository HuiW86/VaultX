# HANDOFF
<!-- /checkpoint at 2026-03-20 -->

## Active Plan
M2: Search & Quick Access — `docs/plans/m2-search-quickaccess.md`（7/8, 87%）
M3: Security & Polish — `docs/plans/m3-security-polish.md`（11/11, 100% ✅）

## Session Tasks
- [x] M1 实现完成（16/16 tasks, 50 tests）
- [x] M2 Task 1-3（搜索、Quick Access、ContextMenu）
- [x] M3 Task 1 Settings UI + Persistence（settings.rs + settingsStore + SettingsPanel）
- [x] M3 Task 2 Auto-lock + Brute-force（timer thread + failed_attempts + heartbeat + countdown UI）
- [x] M3 Task 3 Touch ID（Keychain + LAContext + LockScreen button + Settings toggle）
- [x] M3 Task 4 Password Generator（random/words modes + EFF diceware + expandable panel）
- [x] M3 Task 5 Recovery Kit（128-bit key + AES-GCM encrypt master_key + SetupWizard 3-step + LockScreen recovery flow）
- [ ] M2 Task 3.2 Recovery Kit（延后到 M3，已完成）

## Key Files
- `CLAUDE.md` — 项目约定和架构规则
- `src-tauri/src/commands/` — settings.rs, security.rs, recovery.rs (M3 新增)
- `src-tauri/src/state.rs` — AppState（含 last_activity for auto-lock）
- `src-tauri/data/eff-short-wordlist.txt` — EFF diceware 词表
- `src/components/settings/SettingsPanel.tsx` — 设置面板
- `src/components/entry/PasswordGenerator.tsx` — 密码生成器面板
- `src/components/lock/SetupWizard.tsx` — 3 步设置向导（密码→Recovery Kit→完成）
- `src/components/lock/LockScreen.tsx` — 含 Touch ID + brute-force countdown + 忘记密码恢复

## Next Steps
- M3 完成，VaultX 已具备 daily-driveable 能力
- 可选后续：多 vault、标签、导入导出、浏览器扩展、Sidebar 折叠
