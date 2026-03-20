# VaultX Design Spec v1.0

> Visual specification for AI-assisted UI development.
> Companion to `product-spec.md` — this file defines "how it looks", that file defines "how it works".
> AI must reference this file before generating any UI code.

---

## 0. Usage

### 0.1 AI Behavior Contract

When generating UI code, AI must:
1. Look up the relevant token before writing any visual value
2. Check the component spec for required states and accessibility
3. Check the interaction pattern spec for form/error/loading behaviors
4. Check the composition pattern for page-level structure
5. Use the shared vocabulary from -> PS:0.2

### 0.2 Shared Component Vocabulary

> Defined in product-spec.md §0.2 (single source of truth). Component names used in this file must match that table. When adding a new component to this spec, register it in PS:0.2 first.

### 0.3 Cross-Reference Convention

- `-> DS:x.x` = this file section x.x
- `-> PS:x.x` = product-spec.md section x.x

---

## 1. Design Principles

### 1.1 Visual Style Keywords

Dark-first, macOS-native, 1Password 8 design language, clean, professional, high information density with clear hierarchy.

### 1.2 Core Design Constraints

- Desktop-first (macOS), minimum window 900×600px
- Dark theme is primary; light theme is secondary
- Three-panel layout as default, responsive panel collapse
- Keyboard-navigable — every action reachable without mouse
- Sensitive data hidden by default (password fields masked)

### 1.3 Priority Order

Security > Function > Clarity > Aesthetics > Animation

When trade-offs arise, security always wins. Never show sensitive data in animations, transitions, or error messages.

### 1.4 Interaction Design Principles

> Derived from 1Password UX analysis. See -> PS:1.4 for the product perspective.

| Principle | Design Implication |
|-----------|-------------------|
| **Zero-config security** | Security controls (lock timeout, clipboard clear) never appear as setup blockers. Use best-practice defaults, let users discover settings later. No security modals during onboarding except Recovery Kit. |
| **Make security visible** | StrengthMeter is always visible next to password fields. CopyButton shows a live countdown ("clearing in 28s"). Lock icon pulses briefly when auto-lock engages. These micro-cues build trust without interrupting flow. |
| **One place for everything** | All 5 entry types share the same visual framework: title bar + field list + action buttons. Category differences are expressed through field templates (-> PS:2.4), not different page layouts. |
| **Least steps to goal** | Quick Access: invoke → type → Enter = done. Main UI: click field → copied. No confirmation dialogs for non-destructive actions (copy, search, navigate). |
| **Smart defaults, no interruptions** | Password generator auto-fills on new Login entries. Touch ID pre-selected in setup. Theme follows system. No "are you sure?" for reversible actions. |
| **Progressive disclosure** | Entry edit form starts with only the category's default fields. "Add field" button reveals extras. Password generator is collapsed by default, expands inline. Notes field hidden until clicked. |

---

## 2. Design Token System

> Every visual value must use a token. Dark theme is the primary definition.

### 2.1 Color System

#### 2.1.1 Brand / Primary Colors

| Token | Dark (primary) | Light | Usage |
|-------|---------------|-------|-------|
| `--color-primary` | `#0066FF` | `#0066FF` | Primary actions: main buttons, active states, links |
| `--color-primary-hover` | `#1A7AFF` | `#0055DD` | Hover state of primary elements |
| `--color-primary-active` | `#0052CC` | `#0044BB` | Active/pressed state |
| `--color-primary-bg` | `rgba(0,102,255,0.12)` | `rgba(0,102,255,0.08)` | Selected entry background, active sidebar item |
| `--color-accent` | `#4CC9F0` | `#0EA5E9` | Secondary highlights, TOTP timer, links |

#### 2.1.2 Functional Colors

| Token | Dark | Light | Usage |
|-------|------|-------|-------|
| `--color-success` | `#10B981` | `#10B981` | Strong password, save success |
| `--color-success-bg` | `rgba(16,185,129,0.12)` | `rgba(16,185,129,0.08)` | Strength meter strong zone |
| `--color-warning` | `#F59E0B` | `#F59E0B` | Medium password, expiry warning |
| `--color-warning-bg` | `rgba(245,158,11,0.12)` | `rgba(245,158,11,0.08)` | Strength meter medium zone |
| `--color-error` | `#EF4444` | `#EF4444` | Weak password, breach detected, delete actions |
| `--color-error-bg` | `rgba(239,68,68,0.12)` | `rgba(239,68,68,0.08)` | Error backgrounds |

#### 2.1.3 Neutral Colors

| Token | Dark | Light | Usage |
|-------|------|-------|-------|
| `--color-bg-app` | `#1A1A2E` | `#F5F5F7` | App background (behind all panels) |
| `--color-bg-sidebar` | `#16163A` | `#EEEEF0` | Sidebar background |
| `--color-bg-panel` | `#222244` | `#FFFFFF` | EntryList and DetailPanel background |
| `--color-bg-elevated` | `#2A2A50` | `#FFFFFF` | Modals, popovers, Quick Access panel |
| `--color-bg-input` | `#1E1E3A` | `#FFFFFF` | Input field background |
| `--color-bg-hover` | `rgba(255,255,255,0.06)` | `rgba(0,0,0,0.04)` | List item hover, sidebar item hover |
| `--color-bg-spotlight` | `rgba(0,0,0,0.60)` | `rgba(0,0,0,0.45)` | Modal overlay backdrop |
| `--color-border` | `rgba(255,255,255,0.10)` | `#D9D9D9` | Default border, panel dividers |
| `--color-border-light` | `rgba(255,255,255,0.06)` | `#F0F0F0` | Subtle dividers within panels |
| `--color-text-primary` | `#E8E8F0` | `rgba(0,0,0,0.88)` | Titles, entry names, field values |
| `--color-text-secondary` | `#8888AA` | `rgba(0,0,0,0.65)` | Subtitles, descriptions, labels |
| `--color-text-tertiary` | `#777799` | `rgba(0,0,0,0.45)` | Placeholders, hints, disabled nav items |
| `--color-text-inverse` | `#FFFFFF` | `#FFFFFF` | Text on colored backgrounds (buttons) |

