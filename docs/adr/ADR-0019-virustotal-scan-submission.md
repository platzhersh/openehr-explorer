# ADR-0019: VirusTotal Scan Submission in the Release Pipeline

**Date:** 2026-04-20
**Status:** Accepted
**Repo:** `openehr-explorer`

---

## Context

openEHR Explorer distributes unsigned Windows binaries (`.exe` NSIS installer, `.msi`). Unsigned Tauri apps exhibit behavioural characteristics — outbound HTTPS calls, filesystem writes, self-extraction — that overlap with patterns used by malware. This causes two classes of friction:

1. **EDR quarantine at first run** (e.g. Cortex/Palo Alto at CISTEC AG) — a hard block requiring IT intervention.
2. **Elevated false-positive risk in AV databases** — engines that have never seen the file will apply heuristic scoring, which can produce false-positive detections that then propagate to other engines and corporate threat-intel feeds.

The second class is addressed by proactively submitting the release binary to VirusTotal before or at the time of release. VirusTotal scans the file across 70+ AV engines, stores the result against the SHA-256 hash, and makes that result available to any tool or EDR that queries it. Once a file's hash has a clean (or near-clean) VirusTotal report, many corporate security tools treat it with elevated trust.

**Immediate incident context:** `openEHR.Explorer_0.3.0_x64-setup.exe` was quarantined by Cortex. The file was manually submitted to VirusTotal (SHA-256: `19ea1bc54f375b82cfac0a17041f166d1096fe4ae87d2a3e7ce1d84c802e6ee1`), resulting in a 1/72 score — only SecureAge flagged it, a known false-positive pattern for unsigned Tauri/NSIS installers. A false-positive report was submitted to SecureAge. The manual process should be automated for all future releases.

---

## Decision

We will integrate VirusTotal scan submission into the GitHub Actions release pipeline using the `crazy-max/ghaction-virustotal` action. The scan runs automatically on every tag-triggered release, uploads the Windows installer(s) to VirusTotal, and appends the resulting scan link(s) to the release notes.

This is a transparency measure, not a security gate. The workflow does not block or fail the release on a positive detection — that would be counterproductive for a false-positive scenario. The scan link in the release notes gives users, IT teams, and EDR administrators an immediate, authoritative reference when evaluating the binary.

---

## Implementation

### Prerequisites

1. Register a free VirusTotal Community account at [virustotal.com](https://www.virustotal.com)
2. Retrieve the API key from the account profile page
3. Add the key as a GitHub Actions repository secret: `VT_API_KEY`

The free public API is sufficient: 500 requests/day, 4 requests/minute. A release pipeline producing 1–2 Windows installers per release is well within these limits.

### GitHub Actions workflow step

Added to `.github/workflows/ci.yml` as an additional step in the `build-windows` job, gated on tag pushes so it only fires during a real release (not on manual `workflow_dispatch` Windows builds):

```yaml
- name: VirusTotal scan
  if: startsWith(github.ref, 'refs/tags/')
  uses: crazy-max/ghaction-virustotal@v5
  with:
    vt_api_key: ${{ secrets.VT_API_KEY }}
    update_release_body: true
    request_rate: 4
    files: |
      src-tauri/target/release/bundle/nsis/openEHR.Explorer_*.exe
      src-tauri/target/release/bundle/msi/openEHR.Explorer_*.msi
```

The step runs after `Upload to GitHub Release` so the release already exists when the action attempts to append the scan links.

`update_release_body: true` appends the VirusTotal analysis URL(s) directly to the GitHub Release notes, making them publicly visible to anyone reading the release page.

`request_rate: 4` respects the free tier rate limit.

### What gets submitted

- The NSIS `.exe` installer (primary Windows distribution format)
- The `.msi` installer (secondary, for enterprise/GPO deployment — produced by Tauri with `bundle.targets: "all"`)
- Linux and macOS builds are not submitted — VirusTotal coverage for those platforms is less relevant to the EDR friction problem, and API quota is conserved

### Behaviour when hash is already known

If the file has been previously submitted (e.g. manually, as in the 0.3.0 incident), VirusTotal returns the existing report rather than re-scanning. The action handles this transparently — the release notes still receive the correct link.

---

## Consequences

### Positive

- **Day-0 scan record**: Every release has a VirusTotal report from the moment it is published. IT teams and EDR administrators have an immediate reference without needing to submit manually.
- **Reduced friction for corporate users**: Many EDR products (including Cortex) query VirusTotal reputation as one signal. A clean, timestamped report for the exact binary hash reduces the probability of quarantine.
- **Public transparency**: The VirusTotal link in release notes is visible to all users — consistent with the project's open-source ethos and useful for security-conscious clinical IT audiences.
- **False-positive visibility**: If any engine flags the binary, the maintainer is aware immediately at release time rather than when a user reports it.
- **Zero cost**: The free VirusTotal public API is sufficient for the project's release cadence.
- **Minimal CI overhead**: One additional step, ~30 seconds, no build dependencies.

### Negative / risks

- **Not a security gate**: The step does not fail the release on detections. A high detection count would need to be caught by the maintainer reviewing the release notes. This is intentional — a false-positive should not block a legitimate release.
- **Public API terms**: The VirusTotal public API must not be used in commercial products or services. openEHR Explorer is Apache 2.0 open-source with no commercial use — this constraint is satisfied. If the project's status changes, the API usage must be re-evaluated.
- **API key as secret**: The `VT_API_KEY` must be stored as a GitHub Actions secret and rotated if compromised. Low risk — the free public API key has no elevated privileges and rate-limited exposure is the only consequence of compromise.
- **Does not replace code signing**: VirusTotal reputation is one trust signal. EDR products with strict unsigned-binary policies (like the Cortex rule that triggered the original incident) will still block unsigned binaries regardless of VirusTotal score. Code signing (not yet adopted in this project) would be complementary, not a substitute.

### Neutral

- **Covers Windows installers only**: macOS and Linux builds are excluded. macOS uses Apple notarization for trust (separate concern); Linux users are generally less subject to EDR-driven quarantine.
- **False-positive reporting remains manual**: When an engine flags the binary, the false-positive report to that engine's vendor (e.g. SecureAge's submission portal) must still be submitted manually. This ADR automates detection visibility, not remediation.

---

## Alternatives considered

### A. Manual submission only (status quo)

**Rejected.** The 0.3.0 incident demonstrated that manual submission is reactive — the binary was quarantined before the scan was available. Automating submission at release time makes the report available before users encounter the binary.

### B. Fail the release on any positive detection

**Rejected.** A single false-positive from an engine like SecureAge (known for high false-positive rates on unsigned Tauri apps) would block every release until remediated. The false-positive report process takes 24–48 hours. This is unacceptable for a project where the maintainer is the only person cutting releases.

### C. Submit all platforms (macOS, Linux, Windows)

**Not adopted for now.** macOS builds are notarized by Apple, which provides its own trust chain. Linux users are rarely subject to AV quarantine in the developer/clinical informatics context. API quota is better conserved for the Windows installer, where the problem is real.

---

## References

- [crazy-max/ghaction-virustotal — GitHub Marketplace](https://github.com/marketplace/actions/virustotal-github-action)
- [VirusTotal API v3 — Files: Upload a file](https://docs.virustotal.com/reference/files-scan)
- [VirusTotal — Public vs Premium API](https://docs.virustotal.com/reference/public-vs-premium-api)
- [openEHR.Explorer_0.3.0 VirusTotal report](https://www.virustotal.com/gui/file/19ea1bc54f375b82cfac0a17041f166d1096fe4ae87d2a3e7ce1d84c802e6ee1/summary)
