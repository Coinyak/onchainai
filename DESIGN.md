---
version: alpha
name: OnchainAI
description: Crypto tool directory — neutral light with orange accent, mobile-first readability
colors:
  primary: "#1A1A1A"
  secondary: "#6B6B6B"
  tertiary: "#E76F00"
  neutral-bg: "#FFFFFF"
  neutral-surface: "#F5F5F0"
  neutral-hover: "#FAFAFA"
  border: "#E5E5E5"
  border-strong: "#D1D1D1"
  text-muted: "#999999"
  error: #C0392B
  success: #2D7D46
  on-tertiary: "#FFFFFF"
typography:
  h1:
    fontFamily: Inter
    fontSize: 28px
    fontWeight: 700
    lineHeight: 1.2
    letterSpacing: -0.02em
  h2:
    fontFamily: Inter
    fontSize: 20px
    fontWeight: 600
    lineHeight: 1.3
    letterSpacing: -0.01em
  h3:
    fontFamily: Inter
    fontSize: 16px
    fontWeight: 600
    lineHeight: 1.4
  body-md:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: 400
    lineHeight: 1.6
  body-sm:
    fontFamily: Inter
    fontSize: 13px
    fontWeight: 400
    lineHeight: 1.5
    color: "#6B6B6B"
  label-caps:
    fontFamily: Inter
    fontSize: 11px
    fontWeight: 600
    lineHeight: 1
    letterSpacing: 0.06em
  code:
    fontFamily: "JetBrains Mono"
    fontSize: 13px
    fontWeight: 400
    lineHeight: 1.5
  mobile-body:
    fontFamily: Inter
    fontSize: 16px
    fontWeight: 400
    lineHeight: 1.65
rounded:
  sm: 6px
  md: 8px
  lg: 12px
  full: 9999px
spacing:
  xs: 4px
  sm: 8px
  md: 16px
  lg: 24px
  xl: 32px
  2xl: 48px
  gutter: 16px
  margin: 24px
components:
  button-primary:
    backgroundColor: "{colors.tertiary}"
    textColor: "{colors.on-tertiary}"
    rounded: "{rounded.md}"
    padding: 12px
  button-primary-hover:
    backgroundColor: "#D96400"
  button-secondary:
    backgroundColor: "{colors.neutral-bg}"
    textColor: "{colors.primary}"
    borderColor: "{colors.border-strong}"
    borderWidth: 1px
    rounded: "{rounded.md}"
    padding: 12px
  button-secondary-hover:
    backgroundColor: "{colors.neutral-surface}"
  card:
    backgroundColor: "{colors.neutral-bg}"
    borderColor: "{colors.border}"
    borderWidth: 1px
    rounded: "{rounded.md}"
    padding: "{spacing.lg}"
  card-hover:
    borderColor: "{colors.border-strong}"
  card-selected:
    backgroundColor: "{colors.neutral-surface}"
    borderColor: "{colors.border-strong}"
  badge-verified:
    backgroundColor: "{colors.neutral-surface}"
    borderColor: "{colors.primary}"
    borderWidth: 1px
    textColor: "{colors.primary}"
    rounded: "{rounded.sm}"
    padding: "2px 8px"
  badge-neutral:
    backgroundColor: "{colors.neutral-hover}"
    borderColor: "{colors.border-strong}"
    borderWidth: 1px
    textColor: "{colors.primary}"
    rounded: "{rounded.sm}"
    padding: "2px 8px"
  badge-x402:
    backgroundColor: "{colors.primary}"
    borderColor: "{colors.primary}"
    borderWidth: 1px
    textColor: "{colors.neutral-bg}"
    rounded: "{rounded.sm}"
    padding: "2px 8px"
  badge-tertiary:
    backgroundColor: "{colors.tertiary}"
    borderColor: "{colors.tertiary}"
    borderWidth: 1px
    textColor: "{colors.on-tertiary}"
    rounded: "{rounded.sm}"
    padding: "2px 8px"
  input:
    backgroundColor: "{colors.neutral-bg}"
    borderColor: "{colors.border}"
    borderWidth: 1px
    textColor: "{colors.primary}"
    rounded: "{rounded.md}"
    padding: "12px 16px"
  input-focus:
    borderColor: "{colors.tertiary}"
  code-block:
    backgroundColor: "{colors.neutral-surface}"
    borderColor: "{colors.border}"
    borderWidth: 1px
    textColor: "{colors.primary}"
    rounded: "{rounded.sm}"
    padding: "12px 16px"
  sidebar-section-header:
    typography: "{typography.label-caps}"
    textColor: "{colors.primary}"