#### 2.1.4 Dark/Light Mode Strategy

- Default: follow system (`prefers-color-scheme`), with manual override in Settings (Dark / Light / System)
- All tokens define both dark and light values
- Images/icons: follow text color tokens via `currentColor`
- Shadows: both modes have dedicated values (-> DS:2.5)

### 2.2 Typography

#### 2.2.1 Font Family

| Role | Stack |
|------|-------|
| UI text | `-apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Geist', 'Helvetica Neue', sans-serif` |
| Headings | `-apple-system, BlinkMacSystemFont, 'SF Pro Display', 'Geist', 'Helvetica Neue', sans-serif` |
| Monospace (passwords, keys, code) | `'JetBrains Mono', 'SF Mono', 'Menlo', monospace` |

#### 2.2.5 Font Family Tokens

```css
:root {
  --font-sans: -apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Geist', 'Helvetica Neue', sans-serif;
  --font-display: -apple-system, BlinkMacSystemFont, 'SF Pro Display', 'Geist', 'Helvetica Neue', sans-serif;
  --font-mono: 'JetBrains Mono', 'SF Mono', 'Menlo', monospace;
}
```

#### 2.2.2 Font Size Scale

| Token | Value | Usage |
|-------|-------|-------|
| `--font-size-display` | `2rem` (32px) | Lock screen title |
| `--font-size-h1` | `1.5rem` (24px) | Detail panel entry title |
| `--font-size-h2` | `1.25rem` (20px) | Section titles, sidebar vault name |
| `--font-size-lg` | `1rem` (16px) | Entry card title, primary field values |
| `--font-size-md` | `0.875rem` (14px) | Body text, field labels, sidebar items (base size) |
| `--font-size-sm` | `0.8125rem` (13px) | Secondary text, entry card subtitle |
| `--font-size-xs` | `0.75rem` (12px) | Timestamps, helper text, badges |
| `--font-size-mini` | `0.6875rem` (11px) | Fine print |

#### 2.2.3 Line Height

| Token | Value | Usage |
|-------|-------|-------|
| `--line-height-tight` | `1.25` | Headings, single-line labels |
| `--line-height-normal` | `1.5` | Body text (default) |
| `--line-height-relaxed` | `1.75` | Long notes content |

#### 2.2.4 Font Weight

| Token | Value | Usage |
|-------|-------|-------|
| `--font-weight-regular` | `400` | Body text, field values |
| `--font-weight-medium` | `500` | Field labels, sidebar items, nav items |
| `--font-weight-semibold` | `600` | Entry card titles, section headers |
| `--font-weight-bold` | `700` | Page titles, lock screen title |

```css
/* CORRECT */
.entry-title {
  font-size: var(--font-size-h1);
  font-weight: var(--font-weight-semibold);
  line-height: var(--line-height-tight);
}

/* INCORRECT */
.entry-title { font-size: 24px; font-weight: 600; }
```

### 2.3 Spacing System

Base unit: `4px`

| Token | Value | Usage |
|-------|-------|-------|
| `--spacing-xs` | `4px` | Icon-to-text gap, inline element gaps |
| `--spacing-sm` | `8px` | Within components: button icon gap, list item internal |
| `--spacing-md` | `12px` | Component internal padding: card content, input padding |
| `--spacing-lg` | `16px` | Between components: entry cards, form fields |
| `--spacing-xl` | `24px` | Between sections within a panel |
| `--spacing-2xl` | `32px` | Major section breaks, panel padding |
| `--spacing-3xl` | `48px` | Lock screen spacing |

#### Spacing Patterns

| Pattern | Rule |
|---------|------|
| Sidebar internal | padding: `--spacing-sm`, item gap: `2px` |
| EntryList internal | padding: `--spacing-sm`, card gap: `2px` |
| DetailPanel internal | padding: `--spacing-2xl`, section gap: `--spacing-xl` |
| Form fields | gap: `--spacing-lg` |
| Modal internal | padding: `--spacing-xl`, button gap: `--spacing-sm` |

```css
/* CORRECT */
.detail-panel { padding: var(--spacing-2xl); }
.detail-section + .detail-section { margin-top: var(--spacing-xl); }

/* INCORRECT */
.detail-panel { padding: 32px; }
.detail-section + .detail-section { margin-top: 20px; }
```

### 2.4 Border Radius

| Token | Value | Usage |
|-------|-------|-------|
| `--radius-none` | `0` | Full-width dividers |
| `--radius-sm` | `4px` | Tags, badges, small elements |
| `--radius-md` | `6px` | Buttons, inputs, entry cards |
| `--radius-lg` | `8px` | Panels, cards, containers |
| `--radius-xl` | `12px` | Modals, Quick Access panel |
| `--radius-full` | `50%` | Avatars, vault icons |

### 2.5 Shadow System

| Token | Dark | Light | Usage |
|-------|------|-------|-------|
| `--shadow-sm` | `0 1px 3px rgba(0,0,0,0.3)` | `0 1px 3px rgba(0,0,0,0.08)` | Entry cards (subtle lift) |
| `--shadow-md` | `0 4px 12px rgba(0,0,0,0.4)` | `0 4px 12px rgba(0,0,0,0.12)` | Tooltips, popovers, context menus |
| `--shadow-lg` | `0 8px 32px rgba(0,0,0,0.5)` | `0 8px 32px rgba(0,0,0,0.16)` | Modals, Quick Access panel |
| `--shadow-window` | `0 16px 48px rgba(0,0,0,0.6)` | `0 16px 48px rgba(0,0,0,0.2)` | Window drop shadow (Tauri managed) |

