# ADR-0014: XSS Hardening and Input Validation

**Date:** 2026-04-11
**Status:** Proposed
**Related:** ADR-0011 (Centralized Request Instrumentation)

---

## Context

A pre-publication security audit identified several cross-site scripting (XSS) vectors and missing input validation in the frontend. Because this is a Tauri desktop app, XSS is more severe than in a typical web app: JavaScript running in the webview has access to Tauri IPC commands, which can read files, access stored credentials, and make authenticated HTTP requests to configured openEHR servers.

### XSS via `v-html` and `innerHTML`

Server-controlled data (compositions, templates, AQL results) is rendered as raw HTML in multiple components for syntax highlighting:

| Component | Usage |
|---|---|
| `CompositionViewer.vue` | `v-html="highlightedJson"` (lines 335, 351) |
| `TemplateBrowser.vue` | `v-html` for JSON, XML, and FLAT paths (lines 634, 655, 677) |
| `RequestInspector.vue` | `v-html="highlightXml(...)"` (line 496) |
| `CompositionTree.vue` | `innerHTML: highlighted` in render function (line 282) |
| `TemplateBrowser.vue` | `tempDiv.innerHTML = html` in search highlight (line ~342) |

The `highlightJson()` and `highlightXml()` utility functions apply basic HTML entity escaping (`<`, `>`, `&`) before wrapping tokens in `<span>` elements via regex. However, regex-based escaping is fragile: the replacement patterns themselves can re-introduce HTML if the input contains carefully crafted strings that survive the initial escaping pass but produce injectable HTML after the regex substitutions.

A malicious or compromised openEHR server could return payloads like:
```json
{ "key": "<img src=x onerror='...'>" }
```

While the current escaping catches this trivial case, more sophisticated payloads exploiting the regex replacement order could bypass it.

### Missing URL scheme validation

Server profile base URLs accept any string. While `reqwest` rejects non-HTTP schemes, there is no frontend validation to warn users about:
- Plain `http://` URLs for non-localhost servers (credentials sent unencrypted)
- Malformed URLs that would fail at runtime

### CDN scripts without Subresource Integrity

`index.html` loads `medblocks-ui` from unpkg.com and Shoelace CSS from jsdelivr.net without `integrity` attributes. A CDN compromise or DNS hijack could inject arbitrary JavaScript.

---

## Decision

### 1. Replace regex-based syntax highlighting with a proper library

Replace the hand-rolled `highlightJson()` and `highlightXml()` functions with a well-tested syntax highlighting library. Candidates:

- **highlight.js** — Mature, broad language support, works client-side. Can be configured for just JSON + XML to keep bundle size small.
- **Shiki** — TextMate grammar-based, accurate highlighting, but heavier.

The library handles HTML escaping internally, eliminating the regex bypass risk entirely. All `v-html` usages for syntax highlighting should use the library's output.

### 2. Replace `innerHTML` in search highlighting with safe DOM construction

In `CompositionTree.vue` and `TemplateBrowser.vue`, replace `innerHTML`-based search highlighting with Vue render functions (`h()`) that construct `<mark>` elements programmatically:

```typescript
// Before (unsafe)
const highlighted = text.replace(regex, '<mark class="tree-search-match">$1</mark>');
return h("span", { innerHTML: highlighted });

// After (safe)
const parts = text.split(regex);
const children = parts.map((part, i) =>
  i % 2 === 1 ? h("mark", { class: "tree-search-match" }, part) : part
);
return h("span", children);
```

For `TemplateBrowser.vue`'s `highlightSearchInContent`, use `textContent` instead of `innerHTML` to extract plain text from highlighted HTML.

### 3. Add URL validation to server profile form

In `ServerManager.vue`, validate the base URL on save:

- Must be a parseable URL (`new URL(value)`)
- Scheme must be `http` or `https`
- Display a warning badge when using `http://` for a non-localhost host
- Prevent saving profiles with `javascript:`, `data:`, `file:`, or other dangerous schemes

### 4. Add Subresource Integrity hashes to CDN resources

Add `integrity` and `crossorigin="anonymous"` attributes to the CDN `<script>` and `<link>` tags in `index.html`. Generate hashes with:

```bash
curl -sf <URL> | openssl dgst -sha384 -binary | openssl base64 -A
```

Long-term, consider bundling these dependencies locally (via npm) to eliminate CDN dependency entirely.

---

## Consequences

### Positive
- Eliminates all known XSS vectors from server-controlled data
- URL validation prevents credential leakage over plain HTTP to remote servers
- SRI hashes protect against CDN supply chain attacks
- Defence-in-depth alongside the CSP policy added in the quick-fix round

### Negative
- Adding highlight.js increases the frontend bundle size (~30-50 KB gzipped for JSON + XML grammars)
- URL validation may require users to update existing profiles that use `http://` for remote servers
- SRI hashes must be updated whenever CDN dependency versions change

### Neutral
- The AQL runner intentionally sends user-typed queries to the server — this is by design and not an XSS concern (it's server-side, and the server enforces its own authorization)

---

## Implementation Notes

### Files to modify

| File | Change |
|---|---|
| `package.json` | Add `highlight.js` dependency (pinned version) |
| `src/lib/highlight.ts` | New: thin wrapper configuring highlight.js for JSON + XML |
| `src/views/CompositionViewer.vue` | Replace `highlightJson()` with library call |
| `src/views/TemplateBrowser.vue` | Replace `highlightJson()`, `highlightXml()`, fix `innerHTML` in search |
| `src/components/RequestInspector.vue` | Replace `highlightXml()` with library call |
| `src/components/CompositionTree.vue` | Replace `innerHTML` with `h()` render function |
| `src/views/ServerManager.vue` | Add URL validation on save |
| `index.html` | Add `integrity` attributes to CDN tags |

### Testing

- Verify syntax highlighting renders correctly for JSON compositions, XML OPTs, and Web Templates
- Test with compositions containing HTML-like strings in values (e.g., `<br>`, `<script>`, `onclick=`)
- Verify search highlighting works in template browser and composition tree
- Test URL validation with valid HTTPS, valid HTTP localhost, invalid schemes, and malformed URLs
