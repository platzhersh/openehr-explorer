# ADR-0020: Authenticode Code Signing via SignPath

**Date:** 2026-08-23
**Status:** Proposed
**Repo:** `openehr-explorer`

---

## Context

ADR-0019 added VirusTotal scan submission as a transparency measure for
unsigned Windows releases, but explicitly called out that it "does not
replace code signing" and that EDR/AV products with unsigned-binary
policies would keep blocking regardless of VirusTotal score.

That prediction played out again on the 0.5.0 release:
`openehr-explorer.exe`, SHA-256
`05a520a49f44041e0f4a965d59891a06e60f9a55d394bcb41f41d28bf091f2b0`, was
flagged 1/71 by Trapmine (`Malicious.high.ml.score`) — an ML-based static
classifier, not a signature match against known malware. This mirrors the
SecureAge false positive from the 0.3.0 incident that originally motivated
ADR-0019.

The root cause is structural, not incidental: the NSIS installer is
unsigned, every release is a brand-new SHA-256 with zero reputation, and
statically-linked Rust binaries wrapped in a self-extracting installer with
an embedded HTTP client (`reqwest`) resemble the feature profile these ML
models are trained to flag. Reporting individual false positives to each
vendor that trips on each new release's hash (e.g. `fp@trapmine.com`, the
SecureAge portal used for 0.3.0) doesn't scale — there is no API for it on
either the VirusTotal or vendor side, submission is manual/email-based, and
a hash cleared today says nothing about the next release's hash.