### 2.6 Motion System

| Token | Value | Usage |
|-------|-------|-------|
| `--duration-fast` | `100ms` | Password reveal, button press, tooltip |
| `--duration-normal` | `200ms` | Panel transitions, toast enter/exit, sidebar collapse |
| `--duration-slow` | `300ms` | Modal open/close, Quick Access appear/dismiss |
| `--ease-out` | `cubic-bezier(0, 0, 0.2, 1)` | Elements entering (modal open, toast in) |
| `--ease-in` | `cubic-bezier(0.4, 0, 1, 1)` | Elements exiting (modal close, toast out) |
| `--ease-in-out` | `cubic-bezier(0.4, 0, 0.2, 1)` | Elements moving (sidebar collapse, panel resize) |

```css
/* CORRECT */
@media (prefers-reduced-motion: reduce) {
  * { transition-duration: 0ms !important; animation-duration: 0ms !important; }
}
```

---

## 3. Layout System

### 3.1 Three-Panel Layout

```
┌──────────────────────────────────────────────────────────────┐
│ [Titlebar / Traffic Lights]                    [window controls] │
├─────────┬──────────────┬─────────────────────────────────────┤
│ Sidebar │  EntryList   │          DetailPanel                │
│         │              │                                     │
│ 220px   │   300px      │          flex: 1 (min 400px)        │
│         │              │                                     │
│ ┌─────┐ │ ┌──────────┐ │  ┌───────────────────────────────┐  │
│ │Vault│ │ │ Search   │ │  │  Entry Title                  │  │
│ │List │ │ ├──────────┤ │  │  ──────────────────────────── │  │
│ │     │ │ │EntryCard │ │  │  Field 1: username   [copy]   │  │
│ │Cate-│ │ │EntryCard │ │  │  Field 2: ●●●●●●●●  [👁][copy]│  │
│ │gory │ │ │EntryCard │ │  │  Field 3: URL        [copy]   │  │
│ │     │ │ │  ...     │ │  │  ...                          │  │
│ │Tags │ │ │          │ │  │                               │  │
│ │     │ │ │          │ │  │  [Edit]  [Delete]             │  │
│ ├─────┤ │ │          │ │  └───────────────────────────────┘  │
│ │ ⚙️  │ │ │          │ │                                     │
│ │ 🔒  │ │ │          │ │                                     │
│ └─────┘ │ └──────────┘ │                                     │
├─────────┴──────────────┴─────────────────────────────────────┤
```

#### Panel Sizing

| Panel | Width | Min Width | Resizable |
|-------|-------|-----------|-----------|
| Sidebar | `220px` (expanded) / `48px` (collapsed) | `48px` | Toggle only (not drag) |
| EntryList | `300px` | `240px` | Drag border |
| DetailPanel | `flex: 1` | `400px` | Auto-fill |

#### Responsive Behavior

| Window Width | Behavior |
|-------------|----------|
| >= 1100px | Full three-panel layout |
| 900-1099px | Sidebar collapses to icon-only (48px) |
| < 900px | Not supported (minimum window size) |

### 3.2 Panel Dividers

- Vertical dividers between panels: `1px solid var(--color-border)`
- Divider is the drag handle for resizing (cursor: `col-resize`)
- No horizontal dividers within panels (use spacing instead)

### 3.3 Titlebar

- Use Tauri's custom titlebar (transparent, integrated)
- macOS traffic lights (close/minimize/maximize) positioned at top-left
- Traffic light offset: `left: 16px, top: 16px`
- Draggable region: full titlebar area except interactive elements
- Sidebar header (vault name / app logo) shares the titlebar row

### 3.4 Scrolling Rules

| Panel | Scrolling |
|-------|-----------|
| Sidebar | Vertical scroll, hidden scrollbar (show on hover) |
| EntryList | Vertical scroll, virtual list for > 100 items, hidden scrollbar |
| DetailPanel | Vertical scroll for long entries, visible thin scrollbar |
| Quick Access | Vertical scroll, max height 60vh |

---

## 4. Component Specs

### 4.1 Modal / Dialog

| Property | Value |
|----------|-------|
| Width | `480px` |
| Border radius | `--radius-xl` |
| Padding | `--spacing-xl` |
| Background | `--color-bg-elevated` |
| Overlay | `--color-bg-spotlight` |
| Shadow | `--shadow-lg` |
| Title | `--font-size-lg`, `--font-weight-semibold` |
| Body | `--font-size-md`, `--color-text-secondary` |

- Buttons: side-by-side (cancel left, confirm right)
- Danger confirm: use `--color-error` button for destructive actions
- Close: Escape key or click overlay (non-destructive modals only)
- Danger confirm modals: overlay click does NOT close — must explicitly click Cancel or confirm
- Focus trap: Tab cycles within modal
- Enter: activates primary button
- Animation: scale(0.95) + fade -> scale(1) + opaque, `--duration-slow`, `--ease-out`

### 4.2 Toast

| Property | Value |
|----------|-------|
| Position | Bottom-center, `48px` from bottom |
| Max width | `360px` |
| Padding | `--spacing-sm` vertical, `--spacing-md` horizontal |
| Border radius | `--radius-md` |
| Background | `--color-bg-elevated` |
| Border | `1px solid var(--color-border)` |
| Text | `--font-size-sm`, `--color-text-primary` |
| Icon | `16px`, left of text |
| Duration | `2000ms` (info/success), `4000ms` (error) |

Variants:
- **Copy success**: check icon + "Copied, clearing in 30s" + countdown timer text
- **Save success**: check icon + "Saved"
- **Error**: error icon (red) + message
- **Clipboard cleared**: info icon + "Clipboard cleared"

