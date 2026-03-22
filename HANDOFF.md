# HANDOFF
<!-- /checkpoint at 2026-03-22 -->

## Session Tasks
- [x] Fix language resetting to English on lock (settingsStore + App.tsx)
- [x] Create GitHub repo (wh759705-creator/VaultX, public, MIT)
- [x] Add bilingual README with author info and SkillNav.dev
- [x] Add .gitignore and MIT LICENSE
- [x] Add author info to Settings About page (with GitHub/SkillNav links)
- [x] Add screenshots to README (lock, main, entry form)
- [x] Fix i18n in EntryForm (hardcoded labels → i18n keys)
- [ ] Cargo.toml has uncommitted minor change (tauri-build features)

## Key Files
- `src/stores/settingsStore.ts` — reset() preserves language
- `src/App.tsx` — I18nProvider uses language directly
- `src/components/entry/EntryForm.tsx` — labelKey + labelToKey reverse mapping
- `README.md` — bilingual with screenshots
- `docs/screenshots/` — lock-screen, main-view, entry-form PNGs
