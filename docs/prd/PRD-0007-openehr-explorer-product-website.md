# PRD-0007: openEHR Explorer — Product Website (GitHub Pages)

**Version:** 1.1
**Date:** 2026-04-05
**Status:** Draft
**Owner:** openEHR Explorer
**Repo slug:** `openehr-explorer`
**Depends on:** PRD-0001 (Desktop CDR Browser), PRD-0002 (CI/CD)

**Changelog:**
- v1.1: Brand colours updated to match actual app design (navy/teal palette from `src/App.vue`). Brand kit section expanded with full spec. Logo placeholder guidance added.

---

## Executive Summary

Build a public product website for openEHR Explorer, hosted on GitHub Pages under `platzhersh.github.io/openehr-explorer` (or a custom domain). The site is modelled structurally on the oehrpy website (same layout patterns, Inter font, sticky header, two-column docs page, no build step) but uses openEHR Explorer's **own brand identity** — the navy/teal palette from the app itself — rather than copying oehrpy's blue/orange colours.

The website consists of three pages: a **landing page** (`index.html`), a **documentation page** (`docs.html`), and a **brand kit page** (`brand-kit.html`). All pages are pure HTML + CSS, deploying instantly on GitHub Pages with zero configuration.

**Goal in one sentence:** Give openEHR Explorer a polished, on-brand web presence that feels unmistakably like a sibling to the app itself, lowering friction for new developers to discover, understand, and download the tool.

---

## Problem Statement

**Current State:**

- openEHR Explorer has no public-facing website. Discovery happens only through the GitHub repository README.
- The README is developer-focused and long — it does not sell the value proposition at a glance.
- There is no single URL to share in a Discourse post, LinkedIn article, or community demo that immediately communicates what the tool is.
- No installation instructions are presented in a copy-paste-friendly, visually distinct format.
- Screenshots of the app's distinctive teal-on-navy UI are buried or absent.

**Pain Points:**

- Developers landing on the GitHub repo must read a wall of markdown before understanding the value proposition.
- The tool's relationship to the oehrpy / Open CIS ecosystem is invisible without reading deeply.
- The app has a strong visual identity (`#64ffda` teal-on-navy) that is invisible from the outside.

---

## Goals & Success Metrics

### Goals

- Provide a polished landing page communicating the value proposition in under 10 seconds.
- Surface installation instructions in a copy-paste-friendly format without requiring the visitor to leave the page.
- Host full documentation on a linked docs page.
- **Reflect the app's actual brand identity** — navy backgrounds, teal primary, Inter + JetBrains Mono — so the website feels continuous with the desktop app.
- Deploy automatically to GitHub Pages on every push to `main`.

### Success Metrics

- The site loads in < 1 second on a fast connection (no external JS except Google Fonts).
- A visitor can find and copy the macOS install command without scrolling past the hero.
- Within 2 weeks of the Discourse announcement: > 100 unique page views.
- At least one community member links to the site (not the repo) in an openEHR forum post.
- Lighthouse accessibility score ≥ 90.

---

## Brand Identity

> This section is the authoritative brand specification for the openEHR Explorer product. It is derived directly from the app's CSS custom properties in `src/App.vue` and is the source of truth for both the website and all future design artefacts.

### Colour System

The Explorer palette is a **navy/teal developer aesthetic** — dark navy backgrounds with a cyan-teal primary accent. This is distinct from oehrpy's purple/orange and reflects the tool's character as a precise, terminal-inspired developer instrument.

#### Background Scale

| Token | Hex | Use |
|-------|-----|-----|
| `--color-bg` | `#1a1a2e` | Page / app background |
| `--color-bg-secondary` | `#16213e` | Sidebar, secondary panels |
| `--color-bg-tertiary` | `#0f3460` | Nav icon backgrounds, deep accents |

#### Surface & Border

| Token | Hex | Use |
|-------|-----|-----|
| `--color-surface` | `#1e2a4a` | Cards, panels, inputs |
| `--color-surface-hover` | `#253456` | Hover states |
| `--color-border` | `#2a3a5c` | Dividers, card borders, input borders |

#### Text

| Token | Hex | Use |
|-------|-----|-----|
| `--color-text` | `#e0e0e0` | Primary body text |
| `--color-text-secondary` | `#8892b0` | Subheadings, nav links, muted labels |
| `--color-text-muted` | `#5a6a8a` | Placeholders, timestamps, tertiary info |

#### Primary Accent — Teal

