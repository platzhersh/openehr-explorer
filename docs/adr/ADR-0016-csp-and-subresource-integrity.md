# ADR-0016: Content Security Policy and Subresource Integrity

**Date:** 2026-04-11
**Status:** Accepted
**Related:** ADR-0014 (XSS Hardening and Input Validation)

---

## Context

A pre-publication security audit identified two infrastructure-level gaps that enable or amplify XSS attacks in the Tauri webview:

### No Content Security Policy

The Tauri configuration had CSP disabled (`"csp": null` in `tauri.conf.json`). Without CSP, any successful XSS injection — whether from a malicious openEHR server response rendered via `v-html` (see ADR-0014) or a compromised CDN script — has unrestricted access to:

- Tauri IPC commands (file reads, credential access, HTTP requests)
- Arbitrary network connections (data exfiltration)
- Inline script execution and `eval()`

CSP is the primary defence-in-depth layer that limits the blast radius of XSS even when other mitigations fail.

### CDN scripts without Subresource Integrity

`index.html` loads two external resources from public CDNs:

```html
<script src="https://unpkg.com/medblocks-ui@0.0.217/dist/bundle.js"></script>
<link href="https://cdn.jsdelivr.net/npm/@shoelace-style/shoelace@2.0.0-beta.71/dist/themes/light.min.css" rel="stylesheet"/>
```

Without `integrity` attributes, a CDN compromise, DNS hijack, or man-in-the-middle attack could replace these resources with malicious code. Since `medblocks-ui` is loaded as a `<script>` tag, a compromised version would have full JavaScript execution in the Tauri webview context.

---

## Decision

### 1. Enable a restrictive Content Security Policy

Set CSP in `src-tauri/tauri.conf.json` to:

```
default-src 'self';
script-src 'self' 'unsafe-inline' https://unpkg.com;
style-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net;
connect-src ipc: http://ipc.localhost https: http:;
img-src 'self' asset: http://asset.localhost data: blob:;
font-src 'self' data: https://cdn.jsdelivr.net https://unpkg.com
```

**Directive rationale:**

| Directive | Value | Why |
|---|---|---|
| `default-src` | `'self'` | Baseline: only allow resources from the app origin |
| `script-src` | `'self' 'unsafe-inline' https://unpkg.com` | App scripts + medblocks-ui CDN. `unsafe-inline` needed for web component compatibility — should be removed when medblocks-ui is bundled locally (see future work). |
| `style-src` | `'self' 'unsafe-inline' https://cdn.jsdelivr.net` | App styles + Shoelace CDN. `unsafe-inline` needed for Vue scoped styles and web component shadow DOM styles. |
| `connect-src` | `ipc: http://ipc.localhost https: http:` | Tauri IPC + connections to user-configured openEHR servers (arbitrary hosts by design). |
| `img-src` | `'self' asset: http://asset.localhost data: blob:` | App images + Tauri asset protocol + inline data URIs. |
| `font-src` | `'self' data: https://cdn.jsdelivr.net https://unpkg.com` | App fonts + CDN-hosted fonts + base64 data URI fonts. |

**Note:** `'unsafe-inline'` in `script-src` weakens XSS protection. This is a pragmatic trade-off for medblocks-ui web component compatibility. ADR-0014's recommendation to replace `v-html` with a proper syntax highlighter remains the primary XSS mitigation. The long-term goal is to bundle medblocks-ui locally and remove `'unsafe-inline'`.

### 2. Add Subresource Integrity hashes to CDN resources

Add `integrity` and `crossorigin="anonymous"` attributes to all CDN tags in `index.html`:

```html
<script
  src="https://unpkg.com/medblocks-ui@0.0.217/dist/bundle.js"
  integrity="sha384-<hash>"
  crossorigin="anonymous"
></script>
<link
  href="https://cdn.jsdelivr.net/npm/@shoelace-style/shoelace@2.0.0-beta.71/dist/themes/light.min.css"
  rel="stylesheet"
  integrity="sha384-<hash>"
  crossorigin="anonymous"
/>
```

Generate hashes with:
```bash
curl -sf <URL> | openssl dgst -sha384 -binary | openssl base64 -A
# Then prefix with "sha384-"
```

The `crossorigin="anonymous"` attribute has been added as a prerequisite. The actual `integrity` hash values must be generated with network access and added before publication.

---

## Status of Implementation

| Item | Status |
|---|---|
| CSP policy in `tauri.conf.json` | Done (this commit) |
| `crossorigin="anonymous"` on CDN tags | Done (this commit) |
| SRI `integrity` hashes on CDN tags | Pending — requires network access to compute hashes |

---

## Consequences

### Positive
- CSP blocks exfiltration via `connect-src` to unauthorized origins (partially — `https:` and `http:` are allowed for openEHR server connectivity)
- CSP blocks `eval()`, `new Function()`, and other dynamic code generation
- SRI ensures CDN resources haven't been tampered with
- Defence-in-depth: even if ADR-0014's `v-html` fixes are incomplete, CSP limits what injected code can do

### Negative
- `'unsafe-inline'` in `script-src` still permits inline script injection — this is the main gap until medblocks-ui is bundled locally
- Broad `connect-src` (allowing `https:` and `http:`) is necessary for the app's core functionality but means CSP cannot prevent data exfiltration to attacker-controlled servers
- SRI hashes must be regenerated whenever CDN dependency versions are bumped
- CSP violations may cause subtle breakage in web components — needs testing

### Future work
- **Bundle medblocks-ui and Shoelace locally** via npm to eliminate CDN dependency entirely, remove `'unsafe-inline'` from `script-src`, and remove external domain allowlists
- **Add CSP violation reporting** via `report-uri` or `report-to` directive to catch violations during development
- **Tighten `connect-src`** if a future proxy architecture routes all openEHR traffic through a single backend endpoint
