# ADR-0002: Use medblocks-ui for Web Template Form Rendering

**Date:** 2026-04-03

## Status

Accepted

---

## Context

PRD-0002 introduces a **Web Template Form Renderer** — the ability to generate a usable data-entry form from any openEHR Web Template JSON, submit a valid FLAT composition to the connected CDR, and pre-populate that same form when editing an existing composition.

This is the most technically complex feature in PRD-0002. The Web Template tree can be arbitrarily deep, contains ~15 distinct `rmType` leaf types (each with its own input semantics), supports repeatable nodes with dynamic indices, and must produce a correctly structured FLAT JSON payload that EHRBase will accept.

The decision is whether to **build this renderer from scratch** in TypeScript/Vue 3, or to **adopt an existing open-source library** that already solves the problem.

### The custom-build option

A custom renderer means:

- Implementing a recursive depth-first Web Template traverser
- Rendering each `rmType` as the appropriate Vue component (`DV_QUANTITY` → numeric + unit select, `DV_CODED_TEXT` → constrained select, `DV_DATE_TIME` → datetime picker, etc.)
- Assembling the FLAT key-value output with correct path and suffix conventions (`|magnitude`, `|unit`, `|value`, `|code`, `|terminology`)
- Handling repeatable node groups with `:N` index management
- Validating required fields (nodes where Web Template `min >= 1`)
- Writing tests for all of the above

This is a significant body of work — estimated 3–4 weeks to reach parity with real-world templates, with ongoing maintenance as edge cases surface. It also duplicates work the openEHR community has already done.

### medblocks-ui