Animation: slide-up + fade in, `--duration-normal`

### 4.3 Feedback States (EmptyState / ErrorState / Skeleton)

#### EmptyState

| Property | Value |
|----------|-------|
| Illustration size | `120px` |
| Title | `--font-size-lg`, `--font-weight-medium`, `--color-text-primary` |
| Description | `--font-size-sm`, `--color-text-secondary` |
| Action button | Secondary button below description |
| Vertical position | Centered in panel |

Contextual messages:
- Empty vault: "No items yet" + "Add your first item" button
- No search results: "No matching items" + "Try a different search"
- No selection: "Select an item to view details" (DetailPanel only, no illustration)

#### ErrorState

Inherits EmptyState layout. Error icon in `--color-error`. Always includes retry button.

#### Skeleton

- Match exact layout of the content it replaces
- EntryCard skeleton: icon circle + two text lines
- DetailPanel skeleton: title bar + 4 field rows
- Shimmer animation: left-to-right gradient sweep, `--duration-slow` × 2 loop
- Background: `var(--color-bg-hover)`
- Highlight: `rgba(255,255,255,0.06)` sweep

### 4.4 Tooltip / ContextMenu

#### Tooltip

| Property | Value |
|----------|-------|
| Max width | `240px` |
| Padding | `--spacing-xs` `--spacing-sm` |
| Border radius | `--radius-md` |
| Background | `--color-bg-elevated` |
| Border | `1px solid var(--color-border)` |
| Shadow | `--shadow-md` |
| Font size | `--font-size-xs` |
| Delay | `500ms` show, `100ms` hide |
| Arrow | `6px` triangle |

#### ContextMenu (right-click)

| Property | Value |
|----------|-------|
| Min width | `180px` |
| Max width | `260px` |
| Item height | `32px` |
| Item padding | `--spacing-sm` `--spacing-md` |
| Border radius | `--radius-lg` |
| Background | `--color-bg-elevated` |
| Shadow | `--shadow-md` |
| Separator | `1px solid var(--color-border-light)`, margin `--spacing-xs` vertical |
| Shortcut text | `--font-size-xs`, `--color-text-tertiary`, right-aligned |

Standard entry context menu items:
- Copy Username — `Cmd+U`
- Copy Password — `Cmd+Shift+C`
- Copy URL — `Cmd+Shift+U`
- separator
- Edit — `Cmd+E`
- Add to Favorites / Remove from Favorites
- separator
- Move to Trash — `Cmd+Backspace` (danger color)

### 4.5 PasswordField / StrengthMeter / CopyButton

#### PasswordField

| Property | Value |
|----------|-------|
| Height | `40px` |
| Font | `--font-mono`, `--font-size-lg` |
| Background | `--color-bg-input` |
| Border | `1px solid var(--color-border)` |
| Border radius | `--radius-md` |
| Padding | `--spacing-md` (left), `80px` (right, for action buttons) |

States:
- **Masked** (default): display `●●●●●●●●` in monospace
- **Revealed**: show plaintext, triggered by hover on eye icon or click
- **Edit mode**: standard text input with generate button

Action buttons (right-aligned inside field):
- Eye icon (toggle reveal): `20px`, `--color-text-tertiary`, hover: `--color-text-primary`
- CopyButton: `20px`, `--color-text-tertiary`, hover: `--color-primary`
- Generate (edit mode only): dice icon, `20px`, `--color-primary`

Reveal behavior:
- Click eye: toggle persistent reveal/mask
- Hover eye: reveal while hovering only (release to mask)
- Auto-mask: re-mask after `30s` of reveal if no interaction

#### StrengthMeter

| Property | Value |
|----------|-------|
| Height | `4px` |
| Border radius | `--radius-full` |
| Background | `--color-border` (track) |
| Width | `100%` of parent |

| zxcvbn Score | Fill Width | Color | Label |
|-------------|-----------|-------|-------|
| 0 | 10% | `--color-error` | Very weak |
| 1 | 25% | `--color-error` | Weak |
| 2 | 50% | `--color-warning` | Fair |
| 3 | 75% | `--color-success` | Strong |
| 4 | 100% | `--color-success` | Very strong |

Label: `--font-size-xs`, same color as bar, right-aligned above meter.

Transition: width + color change with `--duration-normal`, `--ease-in-out`.

#### CopyButton

| Property | Value |
|----------|-------|
| Size | `20px` icon, `32px` touch target |
| Default icon | Clipboard outline, `--color-text-tertiary` |
| Hover | `--color-primary` |
| Success state | Checkmark icon, `--color-success`, `2s` then revert |
| Tooltip | "Copy [field_name]" |

On click: copy value → show checkmark → trigger Toast with countdown → start 30s clipboard clear timer.

### 4.6 EntryCard / VaultIcon / CategoryBadge

#### EntryCard

| Property | Value |
|----------|-------|
| Min height | `60px` |
| Padding | `--spacing-sm` `--spacing-md` |
| Border radius | `--radius-md` |
| Background | transparent |
| Hover | `--color-bg-hover` |
| Selected | `--color-primary-bg` |

Layout:
```
[Icon 32px] --spacing-sm [Title --font-size-md --font-weight-medium]  [Favorite ★]
                          [Subtitle --font-size-sm --color-text-secondary]
```

- Icon: favicon (if URL available) or CategoryBadge default icon
- Favicon fallback: first letter of title on colored circle
- Favorite star: `--color-warning` when active, hidden when inactive (show on hover)
- Selected state: left border `3px solid var(--color-primary)` + `--color-primary-bg`

#### VaultIcon

