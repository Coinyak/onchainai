---
name: onchainai-ui-workflow
description: Use when changing or reviewing OnchainAI UI, Leptos components, Tailwind/CSS, responsive behavior, visual polish, accessibility, screenshots, browser smoke checks, or any public page layout in /src/pages, /src/components, /style, DESIGN.md, or docs/UI_UX_DESIGN.md.
---

# OnchainAI UI Workflow

Use this repo-specific workflow for UI work before reaching for generic frontend advice. OnchainAI already has strong design and deploy rules; the job is to preserve them while making small, visible improvements.

## Source Priority

Read these first for any UI change:

1. `AGENTS.md`
2. `DESIGN.md`
3. `docs/UI_UX_DESIGN.md`
4. `docs/BUILD_DEPLOY_RULES.md`

Then inspect the affected Rust/Leptos component, page, CSS, and script files. Do not infer UI intent from screenshots alone when a design doc covers the surface.

## Design Invariants

- UI text is English.
- No emoji in UI text.
- Use lucide-style SVG line icons or existing icon/logo patterns.
- Keep the calm light theme: neutral surfaces, orange only for primary interaction/focus, no crypto-gradient styling.
- Preserve information density without cramped mobile layouts.
- Body text on mobile must stay readable; touch targets should be at least 44px.
- Cards stay at 8px radius or less unless the existing design document says otherwise.
- Do not introduce x402 payment execution, custody, wallet, facilitator, or fund-moving UI. x402 belongs to metadata, attribution, and trust signals only.

## Workflow

### 1. Frame The Change

Write down:

- Target route or component.
- User task it supports.
- Viewports that matter, usually `1280x900` and `375x812`.
- Whether this is a layout, responsive, accessibility, component-boundary, or visual-polish issue.

If the issue is vague, use `web-design-guidelines` or `visual-qa` after capturing screenshots.

### 2. Choose The Narrow Skill

- Broad UI critique or launch readiness: `web-design-guidelines`
- Mobile, tablet, overflow, wrapping, zoom/reflow: `responsive-design`
- Keyboard, focus, labels, contrast, announcements: `web-accessibility`
- Repeated cards, buttons, filters, modals, slots: `ui-component-patterns`
- Token math, contrast, palette/spacing scale: `design-tokens`
- Tailwind/CSS spacing and utility audits: `tailwind`
- Rendered screenshot review: `visual-qa`
- Browser automation or visual regression: `playwright-best-practices`

Use the smallest matching skill. Do not stack three UI skills unless the problem actually crosses those boundaries.

### 3. Edit Small

Prefer one UI surface at a time:

- Component/markup change.
- CSS/Tailwind change.
- Server/data change.

Avoid mixing all three unless required. Preserve existing component names and shell/sidebar patterns.

### 4. Inspect The Rendered UI

Code review is not enough. Start or reuse a local server, then inspect:

```bash
./scripts/smoke-test.sh http://localhost:3000
node scripts/browser-smoke.mjs http://localhost:3000
node scripts/visual-snapshots.mjs http://localhost:3000 --out .playwright-cli/ui-snapshots
```

For final release checks, follow `docs/BUILD_DEPLOY_RULES.md`: build once with `cargo leptos build --release`, verify bundle coherence, restart the matching binary, then run curl and browser smoke.

### 5. Visual QA Gate

Before calling UI work done, inspect desktop and mobile screenshots against:

- `DESIGN.md` colors, type, radius, spacing, and orange usage.
- `docs/UI_UX_DESIGN.md` route-level layout expectations.
- No horizontal scroll, overlap, clipped text, unreadable tiny type, missing focus state, or dead-end interaction.

If screenshots reveal problems, fix them before expanding scope.

## Verification Matrix

Use the smallest set that matches the risk:

| Change | Minimum verification |
|---|---|
| Text or tiny style change | `cargo fmt --check`, targeted visual screenshot |
| Leptos component/layout | `cargo build --features ssr`, desktop/mobile screenshots |
| Public route behavior | `./scripts/smoke-test.sh`, `node scripts/browser-smoke.mjs` |
| Release/deploy-facing UI | `./scripts/release-build.sh`, `./scripts/verify-bundle.sh`, restart, smoke, browser smoke |

If Playwright is missing, say so and include the install hint from `scripts/post-deploy-verify.sh`. Do not claim browser QA passed without running it.

## Output

When reporting UI work, include:

- Files changed.
- Routes and viewports inspected.
- Screenshots or snapshot paths when captured.
- Verification commands run and their result.
- Any remaining visual risk.