Note: the CI workflow already sets `TAURI_SIGNING_PRIVATE_KEY` /
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` for every platform build. That is
Tauri's own **minisign** signature for its auto-updater protocol — it lets
the app verify an update payload came from this project before installing
it. It has nothing to do with Windows Authenticode, SmartScreen, or AV
trust, and doesn't address any of the above.

---

## Decision

Authenticode-sign the Windows NSIS installer in CI using
[SignPath.io](https://signpath.io/solutions/open-source-community)'s free
code-signing program for open-source projects, via SignPath's official
GitHub Actions integration
([`signpath/github-action-submit-signing-request`](https://github.com/SignPath/github-action-submit-signing-request)).

SignPath was chosen over Azure Trusted Signing (the other option
[OEH-11](https://linear.app/platzh1rsch/issue/OEH-11) considered) because
this project is open source with no signing budget: SignPath's OSS tier is
free, where Azure Trusted Signing runs roughly $10/month. Azure Trusted
Signing remains the fallback if SignPath's OSS enrollment is rejected or
becomes impractical (e.g. approval stalls, policy changes).

This is additive to ADR-0019, not a replacement — the VirusTotal
transparency scan stays in place regardless of signing status.

---

## Implementation

### Manual setup (one-time, outside this repo — required before this activates)

CI cannot create the SignPath account or approve OSS enrollment itself;
these steps need a project admin:

1. Register at [signpath.io](https://signpath.io) and apply for the free
   open-source tier for `platzhersh/openehr-explorer`. Approval is a
   manual review on SignPath's side.
2. Install the SignPath GitHub App on the repository so SignPath can trust
   builds originating from it.
3. In the SignPath dashboard, create a project (slug: `openehr-explorer`,
   matching the `project-slug` used in `ci.yml`) with two signing
   policies: `release-signing` (used for tag-triggered builds) and
   `test-signing` (used for manual `workflow_dispatch` builds) — CI
   selects between them based on `github.ref`.
4. Generate an API token scoped as a submitter on those signing policies;
   add it as the repository secret `SIGNPATH_API_TOKEN`.
5. Add the SignPath organization ID as the repository (or environment)
   **variable** `SIGNPATH_ORGANIZATION_ID` — not a secret, per SignPath's
   own convention, since it isn't sensitive.

### CI wiring (`build-windows` job in `.github/workflows/ci.yml`)

Follows SignPath's documented pattern (build unsigned → upload as a GitHub
Actions artifact → submit for signing → use the signed output) rather than
Tauri's `bundle.windows.signCommand` hook, since SignPath's signing request
is an async, potentially queued operation better suited to a dedicated
Action step than a synchronous shell-out during bundling:

1. `tauri build` runs exactly as before, producing the unsigned NSIS
   installer (and, if `TAURI_SIGNING_PRIVATE_KEY` is set, a minisign
   `.sig` computed over those *unsigned* bytes).
2. The whole SignPath block is skipped — build stays unsigned, unchanged
   from today — if `SIGNPATH_API_TOKEN` isn't set. Same
   graceful-degradation convention the other optional signing steps in
   this job already use (see `RELEASING.md` § Required secrets).
3. Otherwise: upload the unsigned installer as a GitHub Actions artifact,
   submit it to SignPath via the Action (`wait-for-completion: true`),
   and replace the installer in place with the signed download.
4. **Regenerate the updater `.sig`** against the now-signed installer via
   `tauri signer sign`. This step is easy to miss: Authenticode-signing
   changes the file's bytes, so the `.sig` from step 1 (computed over the
   pre-signing bytes) would no longer verify against the file actually
   shipped — silently breaking the auto-updater for that release if
   skipped.
5. The existing "Upload NSIS installer artifact", "Upload to GitHub
   Release", and "VirusTotal scan" steps are unchanged — they glob the
   same `nsis/` directory, so they pick up the signed installer
   automatically.

---

## Consequences

### Positive

- Addresses the root cause ADR-0019 couldn't: a consistent Authenticode
  publisher identity is exactly the signal ML classifiers like Trapmine's
  weight against when scoring an unknown binary, and it should reduce (not
  eliminate) false-positive risk across releases as that identity builds
  reputation — unlike per-hash FP reports, which reset every release.
- Also fixes Windows SmartScreen's separate "Unknown Publisher" warning on
  first run, which VirusTotal submission and FP reporting do nothing for.
- Zero cost, consistent with the project having no signing budget.
- Fully optional/non-blocking: the release pipeline behaves exactly as it
  does today until the manual SignPath setup is complete.

### Negative / risks

- **Depends on manual, external approval** — SignPath's OSS review is out
  of this project's control and has no committed SLA; this ADR's CI
  changes are inert until that approval lands and the secrets are set.
- **Adds a wait to `build-windows`** — signing requests may queue on
  SignPath's side; `wait-for-completion: true` blocks the job until the
  request resolves.
- **New failure mode**: if the signing request fails or times out, the
  Windows build fails outright (unlike the other optional secrets in this
  job, which degrade gracefully by omission — this one is all-or-nothing
  once `SIGNPATH_API_TOKEN` is set, since a build we intended to sign but
  silently didn't would be worse than a build that fails loudly).
- **Not a complete fix**: signing improves reputation over time but is not
  a guarantee against every ML false positive; VirusTotal submission
  (ADR-0019) remains the transparency backstop for whatever gets through.

### Neutral

- macOS already has its own signing/notarization path (Apple secrets);
  this ADR only covers Windows.
- Automating false-positive email submission to individual AV vendors was
  considered and rejected in favor of this — see
  [OEH-11](https://linear.app/platzh1rsch/issue/OEH-11) for the reasoning
  (no vendor API, and a cleared hash doesn't carry forward to the next
  release).

---

## Alternatives considered

### A. Azure Trusted Signing

**Not adopted for now.** Functionally comparable and faster to provision
(no OSS-eligibility review), but costs ~$10/month against a project with
no signing budget. Revisit if SignPath enrollment doesn't work out.

### B. `tauri.conf.json` `bundle.windows.signCommand` instead of a
dedicated Action step

**Rejected.** This hook expects a synchronous local signing command
(e.g. `signtool` against a certificate on the build machine). SignPath's
signing request is asynchronous and potentially queued — shelling out to
poll for completion mid-bundle is worse than using SignPath's
purpose-built Action, which already implements that wait.

### C. Automate false-positive reporting to AV vendors instead of signing

**Rejected** — see Context above and OEH-11. Doesn't scale past the
current release's hash.

---

## References

- [SignPath — Free code signing for open-source projects](https://signpath.io/solutions/open-source-community)
- [SignPath GitHub Actions integration docs](https://docs.signpath.io/trusted-build-systems/github)
- [`signpath/github-action-submit-signing-request`](https://github.com/SignPath/github-action-submit-signing-request)
- [SignPath GitHub Actions demo project](https://github.com/SignPath/github-actions-demo)
- [ADR-0019: VirusTotal Scan Submission in the Release Pipeline](./ADR-0019-virustotal-scan-submission.md)
- [OEH-11: Code-sign Windows builds (Authenticode)](https://linear.app/platzh1rsch/issue/OEH-11)
- [openEHR Explorer 0.5.0 VirusTotal report](https://www.virustotal.com/gui/file/05a520a49f44041e0f4a965d59891a06e60f9a55d394bcb41f41d28bf091f2b0)
