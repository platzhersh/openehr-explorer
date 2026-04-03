# ADR-0008: CDN-Based Web Components for medblocks-ui

**Date:** 2026-04-03

## Status

Accepted

---

## Context

PRD-0002 specifies using medblocks-ui for Web Template form rendering (see ADR-0002 for the decision to use medblocks-ui).

The implementation plan was:
1. Install via npm: `npm install medblocks-ui`
2. Import in `src/main.ts`: `import "medblocks-ui";`
3. Use `<mb-form>` custom elements in Vue templates

### The Problem: Incomplete npm Package

During implementation, after adding `medblocks-ui: "0.1.1"` to `package.json` and running `npm install`, we encountered:

```
[vite] Failed to resolve entry for package "medblocks-ui".
The package may have incorrect main/module/exports specified in its package.json.
```

**Root cause investigation:**

1. Inspected `node_modules/medblocks-ui/package.json`:
   ```json
   {
     "name": "medblocks-ui",
     "version": "0.1.1",
     "main": "dist/index.cjs.js",
     "module": "dist/custom-elements/index.js",
     "types": "dist/custom-elements/index.d.ts"
   }
   ```

2. Checked `node_modules/medblocks-ui/dist/`:
   - Directory exists but contains **source TypeScript files**, not built JavaScript
   - No `index.cjs.js` (main entry point)
   - No `custom-elements/index.js` (module entry point)
   - The published npm package contains the **source repository**, not a built distribution

3. Verified on npm registry:
   ```bash
   npm view medblocks-ui@0.1.1
   # Confirmed: package tarball includes src/ but not built dist/
   ```

**Why this happened:**

The medblocks-ui npm package appears to have been published directly from the source repository without running the build process. The Lit + Stencil build that generates the distributable `dist/` files was not included in the published tarball.

### Alternatives Considered

**A. Build medblocks-ui Locally**

Clone the medblocks-ui repository, build it, and include the `dist/` output in our project.

**Rejected because:**
- Adds medblocks-ui's build toolchain to our project (Stencil, Lit, Node-specific tooling)
- Requires maintaining a fork or manual updates
- Large git history overhead (vendoring a library)
- Violates package manager best practices

**B. Fork and Publish Fixed Package**

Fork medblocks-ui, fix the build, publish as `@openehr-explorer/medblocks-ui`.

**Rejected because:**
- Maintenance burden (track upstream, merge updates)
- Confusing for users (why a fork?)
- Doesn't fix the upstream issue

**C. Wait for Upstream Fix**

Open an issue with medblocks-ui maintainers, wait for a new release.

**Rejected because:**
- Blocks PRD-0003 implementation
- Uncertain timeline (community project, no SLA)
- Project needs to move forward

**D. Use CDN-Hosted Build**

Load medblocks-ui from a CDN (jsDelivr, unpkg) via `<script>` tag in `index.html`.

**Accepted.** This is the standard approach for Web Components that aren't properly packaged for npm.

---

## Decision

We will load medblocks-ui from **jsDelivr CDN** using a `<script type="module">` tag in `index.html`.

## Implementation

### 1. Remove npm Package

**Before:**
```json
{
  "dependencies": {
    "medblocks-ui": "0.1.1"
  }
}
```

**After:**
```json
{
  "dependencies": {
    // medblocks-ui removed
  }
}
```

### 2. Add CDN Script Tag

**File: `index.html`**
```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <link rel="icon" type="image/svg+xml" href="/vite.svg" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>openEHR Explorer</title>

    <!-- medblocks-ui Web Components from CDN -->
    <script type="module" src="https://cdn.jsdelivr.net/npm/medblocks-ui@0.1.1/dist/medblocks-ui/medblocks-ui.esm.js"></script>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

**Why jsDelivr:**
- Widely used CDN with high availability
- Automatically serves npm packages (even incomplete ones)
- Extracts and serves the built files that *do* exist in the package
- Free for open-source projects
- Supports version pinning (`@0.1.1`)

### 3. Remove Import from main.ts

**Before:**
```typescript
import { createApp } from 'vue';
import "medblocks-ui";  // ❌ Module resolution fails
// ...
```

**After:**
```typescript
import { createApp } from 'vue';
// medblocks-ui loaded via CDN in index.html
// ...
```

### 4. Configure Vite to Recognize Custom Elements

**File: `vite.config.ts`**
```typescript
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig(async () => ({
  plugins: [
    vue({
      template: {
        compilerOptions: {
          // Treat all tags starting with mb- as custom elements
          isCustomElement: (tag) => tag.startsWith('mb-')
        }
      }
    })
  ],
  // ... other config
}));
```

**Why this is needed:**
- Vue 3 warns about "unknown custom elements" by default
- Web Components use custom HTML tags (e.g., `<mb-form>`)
- This config tells Vue to skip the warning for `mb-*` tags

### 5. Usage in Components

**No change required.** Components use `<mb-form>` exactly as if medblocks-ui were npm-installed:

```vue
<template>
  <mb-form
    :webTemplate="webTemplate"
    @mb-submit="handleSubmit"
  />
