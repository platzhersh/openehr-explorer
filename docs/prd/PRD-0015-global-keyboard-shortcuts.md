# PRD-0015: Global Keyboard Shortcuts

**Version:** 1.0
**Date:** 2026-04-10
**Status:** ✅ Implemented
**Owner:** openEHR Explorer

## Summary

Implement global keyboard shortcuts for rapid navigation and common actions, improving productivity for power users and aligning with industry-standard conventions from VS Code, Chrome DevTools, and other developer tools.

## Problem

Users must click through the sidebar navigation to switch between views. No keyboard-driven workflow exists for common navigation tasks, slowing down developers who prefer keyboard-centric interfaces.

## Solution

Add global keyboard shortcuts that work from any view:

### Navigation Shortcuts

| Shortcut | Action | Route |
|----------|--------|-------|
| `Ctrl/Cmd + 1` | Switch to EHR Browser | `/ehrs` |
| `Ctrl/Cmd + 2` | Switch to Template Browser | `/templates` |
| `Ctrl/Cmd + 3` | Switch to AQL Runner | `/aql` |
| `Ctrl/Cmd + 4` | Switch to Server Manager | `/servers` |
| `Ctrl/Cmd + ,` | Open Settings | `/settings` |

### Action Shortcuts

| Shortcut | Action | Context |
|----------|--------|---------|
| `Ctrl/Cmd + Shift + I` | Toggle Request Inspector | Global |
| `Ctrl/Cmd + Shift + D` | Open Documentation | Global |
| `Ctrl/Cmd + Enter` | Execute AQL query | AQL Runner |
| `Escape` | Close dialog / panel | Global |

## Implementation

**File:** `src/App.vue`

- Global `keydown` event listener attached in `onMounted`
- Checks for `Ctrl` (Windows/Linux) or `Cmd` (macOS) modifier
- Uses Vue Router's `push()` for navigation
- Prevents default browser behavior (e.g., opening bookmarks)
- Properly cleans up listener on unmount

**File:** `src/components/RequestInspector.vue`

- Dedicated handler for `Ctrl/Cmd + Shift + I` to toggle drawer state
- Cycles through: collapsed → half → expanded → collapsed

## Design Decisions

### Why `Ctrl/Cmd + ,` for Settings?
Industry standard across VS Code, Slack, Chrome, Figma, and most desktop applications.

### Why `Ctrl/Cmd + Shift + I` for Inspector?
- **I** = Inspector (mnemonic)
- Similar to browser DevTools shortcuts (F12, Cmd+Opt+I)
- Shift modifier prevents conflict with text input fields

### Why `Ctrl/Cmd + Shift + D` for Documentation?
- **D** = Documentation (mnemonic)
- Shift modifier prevents conflict with text input fields
- Opens external documentation in user's default browser

### Why Number Keys (1-4)?
- Common pattern in developer tools (VS Code, terminals, browsers)
- Single-key shortcuts with modifier are fast and ergonomic
- Maps naturally to sidebar order (top to bottom)

## Non-Goals

- View-specific shortcuts (e.g., composition viewer actions) — deferred to future PRDs
- Customizable shortcuts — use standard conventions for consistency
- Chord-based shortcuts (e.g., `Ctrl+K Ctrl+S`) — too complex for v1

## Success Criteria

- ✅ All shortcuts work on macOS (Cmd) and Windows/Linux (Ctrl)
- ✅ Shortcuts prevent default browser behavior
- ✅ Documented in user-facing docs (`docs/docs.html`)
- ✅ Event listeners properly cleaned up to prevent memory leaks

## Related

- **Implemented in:** v0.2.0
- **Documentation:** `docs/docs.html` (Keyboard Shortcuts section)
- **Files modified:**
  - `src/App.vue` (navigation shortcuts)
  - `src/components/RequestInspector.vue` (inspector toggle)