---

# DESIGN.md — OnchainAI

## Overview

A clean, information-dense directory for crypto tools (MCP, CLI, SDK, API, x402, RWA, AI agents). Light mode only, neutral palette (white / gray / beige) with a single orange accent for interaction. Mobile-first readability — large touch targets, generous line height, no cramped layouts. Designed for developers and agents who scan fast and decide faster.

**Brand personality**: Technical, trustworthy, calm. Not flashy. Not "crypto bro" gradients. Closer to Stripe / Linear than to Binance / DexScreener.

## Colors

The palette is three neutrals plus one accent. No gradients. No multi-hue category colors.

- **Primary (#1A1A1A):** Near-black ink for headlines, body text, primary buttons text.
- **Secondary (#6B6B6B):** Slate gray for descriptions, metadata, timestamps, secondary text.
- **Tertiary (#E76F00):** Orange — the sole interaction driver. Used for primary CTAs, active filter indicators, links, focus rings, and the single most important action per screen. Never for decoration.
- **Neutral BG (#FFFFFF):** Page background. Pure white.
- **Neutral Surface (#F5F5F0):** Warm beige for section bands, selected card state, code blocks, badges backgrounds.
- **Neutral Hover (#FAFAFA):** Subtle off-white for hover states on cards and list items.
- **Border (#E5E5E5):** Default 1px borders on cards, dividers, inputs.
- **Border Strong (#D1D1D1):** Hover and selected card borders, badge outlines.
- **Text Muted (#999999):** Placeholder text, disabled states, empty-state icons.
- **Error (#C0392B):** Validation errors, destructive actions. Used sparingly.
- **Success (#2D7D46):** Positive confirmations (verified badge check, copy success). Used sparingly.

**Orange usage rules:**
- Primary CTA buttons only (Submit, Install, Register)
- Focus rings on inputs and search bar
- Active sidebar filter dot indicator (small 4px circle, not full text color)
- Links in body text (underlined, orange on hover)
- Never on: backgrounds, card borders, category icons, badges (except x402 which is black)

## Typography

Single font family: **Inter** (sans-serif) for everything except code. **JetBrains Mono** for install commands and code blocks.

- **Headlines (h1):** Inter 700, 28px, tight tracking. Page titles and hero text only.
- **Section headers (h2):** Inter 600, 20px. "Categories", "Description", "Install", "Chains".
- **Tool names (h3):** Inter 600, 16px. Tool card titles, sidebar section headers.
- **Body (body-md):** Inter 400, 14px, 1.6 line height. Descriptions, comments, card body.
- **Body small (body-sm):** Inter 400, 13px, #6B6B6B. Metadata, timestamps, "3d ago", star counts.
- **Label caps:** Inter 600, 11px, uppercase, 0.06em tracking. Badge text, sidebar section headers, "Categories" label.
- **Code:** JetBrains Mono 400, 13px, 1.5 line height. Install commands, endpoints.
- **Mobile body:** Inter 400, 16px, 1.65 line height. Body text on mobile screens (<768px) — upscaled from 14px for readability.

**Mobile readability rules:**
- All body text ≥ 16px on mobile (never 14px)
- Line height ≥ 1.65 on mobile (vs 1.6 desktop)
- Headlines scale down 2px on mobile (h1: 26px, h2: 18px)
- Minimum touch target: 44x44px (buttons, list items, filter chips)
- Max content width on mobile: 100% - 32px horizontal padding

## Layout

Fluid layout. Desktop: fixed left sidebar (240px or 40px collapsed) + flexible content area. Mobile: single column, sidebar becomes fullscreen overlay.

**Spacing scale:** Strict 4px base. 4 / 8 / 16 / 24 / 32 / 48px.

**Desktop grid:**
- Sidebar: 240px (expanded) / 40px (collapsed)
- Content max-width: 960px (list area)
- Preview panel: 400px (when open, list shrinks)
- Category grid: 5 columns, 16px gap

**Mobile grid:**
- Single column, 32px horizontal margin (16px each side)
- Category grid: 2 columns, 12px gap
- Cards: full width, 16px vertical gap between
- Preview panel: fullscreen overlay (no side panel)
- Sidebar: fullscreen overlay via hamburger menu

**Whitespace philosophy:** Dense but breathable. Cards have 24px internal padding. List items separated by 1px border, not large gaps. Section bands use beige background to create visual separation without adding whitespace.

## Elevation & Depth

Minimal shadows. Depth is conveyed through 1px borders and tonal layering (white cards on beige sections).

- **Default card:** No shadow. 1px #E5E5E5 border.
- **Hover card:** `0 2px 8px rgba(0,0,0,0.06)` — barely visible lift. Border darkens to #D1D1D1.
- **Selected card (preview open):** No shadow. Border #D1D1D1. Background shifts to beige #F5F5F0.
- **Preview panel:** `0 4px 16px rgba(0,0,0,0.08)` on desktop (left edge shadow, panel floats above list). No shadow on mobile (fullscreen).
- **Dropdowns/modals:** `0 8px 32px rgba(0,0,0,0.12)`.

Never use heavy shadows, colored shadows, or glow effects.

## Shapes

Corner radius is minimal and consistent. No pill shapes except badges.

- **Cards:** 8px
- **Buttons:** 8px
- **Inputs:** 8px
- **Code blocks:** 6px
- **Badges:** 6px
- **Sidebar filter chips:** 6px
- **Category grid cards:** 8px
- **Avatar/logo:** 8px (not circular — rounded square)

All interactive elements share the same radius. Never mix 4px and 12px in the same view.

## Components

- **Primary button:** Orange #E76F00 fill, white text, 8px radius, 12px padding. Hover: darker orange #D96400. One per screen maximum.
- **Secondary button:** White fill, 1px #D1D1D1 border, black text. Hover: beige #F5F5F0 fill.
- **Cards:** White background, 1px #E5E5E5 border, 8px radius, 24px padding. Hover: border darkens, subtle shadow. Selected: beige background, stronger border.
- **Badges:** 6px radius, 1px border, 2px 8px padding, uppercase 11px semibold. Three tiers: verified (beige bg, black border), neutral (off-white bg, gray border), x402 (black fill, white text).
- **Inputs:** White fill, 1px #E5E5E5 border, 8px radius, 12px 16px padding. Focus: border becomes orange #E76F00, no glow.
- **Code blocks:** Beige #F5F5F0 background, 1px #E5E5E5 border, 6px radius, 12px 16px padding. JetBrains Mono 13px.
- **Search bar:** Full width on desktop (max 640px centered), 48px height, 8px radius, 1px border, focus border orange. On mobile: fullscreen overlay from search icon.
- **Sidebar:** 240px width, no border (separated by 1px right divider). Section headers: uppercase 11px. Filter items: 14px, 36px height, 8px radius hover. Active filter: orange 4px dot indicator + black text.
- **Copy button:** Inline, icon-only (clipboard SVG), 13px. Click: text changes to "Copied" for 2s, no toast.
- **Preview panel:** 400px width on desktop, slides in from right (200ms ease). On mobile/tablet: bottom sheet — slides up from bottom (250ms ease-out), 60% screen height default, drag to fullscreen, drag down or tap outside to close. Top corners 12px radius. Outer area dimmed (#1A1A1A 30% opacity + blur).

## Do's and Don'ts

- **Do** use orange (#E76F00) only for the single most important action per screen
- **Do** maintain WCAG AA contrast (4.5:1 normal text, 3:1 large text)
- **Do** use 16px minimum body text on mobile
- **Do** use 1px borders for separation, not heavy shadows
- **Do** use beige (#F5F5F0) for section bands to create depth without color
- **Don't** use gradients anywhere
- **Don't** use multiple accent colors — orange is the only non-neutral
- **Don't** use dark mode (light mode only for MVP)
- **Don't** use emojis — use Lucide SVG line icons (#4B4B4B)
- **Don't** mix corner radii in the same view
- **Don't** use colored category icons — all icons are monochrome gray
- **Don't** crowd mobile screens — max 2 columns, 44px touch targets
- **Don't** use shadows for emphasis — use border + tonal layering
- **Don't** animate excessively — only hover lift (200ms) and panel slide (200ms)