[medblocks-ui](https://github.com/medblocks/medblocks-ui) is an open-source Web Components library from Medblocks (Sidhant Panda et al.) that does exactly this. It:

- Accepts a Web Template JSON object and renders a complete, interactive form
- Handles all common `rmType` values natively
- Emits a standard FLAT JSON object via an `mb-submit` event
- Accepts a `value` prop containing an existing FLAT object to pre-populate the form (used for the Edit flow)
- Is built on Lit (Web Components standard), making it framework-agnostic — works in Vue 3 as a custom element with zero wrapper code
- Is actively maintained (last commit within weeks as of April 2026)
- Is Apache 2.0 licensed
- Is used in production by Medblocks EHR and referenced in the openEHR community

The openEHR Explorer already uses Vue 3 + TypeScript + Tauri. Lit-based Web Components integrate natively — `import 'medblocks-ui'` registers the custom elements globally, and `<mb-form>` works as a standard HTML element in Vue templates.

---

## Decision

We will use **medblocks-ui** for Web Template form rendering.

The `<mb-form>` component is the primary integration point. It receives the Web Template JSON as a prop and emits the FLAT payload on form submit. Context fields (`ctx/language`, `ctx/territory`, `ctx/composer_name`, `ctx/time`) are rendered as plain Vue inputs above the medblocks-ui form and merged into the FLAT payload before CDR submission.

Styling is applied via CSS custom properties exposed by medblocks-ui's theming API, maintained in a single `src/styles/medblocks-overrides.css` file. We do not pierce Shadow DOM boundaries. If a particular element cannot be themed via the public API, we accept the default style for that element rather than using `::part()` or `!important` hacks that would break on library updates.

We do **not** build a custom form renderer for the types medblocks-ui covers. If medblocks-ui does not support a specific `rmType` (e.g. `DV_MULTIMEDIA`, `DV_PARSABLE`), we display a read-only JSON field for that node with a note that editing is not yet supported.

---

## Consequences

### Positive

- **Scope reduction.** Eliminates ~3–4 weeks of form renderer development. The team can focus on CDR integration, UX, and the EHR management features rather than reimplementing openEHR FLAT serialisation.
- **Correctness.** medblocks-ui is tested against real EHRBase instances in production environments. Its FLAT output is known-good for EHRBase. A custom renderer would need extensive testing to reach the same confidence level.
- **Pre-population support.** medblocks-ui's `value` prop makes the Edit flow (Feature 2.1 in PRD-0002) straightforward — pass in the FLAT object from `GET /composition` and the form self-populates.
- **Community alignment.** medblocks-ui is a recognised tool in the openEHR ecosystem. Using it strengthens the Explorer's position as an ecosystem-friendly project and makes it easier for contributors familiar with medblocks-ui to contribute.
- **Repeat node handling.** medblocks-ui handles repeatable nodes (`:N` indices) natively, which is one of the more complex parts of custom implementation.

### Negative

- **Styling constraints.** Web Components use Shadow DOM, which limits CSS styling to what medblocks-ui exposes via CSS custom properties. Some visual inconsistencies between the medblocks-ui form and the rest of the Explorer's design system may be unavoidable without invasive workarounds.
- **External dependency.** The form rendering capability now depends on medblocks-ui's release cadence and maintenance. A breaking change in medblocks-ui requires a coordinated update. Mitigation: pin to a specific minor version and review changelogs before upgrading.
- **Limited rmType coverage.** medblocks-ui targets the most common clinical types. Unusual types (`DV_PARSABLE`, `DV_MULTIMEDIA`, `DV_EHR_URI`, `DV_GENERAL_ORDER_VALUE`) may not be supported. For the Explorer's use case (developer tooling, not full clinical data entry) this is acceptable — we fall back to a read-only JSON display for unsupported types.
- **Better Platform FLAT dialect.** medblocks-ui is primarily tested against EHRBase. Better Platform uses a slightly different FLAT variant (STRUCTURED format option). This is a known risk tracked under PRD-0003; for this PRD, Better CRUD is marked Beta.
- **Bundle size.** Adding medblocks-ui increases the frontend bundle. Tauri apps bundle the frontend locally, so network payload is not a concern, but startup time should be profiled. If startup impact is measurable, we can lazy-load the medblocks-ui import.
- **Framework mismatch friction.** Lit Web Components in Vue 3 require declaring custom elements to avoid Vue's "unknown component" warnings. This is a one-line config change in `vite.config.ts` but is a non-obvious step for new contributors.

### Neutral

- **No runtime dependency on oehrpy.** medblocks-ui is a JavaScript library; it does not require the Python SDK. The decision is consistent with the Explorer's standalone positioning.
- **Alternative (custom renderer) remains viable as Phase 2.** If medblocks-ui proves insufficient for advanced use cases (e.g. deep terminology integration, custom validation rules), a custom renderer can be introduced for those specific types without replacing medblocks-ui for the common case. This ADR does not preclude a hybrid approach.

---

## Alternatives Considered

### A. Build a custom Vue 3 form renderer from scratch

**Rejected.** The development effort (3–4 weeks minimum) is disproportionate to the benefit for a developer tool where perfect form UX is not required. The correctness risk is also high — FLAT path assembly has many edge cases that medblocks-ui has already solved.

### B. Use medblocks-ui's higher-level `<medblocks-form>` React wrapper

**Not applicable.** The Explorer is Vue 3, not React. medblocks-ui's Lit Web Components work directly; no React wrapper is needed or appropriate.

### C. Embed an iframe pointing to a medblocks-ui demo/hosted form

**Rejected.** An iframe approach would not integrate with the Explorer's FLAT preview panel, draft persistence, or Request/Response transparency features. It would also introduce a network dependency that conflicts with the Explorer's offline-capable design.

### D. Use the openEHR Archetype Designer's form renderer

**Rejected.** The Archetype Designer is a full application, not a library. There is no embeddable form renderer extracted from it.

---

## Related

- PRD-0002: Composition & EHR CRUD (uses this decision)
- PRD-0003: Full Better Platform Support (tracks medblocks-ui + Better FLAT dialect compatibility)
- medblocks-ui GitHub: https://github.com/medblocks/medblocks-ui
- medblocks-ui theming docs: https://docs.medblocks.com/medblocks-ui/theming (verify current URL)
- Lit Web Components in Vue 3: https://vuejs.org/guide/extras/web-components