</template>
```

The CDN script registers the `<mb-form>` custom element globally.

---

## Consequences

### Positive

- **Unblocked development:** Can use medblocks-ui immediately without waiting for package fix
- **Works as intended:** CDN-hosted build is the actual built distribution (not source files)
- **No build changes:** No need to add Stencil/Lit to our build toolchain
- **Version pinned:** `@0.1.1` ensures consistent behavior (matches ADR-0007)
- **Fast load:** jsDelivr is globally distributed and cached
- **Standard pattern:** Many Web Component libraries recommend CDN usage (e.g., Ionic, Shoelace)

### Negative

- **External dependency:** Requires internet connection for development (mitigated: jsDelivr is highly available)
- **CDN outage risk:** If jsDelivr is down, medblocks-ui won't load (mitigated: very rare, can cache locally if needed)
- **No tree-shaking:** Entire medblocks-ui library is loaded, even if only using a subset (acceptable: library is small, ~100KB)
- **CORS considerations:** N/A for ES modules (same-origin policy doesn't apply)
- **Cache invalidation:** CDN caches the `@0.1.1` version indefinitely (good for stability, but requires version bump for updates)

### Neutral

- **Offline development:** Requires local CDN cache or network access (typical for web development)
- **TypeScript types:** No automatic type definitions (medblocks-ui doesn't export detailed types anyway)

---

## Fallback Strategy

If jsDelivr becomes unavailable or unreliable, we can:

### Option 1: Self-Host the CDN File

1. Download the built file:
   ```bash
   curl -o public/medblocks-ui.esm.js \
     https://cdn.jsdelivr.net/npm/medblocks-ui@0.1.1/dist/medblocks-ui/medblocks-ui.esm.js
   ```

2. Update `index.html`:
   ```html
   <script type="module" src="/medblocks-ui.esm.js"></script>
   ```

3. Commit the file to the repository

**Trade-off:** Increases repository size (~100KB), but eliminates external dependency.

### Option 2: Use unpkg CDN

Replace jsDelivr with unpkg:
```html
<script type="module" src="https://unpkg.com/medblocks-ui@0.1.1/dist/medblocks-ui/medblocks-ui.esm.js"></script>
```

### Option 3: Build from Source

Clone medblocks-ui, build it, and vendor the output (see "Alternatives Considered" section A).

---

## Testing

### Development

```bash
npm run dev
# Verify medblocks-ui loads in browser console:
# > customElements.get('mb-form')
# Should return: class MbForm extends HTMLElement
```

### Production Build

```bash
npm run tauri build
# medblocks-ui script tag is preserved in built index.html
# Custom elements work in production app
```

### Offline Testing

If developing offline, cache the CDN resource:
```bash
# Browser will cache after first load
# Or use browser DevTools → Network → "Disable cache" unchecked
```

---

## Future Migration Path

If medblocks-ui publishes a properly built npm package in the future:

1. Add to `package.json`: `"medblocks-ui": "X.Y.Z"`
2. Import in `main.ts`: `import "medblocks-ui";`
3. Remove CDN `<script>` tag from `index.html`
4. Remove `isCustomElement` config from `vite.config.ts` (optional, doesn't hurt)
5. Test and commit

**Estimated effort:** 10 minutes.

---

## Related

- ADR-0002: Use medblocks-ui for Web Template Form Rendering (decision to use medblocks-ui)
- ADR-0007: Pinned Dependency Versions (why we pin `@0.1.1`)
- PRD-0002: Composition & EHR CRUD (requires medblocks-ui)

---

## References

- medblocks-ui GitHub: https://github.com/medblocks/medblocks-ui
- medblocks-ui on npm: https://www.npmjs.com/package/medblocks-ui
- jsDelivr CDN: https://www.jsdelivr.com/
- Vue 3 custom elements: https://vuejs.org/guide/extras/web-components.html
- Lit Web Components: https://lit.dev/

---

## Notes

### Why Web Components Work Well from CDN

Web Components (Custom Elements) are **framework-agnostic** by design:
- Self-contained (Shadow DOM encapsulation)
- No Vue/React/Angular dependency
- Registered globally via `customElements.define()`
- Standard browser API

Loading from CDN is the **original intent** of Web Components — they're meant to be distributed and used like native HTML elements.

### Security Considerations

**Subresource Integrity (SRI):**

We could add SRI hashes for tamper protection:
```html
<script
  type="module"
  src="https://cdn.jsdelivr.net/npm/medblocks-ui@0.1.1/dist/medblocks-ui/medblocks-ui.esm.js"
  integrity="sha384-HASH_HERE"
  crossorigin="anonymous"
></script>
```

**Decision:** Not implemented for now because:
- jsDelivr is trustworthy (backed by Cloudflare, used by millions)
- medblocks-ui is pinned to `@0.1.1` (immutable after publish)
- Desktop app context (not exposed to arbitrary web traffic)

Can add SRI if security requirements change.