| Property | Value |
|----------|-------|
| Size | `24px` (sidebar) |
| Border radius | `--radius-sm` |
| Background | generated from vault name (deterministic color hash) |
| Font | emoji or first letter of vault name, `--font-size-sm` |

#### CategoryBadge

| Category | Icon | Color |
|----------|------|-------|
| Login | Key icon | `--color-primary` |
| Card | Credit card icon | `--color-warning` |
| Note | Note icon | `--color-accent` |
| Identity | Person icon | `--color-success` |
| SSH Key | Terminal icon | `--color-text-secondary` |

Size: `14px` icon, displayed as small tag in entry card or detail header.

#### Icon Library

Use [Lucide](https://lucide.dev/) icons (MIT license, tree-shakeable SVG). Import only needed icons to minimize bundle size. All icons render via `currentColor` to follow text color tokens.

#### Entry Type Visual Rules

All entry types share the same Detail/Edit panel layout. Differences are expressed through field templates (-> PS:2.4) and these visual cues:

| Category | Detail Header | Sensitive Fields | Subtitle Source |
|----------|--------------|-----------------|-----------------|
| Login | Favicon (or fallback) + domain text | Password: PasswordField masked | Username |
| Card | Credit card icon + brand color | Card number: show only last 4; CVV + PIN: masked | `•••• {last4}` |
| Note | Note icon | None | First line of note, truncated |
| Identity | Person icon | None | Full name |
| SSH Key | Terminal icon | Private key: masked multiline; Passphrase: masked | Key fingerprint |

**Favicon display rules:**
- Size: `32px` in EntryCard, `48px` in DetailPanel header
- Border radius: `--radius-md`
- Fallback chain: favicon → first letter on colored circle (color from domain hash) → CategoryBadge icon
- Loading: show CategoryBadge icon immediately, swap to favicon when loaded (no skeleton for icons)

**Card number formatting:**
- Edit mode: auto-insert space every 4 digits as user types (e.g., `4242 4242 4242 4242`)
- View mode: masked with last 4 visible: `•••• •••• •••• 4242`
- Monospace font (`--font-mono`) for card numbers

**SSH Key display:**
- Private key: masked by default, show only first/last line when revealed (e.g., `-----BEGIN OPENSSH PRIVATE KEY-----` ... `-----END OPENSSH PRIVATE KEY-----`)
- Public key: always visible, monospace, single-line truncated with expand
- Fingerprint: `--font-mono`, `--font-size-sm`, always visible

### 4.7 Generic Components (Button / Input / Tag / Switch)

#### Button

| Type | Background | Text | Border |
|------|-----------|------|--------|
| primary | `--color-primary` | `--color-text-inverse` | none |
| secondary | transparent | `--color-primary` | `1px solid var(--color-primary)` |
| ghost | transparent | `--color-text-primary` | `1px solid var(--color-border)` |
| danger | `--color-error` | `--color-text-inverse` | none |
| text | transparent | `--color-primary` | none |

Sizes:
| Size | Height | Font | Padding H |
|------|--------|------|-----------|
| lg | `40px` | `--font-size-md` | `--spacing-xl` |
| md | `32px` | `--font-size-sm` | `--spacing-lg` |
| sm | `28px` | `--font-size-xs` | `--spacing-md` |

States:
- hover: lighten bg 8% or use hover token
- active: darken bg 8% or use active token
- disabled: opacity `0.4`, cursor `not-allowed`
- loading: inline spinner, maintain width

```css
/* CORRECT */
.btn-primary { background: var(--color-primary); color: var(--color-text-inverse); }

/* INCORRECT */
.btn-primary { background: #0066FF; color: white; }
```

#### Input

| Property | Value |
|----------|-------|
| Height | `36px` |
| Font | `--font-size-md` |
| Background | `--color-bg-input` |
| Border | `1px solid var(--color-border)` |
| Border radius | `--radius-md` |
| Padding | `--spacing-md` |
| Placeholder color | `--color-text-tertiary` |

States: default / focus (`--color-primary` border) / error (`--color-error` border + bg) / disabled

#### Tag

| Property | Value |
|----------|-------|
| Height | `22px` |
| Padding H | `--spacing-sm` |
| Font | `--font-size-xs` |
| Border radius | `--radius-sm` |
| Background | `--color-primary-bg` |
| Text | `--color-primary` |

#### Switch

| Property | Value |
|----------|-------|
| Width | `40px` |
| Height | `22px` |
| Active | `--color-primary` |
| Inactive | `--color-border` |
| Thumb | `18px` circle, white |
| Transition | `--duration-fast` |

---

## 5. Interaction Patterns

### 5.1 Keyboard Navigation

VaultX is a desktop app — keyboard navigation is a first-class citizen.

#### Global Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd+Shift+Space` | Open/close Quick Access (global, works from any app) |
| `Cmd+K` | Focus search bar (within VaultX) |
| `Cmd+N` | New entry |
| `Cmd+E` | Edit selected entry |
| `Cmd+S` | Save entry (in edit mode) |
| `Cmd+L` | Lock vault |
| `Cmd+,` | Open settings |
| `Cmd+Shift+C` | Copy password of selected entry |
| `Cmd+Shift+U` | Copy username of selected entry |
| `Cmd+Backspace` | Move selected entry to trash |
| `Escape` | Cancel edit / close modal / close Quick Access |

#### Panel Navigation

| Key | Behavior |
|-----|----------|
| `↑` / `↓` | Move selection in EntryList |
| `Enter` | Open selected entry in DetailPanel / confirm modal |
| `Tab` | Move focus between panels (Sidebar → EntryList → DetailPanel) |
| `Shift+Tab` | Reverse panel focus |

#### Focus Management

- On unlock: focus search bar
- On new entry: focus title field
- On modal open: focus first interactive element, trap Tab within modal
- On modal close: restore focus to triggering element
- Focus indicator: `2px solid var(--color-primary)`, offset `2px`, only on `:focus-visible`

```css
/* CORRECT */
*:focus-visible { outline: 2px solid var(--color-primary); outline-offset: 2px; }
*:focus:not(:focus-visible) { outline: none; }

/* INCORRECT */
*:focus { outline: none; }
```

### 5.2 Form Flow

#### Entry Edit Form

| Phase | Behavior |
|-------|----------|
| Enter edit | Pre-fill all fields from current entry; focus first editable field |
| Validation | Inline: validate on blur; title is required; all other fields optional |
| Password change | Old password saved to history automatically |
| Save | `Cmd+S` or Save button; encrypt → persist → switch to view mode |
| Cancel | `Escape` or Cancel button; if changes exist, confirm discard via Modal |
| Error | Toast "Save failed" + keep form data; retry on next save |

#### Password Generator (embedded panel)

| Phase | Behavior |
|-------|----------|
| Open | Show in DetailPanel below password field, or as standalone panel |
| Generate | Instant on parameter change (< 50ms); show result + StrengthMeter |
| Use | "Use Password" button fills the password field in edit form |
| Copy | "Copy" button copies to clipboard (same CopyButton behavior) |
| History | Last 20 generated passwords stored (encrypted) and accessible |

#### Progressive Disclosure in Edit Form

The edit form uses progressive disclosure to reduce visual noise (-> PS:1.4, DS:1.4):

| Element | Default State | Reveal Trigger |
|---------|--------------|----------------|
| Password generator panel | Collapsed (only dice icon visible) | Click dice icon or "Generate" link |
| Notes field | Hidden | Click "+ Add Notes" text button at bottom of form |
| Custom fields | Hidden | Click "+ Add Field" text button at bottom of form |
| Tags | Collapsed to inline display | Click tag area to enter edit mode |
| Password history | Hidden | Click "History" link next to password field (view mode only) |

Collapse/expand animation: height transition `--duration-normal`, `--ease-in-out`. Content fades in with `--duration-fast` delay.

### 5.3 Password Reveal Pattern

| Interaction | Behavior |
|-------------|----------|
| Click eye icon | Toggle reveal/mask (persistent until toggled again or 30s timeout) |
| Hover eye icon | Temporarily reveal while hovering (release to re-mask) |
| Auto-mask timer | If revealed via click, auto-mask after `30s` of no interaction |
| Copy | Copies actual value regardless of reveal state |
| Screen lock | All revealed fields immediately mask on app lock |

### 5.4 Error Recovery Patterns

| Scenario | Component | Behavior |
|----------|-----------|----------|
| DB read error | ErrorState | Show in affected panel; retry button reloads data |
| Save failure | Toast (error) | Keep form data; user retries save |
| Search index corrupt | None (silent) | Rebuild index in background; fall back to SQL LIKE |
| Encryption error | ErrorState | Show in DetailPanel; "Restart App" suggestion |
| Import parse error | Modal | Show error details (line number, expected format); user fixes file |

### 5.5 Loading Strategy

| Scenario | Strategy |
|----------|----------|
| App startup (unlock) | LockOverlay → Argon2id processing (< 1s visual indicator) → main UI |
| EntryList first load | Skeleton × 5 in list panel |
| Entry detail load | Skeleton in detail panel (decrypt takes < 50ms, may not even show) |
| Search | No loading state (< 100ms); if slower, show spinner in search bar |
| Import | Modal with progress bar (item count / total) |
| Password generate | Instant (no loading state needed) |

---

## 6. Composition Patterns

### 6.1 Lock Screen (LockOverlay)

```
┌──────────────────────────────────────────────────┐
│                                                  │
│                                                  │
│              [VaultX Logo 64px]                   │
│              VaultX                               │
│                                                  │
│         ┌──────────────────────┐                 │
│         │  Master Password     │                 │
│         │  ●●●●●●●●           │                 │
│         └──────────────────────┘                 │
│                                                  │
│         [Unlock Button - primary, lg]            │
│                                                  │
│         [Touch ID icon + "Use Touch ID"]         │
│                                                  │
│                                                  │
│   ┌──────────────────────────────┐               │
│   │ Error: Wrong password (4/5)  │  <- error     │
│   └──────────────────────────────┘               │
│                                                  │
└──────────────────────────────────────────────────┘

Background: --color-bg-app with subtle gradient
Title: --font-size-display, --font-weight-bold
Password input: centered, max-width 320px
Touch ID: --color-text-secondary, hover: --color-primary
Error: --color-error, shake animation on wrong password
```

### 6.1.1 Setup Wizard (first_run only)

Replaces LockOverlay content on first launch. Single-panel centered layout, max-width `480px`.

```
┌──────────────────────────────────────────────────┐
│                                                  │
│              [VaultX Logo 64px]                   │
│              Welcome to VaultX                   │
│                                                  │
│         Step 1 of 2            [● ○]             │
│                                                  │
│         Create Your Master Password              │
│         ┌──────────────────────┐                 │
│         │  ●●●●●●●●           │                 │
│         └──────────────────────┘                 │
│         [████████░░░░] Strong                    │
│                                                  │
│         Confirm Password                         │
│         ┌──────────────────────┐                 │
│         │  ●●●●●●●●           │                 │
│         └──────────────────────┘                 │
│                                                  │
│         [✓] Enable Touch ID                      │
│                                                  │
│         [Continue - primary, lg, full-width]     │
│                                                  │
└──────────────────────────────────────────────────┘

Step 2: Save Your Recovery Kit
  ┌──────────────────────────────────┐
  │  🔑 Save Your Recovery Kit       │
  │                                  │
  │  This is your ONLY way to reset  │
  │  your master password. VaultX    │
  │  has no cloud — if you forget    │
  │  your password without this kit, │
  │  your data is lost forever.      │
  │                                  │
  │  [Download Recovery Kit PDF]     │  <- primary, lg
  │                                  │
  │  [Remind me later]               │  <- text button, --color-text-secondary
  └──────────────────────────────────┘

Step 3: Choose how to start
  [Import from 1Password / Chrome / CSV]   <- opens file picker
  [Start from scratch]                      <- creates empty vault, enters main UI
```

- Step 1: master password + Touch ID toggle (smart defaults, no extra config)
- Step 2: recovery kit download — critical for local-only app, strongly encouraged but not forced
- Step 3: import or start empty — auto-creates "Personal" vault with defaults
- Auto-lock timeout, clipboard timer use sensible defaults (8h, 30s), configurable later in Settings
- Progress indicator: `--font-size-xs`, `--color-text-tertiary`, `[● ● ○]` for 3 steps

### 6.2 Three-Panel Main Interface

See -> DS:3.1 for full layout spec.

#### Sidebar

```
[App Logo / Vault Selector]         <- titlebar row
────────────────────────────
[Search shortcut: Cmd+K]           <- optional quick link
────────────────────────────
VAULTS
  [VaultIcon] Personal        (42)
  [VaultIcon] Work            (18)
  [+ New Vault]
────────────────────────────
CATEGORIES
  All Items                   (60)
  Logins                      (35)
  Cards                       (8)
  Notes                       (10)
  Identities                  (5)
  SSH Keys                    (2)
────────────────────────────
  Favorites                   (7)
  Trash                       (3)
────────────────────────────
  [⚙️ Settings]
  [🔒 Lock]
```

- Sections: `--font-size-xs`, `--font-weight-medium`, `--color-text-tertiary`, uppercase
- Items: `--font-size-md`, `--font-weight-medium`
- Count badges: `--font-size-xs`, `--color-text-tertiary`, right-aligned
- Active item: `--color-primary-bg` background, `--color-primary` text
- Hover item: `--color-bg-hover`
- Collapsed mode: only icons, tooltip on hover showing full name

#### EntryList

```
┌──────────────────┐
│ 🔍 Search...     │  <- Cmd+K to focus
├──────────────────┤
│ Sort: Recent ▾   │  <- dropdown: Recent / Name / Created
├──────────────────┤
│ [EntryCard]      │
│ [EntryCard] ←sel │  <- selected state
│ [EntryCard]      │
│ [EntryCard]      │
│ ...              │
└──────────────────┘
```

#### DetailPanel - View Mode

```
┌──────────────────────────────────────┐
│  [CategoryBadge] Entry Title    [★]  │  <- --font-size-h1
│  website.com                         │  <- --font-size-sm, --color-text-secondary
│                                      │
│  ───────────────────────────────     │
│                                      │
│  Username                            │  <- --font-size-xs, --color-text-tertiary
│  john@example.com           [copy]   │  <- --font-size-md
│                                      │
│  Password                            │
│  ●●●●●●●●●●●●     [👁] [copy]      │  <- PasswordField
│                                      │
│  Website                             │
│  https://example.com  [open] [copy]  │
│                                      │
│  Notes                               │
│  Some additional notes here...       │
│                                      │
│  ───────────────────────────────     │
│                                      │
│  Tags: [work] [important]            │  <- Tag components
│                                      │
│  Created: 2026-01-15                 │  <- --font-size-xs, --color-text-tertiary
│  Modified: 2026-03-18                │
│                                      │
│  [Edit]  [Delete]                    │
└──────────────────────────────────────┘
```

### 6.3 Entry Edit Panel

Same position as DetailPanel, replaces view mode content. Field templates per category -> PS:2.4.

```
┌──────────────────────────────────────┐
│  Edit Login                  [Save]  │
│                             [Cancel] │
│  ───────────────────────────────     │
│                                      │
│  Title *                             │
│  ┌──────────────────────────────┐    │
│  │ My Website Login             │    │
│  └──────────────────────────────┘    │
│                                      │
│  Username                            │
│  ┌──────────────────────────────┐    │
│  │ john@example.com             │    │
│  └──────────────────────────────┘    │
│                                      │
│  Password                            │
│  ┌──────────────────────────┐ [🎲]  │  <- generate button
│  │ ●●●●●●●●●●            👁 │       │
│  └──────────────────────────┘        │
│  [████████████░░░] Strong            │  <- StrengthMeter
│                                      │
│  ┌ Password Generator ─────────┐    │  <- expandable panel
│  │ Length: [20] ──────●──────  │    │
│  │ [✓] Uppercase  [✓] Lower   │    │
│  │ [✓] Digits     [✓] Symbols │    │
│  │ [Random ▾]                  │    │
│  │                             │    │
│  │ aX9#mK2$pL8@nQ4&          │    │
│  │            [Use] [Refresh]  │    │
│  └─────────────────────────────┘    │
│                                      │
│  Website                             │
│  ┌──────────────────────────────┐    │
│  │ https://example.com          │    │
│  └──────────────────────────────┘    │
│                                      │
│  Notes                               │
│  ┌──────────────────────────────┐    │
│  │                              │    │
│  │                              │    │
│  └──────────────────────────────┘    │
│                                      │
│  Tags                                │
│  [work] [important] [+ Add tag]      │
│                                      │
│  [+ Add Field]                       │
│                                      │
└──────────────────────────────────────┘
```

### 6.4 Settings Panel

```
┌──────────────────────────────────────┐
│  Settings                    [Close] │
│  ───────────────────────────────     │
│                                      │
│  GENERAL                             │
│  ─────────────────────────────       │
│  Start at login          [switch]    │
│  Show in menu bar        [switch]    │
│                                      │
│  SECURITY                            │
│  ─────────────────────────────       │
│  Auto-lock after      [8 hours ▾]   │
│  Lock on sleep           [switch]    │
│  Clear clipboard after [30 sec ▾]   │
│  Touch ID                [switch]    │
│                                      │
│  APPEARANCE                          │
│  ─────────────────────────────       │
│  Theme                [Dark ▾]      │
│                                      │
│  SHORTCUTS                           │
│  ─────────────────────────────       │
│  Quick Access    [Cmd+Shift+Space]   │
│  Lock            [Cmd+L]             │
│  New Item        [Cmd+N]             │
│                                      │
│  DATA                                │
│  ─────────────────────────────       │
│  Import...                     [→]   │
│  Export...                     [→]   │
│  Database location             [→]   │
│                                      │
│  ABOUT                               │
│  ─────────────────────────────       │
│  Version 0.1.0                       │
│  Open Source (MIT)                   │
│                                      │
└──────────────────────────────────────┘

Section titles: --font-size-xs, --font-weight-medium, --color-text-tertiary, uppercase
Setting items: --font-size-md, height 40px
```

### 6.5 Quick Access Panel (QuickAccessPanel)

```
┌──────────────────────────────────────┐
│  🔍 Quick Access...                  │  <- large search input
├──────────────────────────────────────┤
│  [EntryCard] GitHub       github.com │  <- compact entry cards
│  [EntryCard] GitLab       gitlab.com │
│  [EntryCard] Gitea        gitea.io   │
│                                      │
│  ↑↓ Navigate   Enter Copy   Esc Close│  <- keyboard hints
└──────────────────────────────────────┘

Width: 680px
Max height: 60vh
Position: centered, 20% from top
Border radius: --radius-xl
Background: --color-bg-elevated
Shadow: --shadow-lg
Border: 1px solid var(--color-border)

Animation: scale(0.95) + opacity(0) → scale(1) + opacity(1)
Duration: --duration-slow, --ease-out

Search input: --font-size-lg, no border, full width, auto-focus
Results: compact EntryCards (40px height), max 8 visible

Initial state (no query):
- Show "Recently Used" section header (--font-size-xs, --color-text-tertiary)
- List last 5 used entries, ordered by last access time
- First entry pre-selected

Search state:
- Results sorted by: exact match > recent use > frequency > title match > URL match
- Matching text highlighted with --color-primary (bold, not background)
- Max 8 visible results, scroll for more

Keyboard:
- ↑/↓: select entry
- Enter: copy password of selected entry + close
- Shift+Enter: copy username
- Cmd+Enter: open URL in browser
- Cmd+O: open entry in main window
- Escape: close
- Type: filter results
```

---

## 7. Code Constraints (AI Must Follow)

### 7.1 Must Do

- [ ] All colors, spacing, font sizes, radii, shadows, durations use token variables
- [ ] Every interactive element has hover/active/disabled states
- [ ] Every clickable element meets 32px minimum target (desktop is 32px, not 44px)
- [ ] Keyboard shortcuts are implemented for all major actions
- [ ] Focus indicators visible on `:focus-visible`
- [ ] Password fields are masked by default
- [ ] Sensitive data never appears in console logs, error messages, or DOM attributes
- [ ] All panels handle: loading / data / empty / error states
- [ ] Text overflow: ellipsis for single-line, scrollable for multi-line
- [ ] Use semantic HTML: `<button>`, `<input>`, `<label>`, `<nav>`, `<main>`, `<aside>`
- [ ] `prefers-reduced-motion` respected

### 7.2 Must Not

- [ ] No hardcoded color/spacing/font values
- [ ] No inline styles (`style=""`)
- [ ] No `<div>` with click handler instead of `<button>`
- [ ] No missing loading/error/empty states
- [ ] No password values in `title`, `alt`, `data-*` attributes, or `console.log`
- [ ] No focus indicator removal without `:focus-visible` alternative
- [ ] No critical actions (delete, overwrite) without confirmation Modal

### 7.3 Naming Conventions

| Category | Convention | Example |
|----------|-----------|---------|
| CSS variable | `--{category}-{property}-{variant}` | `--color-text-primary` |
| CSS class | Tailwind utility classes for layout/spacing; BEM for custom component internals | `.entry-card__title--selected` |
| React component | PascalCase | `EntryCard`, `PasswordField` |
| Hook | camelCase with `use` prefix | `useVault`, `useClipboard` |
| Tauri command | snake_case | `create_entry`, `unlock` |
| Store | camelCase with `Store` suffix | `vaultStore`, `settingsStore` |

---

## 8. Design Checklist

Run before finalizing any UI code:

### Token Compliance
- [ ] All colors use `--color-*`
- [ ] All spacing use `--spacing-*`
- [ ] All font sizes use `--font-size-*`
- [ ] All radii use `--radius-*`
- [ ] All shadows use `--shadow-*`
- [ ] All durations use `--duration-*`

### Dark Mode
- [ ] Verified in both dark and light themes
- [ ] No invisible text or harsh contrast issues
- [ ] Icons follow `currentColor`

### Accessibility
- [ ] Interactive targets >= 32px
- [ ] Color contrast >= 4.5:1 body / 3:1 large text (WCAG AA)
- [ ] Focus indicators on all interactive elements
- [ ] Labels associated with inputs
- [ ] Modals trap focus and respond to Escape

### Security
- [ ] Password fields masked by default
- [ ] No sensitive data in DOM attributes, console, or error messages
- [ ] Auto-mask after 30s timeout
- [ ] Clipboard clear timer active after copy

### Interaction
- [ ] All states covered: default / loading / success / error / empty / disabled
- [ ] Keyboard shortcuts match spec
- [ ] Form validation on blur + submit
- [ ] Error recovery preserves user data
- [ ] No double-submit on buttons