| Token | Hex | Use |
|-------|-----|-----|
| `--color-primary` | `#64ffda` | Active states, copy buttons, logo wordmark, h1 gradient |
| `--color-primary-dim` | `#3d9e85` | Primary buttons (filled), icon backgrounds |

The teal `#64ffda` is the **signature colour** of openEHR Explorer. It appears on primary CTAs, active navigation items, the hero gradient, and any element that signals "this is Explorer."

#### Semantic Colours

| Token | Hex | Use |
|-------|-----|-----|
| `--color-success` | `#6bff8e` | Connection status: green, success badges |
| `--color-warning` | `#ffd93d` | Warning callouts, pending states |
| `--color-error` | `#ff6b6b` | Error messages, destructive button text |

#### Hero Gradient

The hero `<h1>` gradient:

```css
/* Option B — teal to sky-blue to indigo */
background: linear-gradient(135deg, #64ffda 0%, #38bdf8 60%, #818cf8 100%);
```

### Typography

| Element | Font | Weight | Size |
|---------|------|--------|------|
| Body | Inter | 400 | 16 px |
| Nav links | Inter | 400 | 15 px |
| H1 hero | Inter | 600 | 4 rem |
| H2 section | Inter | 600 | 2 rem |
| Feature card title | Inter | 600 | 1.2 rem |
| Code / paths | JetBrains Mono | 400/500 | 14 px |
| Header wordmark | Inter | 600 | 24 px |

Both fonts loaded from Google Fonts.

### Logo

The logo **placeholder** uses the wordmark `openEHR Explorer` in Inter SemiBold with the teal primary colour. A proper SVG logomark is to be added in a later milestone.

### CSS Token Reference (canonical)

```css
:root {
  --bg:               #1a1a2e;
  --bg-secondary:     #16213e;
  --bg-tertiary:      #0f3460;
  --surface:          #1e2a4a;
  --surface-hover:    #253456;
  --border:           #2a3a5c;
  --text:             #e0e0e0;
  --text-secondary:   #8892b0;
  --text-muted:       #5a6a8a;
  --primary:          #64ffda;
  --primary-dim:      #3d9e85;
  --success:          #6bff8e;
  --warning:          #ffd93d;
  --error:            #ff6b6b;
  --code-bg:          #0d1117;
  --font-sans: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  --font-mono: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
  --radius: 6px;
}
```

---

## Site Architecture

```
docs/website/                ← GitHub Pages source root
├── index.html               Landing page
├── docs.html                Documentation page
├── brand-kit.html           Brand kit
└── assets/
    ├── screenshot.webp      App UI screenshot (added at v0.1.0)
    └── brand/
        ├── logo.svg                (placeholder SVG → replace when ready)
        ├── logo-dark.png           (logo on dark bg)
        ├── logo-light.png          (logo on light bg)
        ├── logo-icon.svg           (icon-only variant)
        └── palette.svg             (colour palette reference, auto-generated)
```

No Jekyll, no Hugo, no build step. GitHub Pages serves `/docs/website` from `main` directly.

---

## Page Specifications

### Page 1: Landing Page (`index.html`)

See PRD body for full specification of header, hero, feature cards, quick start, ecosystem banner, and footer sections.

### Page 2: Documentation Page (`docs.html`)

Two-column layout with sidebar navigation and content area. Sidebar with active section tracking via `IntersectionObserver`.

### Page 3: Brand Kit (`brand-kit.html`)

Complete brand specification page with logo previews, colour palette swatches, typography specimens, and usage guidelines.

---

## Implementation Plan

### Milestone 1: Landing Page
- Create `docs/website/` folder, enable GitHub Pages
- Implement `index.html` with all sections
- Apply CSS token system from Brand Identity
- Placeholder logo SVG (hexagon + magnifier motif)

### Milestone 2: Docs Page
- Implement `docs.html` with sidebar, two-column layout, all content sections
- Wire active-section highlighting with `IntersectionObserver`

### Milestone 3: Brand Kit Page
- Implement `brand-kit.html` with all sections
- Generate `docs/website/assets/brand/palette.svg`

### Milestone 4: Screenshot & Polish (v0.1.0 app release)
- Capture real screenshot, replace placeholder

### Milestone 5: Logo Integration (Post-v0.1.0)
- Add final SVG logomark, enable download buttons

---

## Related

- PRD-0001: openEHR Explorer Desktop CDR Browser
- PRD-0002: GitHub Actions CI/CD
- `src/App.vue`: authoritative source for CSS colour tokens
