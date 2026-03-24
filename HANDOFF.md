# HANDOFF
<!-- /checkpoint at 2026-03-24 -->

## Active Plan
M4: Quality Hardening — `docs/plans/m4-quality-hardening.md`（0/28, 0%）

## Session Tasks
- [x] Fix EntryForm category selector bug (local state + key prop)
- [x] Fix TypeScript build errors (labelKey: string → TranslationKey)
- [x] Remove unused settingsLoaded in App.tsx
- [x] Run multi-agent review (security/product/code/design)
- [x] Create M4 quality hardening plan from review findings
- [x] Run Codex CLI review, fix P1 finding
- [x] Build v0.1.0 macOS DMG locally
- [x] Create GitHub Release v0.1.0
- [x] Add CI workflow for multi-platform builds (macOS×2 + Windows)
- [x] Move macOS-only Cargo deps behind cfg(target_os)
- [ ] Verify CI builds pass (3 jobs running: macOS aarch64/x86_64, Windows x86_64)
- [ ] Start M4 Phase 1: Security Hardening (file permissions, CSP, brute-force counter)

## Key Files
- `.github/workflows/release.yml` — multi-platform release CI
- `docs/plans/m4-quality-hardening.md` — 28-task quality plan from 4-agent review
- `src/components/entry/EntryForm.tsx` — category bug fix + TS types
- `src-tauri/Cargo.toml` — macOS deps now behind target cfg

## Next Actions
- Run `gh run view` to check CI status for v0.1.0 release
- Begin M4 Phase 1 Task 1.1: `src-tauri/src/db/connection.rs` set dir permissions 0o700
