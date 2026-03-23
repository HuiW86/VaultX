# M4: Quality Hardening

Status: pending
Progress: 0/28
Date: 2026-03-23
Source: Multi-agent review (security, product, code quality, UI/UX)

## Goal

Address critical issues identified by 4-perspective product review. Focus on security hardening, code reliability, and UX polish. After M4, VaultX reaches production-ready quality for open-source release.

## Non-Goals (Deferred to M5+)

- TOTP generator (separate milestone, ~12h)
- Import/Export wizard (separate milestone, ~8h)
- Browser extension
- Multi-vault UI
- Cross-device sync
- Watchtower / security audit panel

## Phases

### Phase 1: Security Hardening (P0, ~3h)

Critical security fixes that should ship immediately.

| # | Task | File(s) | Est |
|---|------|---------|-----|
| 1.1 | Set data directory permissions to 0o700 | `src-tauri/src/db/connection.rs` | 0.5h |
| 1.2 | Configure CSP in tauri.conf.json | `src-tauri/tauri.conf.json` | 0.5h |
| 1.3 | Move brute-force counter to Keychain (prevent reset by file deletion) | `src-tauri/src/commands/auth.rs` | 1.5h |
| 1.4 | Replace `rand::thread_rng()` with `OsRng` for salt generation | `src-tauri/src/crypto/key_derivation.rs`, `commands/recovery.rs` | 0.5h |

**Deliverable**: Data dir locked to owner; CSP active; brute-force resilient to file deletion.

### Phase 2: Code Reliability (P0-P1, ~4h)

Prevent crashes and fix correctness bugs.

| # | Task | File(s) | Est |
|---|------|---------|-----|
| 2.1 | Add React ErrorBoundary wrapping App | `src/App.tsx` + new `src/components/ui/ErrorBoundary.tsx` | 1h |
| 2.2 | Fix LockScreen hooks dependency arrays (line 38, 69) | `src/components/lock/LockScreen.tsx` | 0.5h |
| 2.3 | Replace 11x `.unwrap()` with `.expect("msg")` in commands | `src-tauri/src/commands/entries.rs`, `search.rs` | 0.5h |
| 2.4 | Add heartbeat error logging (remove silent `.catch(() => {})`) | `src/App.tsx` | 0.5h |
| 2.5 | Add IPC timeout wrapper (Promise.race, 30s default) | `src/lib/commands.ts` | 1h |
| 2.6 | Fix password history unbounded growth (cap at 20) | `src-tauri/src/db/queries.rs` | 0.5h |

**Deliverable**: No silent failures; no white-screen crashes; bounded data growth.

### Phase 3: Design Token Compliance (P1, ~4h)

Eliminate magic numbers and enforce design system consistency.

| # | Task | File(s) | Est |
|---|------|---------|-----|
| 3.1 | Extract inline styles to CSS tokens (`paddingTop: 48`, `gridTemplateColumns`) | `src/components/layout/ThreePanel.tsx` | 1h |
| 3.2 | Create `--icon-size-sm/md/lg` tokens, replace hardcoded `size={14/16/20}` | `src/styles/globals.css` + all icon usages | 1h |
| 3.3 | Fix touch target sizes to min 32px (CopyButton, Eye toggle, SearchBar clear) | `src/components/ui/CopyButton.tsx`, `PasswordField.tsx`, `SearchBar.tsx` | 1h |
| 3.4 | Verify & fix WCAG AA color contrast (`--color-text-secondary/tertiary`) | `src/styles/globals.css` | 1h |

**Deliverable**: Zero magic numbers in layout; all touch targets >= 32px; WCAG AA compliant.

### Phase 4: Interaction Completeness (P1, ~5h)

Complete the keyboard-driven experience.

| # | Task | File(s) | Est |
|---|------|---------|-----|
| 4.1 | Register global shortcuts (Cmd+K, Cmd+N, Cmd+L) at App level | `src/App.tsx` or `src/components/layout/ThreePanel.tsx` | 1.5h |
| 4.2 | Arrow key navigation in EntryList | `src/components/entry/EntryList.tsx` | 1.5h |
| 4.3 | Add shortcut hints to buttons/context menus | Various UI components | 1h |
| 4.4 | Specify transition durations using `--duration-fast/normal` tokens | Various components (`transition-colors` usages) | 1h |

**Deliverable**: Full keyboard navigation; shortcut hints visible; consistent animations.

### Phase 5: Security Documentation (P1, ~2h)

Threat model transparency for open-source trust.

| # | Task | File(s) | Est |
|---|------|---------|-----|
| 5.1 | Create `docs/SECURITY.md` documenting threat model | New file | 1h |
| 5.2 | Document encryption architecture (field coverage, JS memory limitations) | `docs/SECURITY.md` | 0.5h |
| 5.3 | Add security contact & responsible disclosure process | `docs/SECURITY.md` + `README.md` | 0.5h |

**Deliverable**: Clear threat model; users understand what VaultX does and does not protect against.

### Phase 6: Test Coverage (P2, ~6h)

Fill critical test gaps.

| # | Task | File(s) | Est |
|---|------|---------|-----|
| 6.1 | React integration tests: setup -> lock -> unlock flow | `src/__tests__/integration/` | 2h |
| 6.2 | Rust integration tests: encrypt -> store -> retrieve -> decrypt | `src-tauri/src/tests/` | 2h |
| 6.3 | Configure coverage reporting (vitest + cargo-tarpaulin) | `package.json`, `Cargo.toml` | 1h |
| 6.4 | Add Rust concurrency test (Mutex race conditions) | `src-tauri/src/tests/` | 1h |

**Deliverable**: Integration test coverage for critical paths; coverage metrics visible.

---

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Brute-force counter storage | Keychain (macOS) | File-based `.vaultx-failed-attempts` can be deleted by attacker to reset counter |
| ErrorBoundary scope | Wrap ThreePanel + SetupWizard separately | Isolate failures; lock screen must always work |
| IPC timeout | 30s default, configurable | Prevent infinite hangs if Rust deadlocks |
| Password history cap | 20 entries per field | Balance audit trail vs storage/privacy |
| Threat model scope | Local attacker with user-level access | Document: root/admin attacker is out of scope |

## Risk Assessment

| Risk | Mitigation |
|------|-----------|
| CSP breaks Tauri IPC | Test with `'self'` + Tauri-specific directives; rollback if broken |
| Keychain API changes across macOS versions | Use stable `security-framework` crate APIs; test on macOS 13+ |
| ErrorBoundary hides real bugs | Log errors to console in dev; show "Report Bug" link in prod |
| Touch target resize breaks layout | Visual regression check after each change |

## Execution Order

```
Week 1: Phase 1 (Security) + Phase 2 (Reliability)  -- 7h
Week 2: Phase 3 (Tokens) + Phase 4 (Interaction)    -- 9h
Week 3: Phase 5 (Docs) + Phase 6 (Tests)            -- 8h
```

Total estimated: ~24h across 3 weeks.

## Acceptance Criteria

- [ ] `ls -la` on data dir shows `drwx------` (0o700)
- [ ] `tauri.conf.json` has non-null CSP
- [ ] Deleting `.vaultx-failed-attempts` does not reset brute-force counter
- [ ] App recovers gracefully from component errors (no white screen)
- [ ] Zero `eslint-disable-line react-hooks/exhaustive-deps` in LockScreen
- [ ] Zero hardcoded pixel values in ThreePanel
- [ ] All interactive elements >= 32px touch target
- [ ] WCAG AA contrast ratio verified for all text colors
- [ ] Cmd+K / Cmd+N / Cmd+L / Arrow keys work as expected
- [ ] `docs/SECURITY.md` exists and covers threat model
- [ ] Integration tests pass for setup->lock->unlock flow
- [ ] Coverage reporting configured and running in CI
