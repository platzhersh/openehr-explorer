# ADR-0012: CodeMirror 6 for AQL Editor

**Date:** 2026-04-04
**Status:** Accepted
**Deciders:** Development Team
**Related:** PRD-0006 (AQL Editor with Monaco/CodeMirror Autocomplete)

## Context

PRD-0006 requires replacing the plain textarea AQL editor with a proper code editor featuring syntax highlighting, autocomplete, and formatting. The PRD originally considered Monaco Editor (VS Code's editor) as the primary option, with CodeMirror as an alternative.

During initial implementation with Monaco Editor, we encountered a critical performance issue: the AQL Runner page would completely freeze on first load in the Tauri WebView environment. Investigation revealed that Monaco Editor's synchronous initialization (~4MB bundle, web worker setup) was blocking the main thread, causing the UI to become unresponsive.

## Decision

We will use **CodeMirror 6** as the code editor for the AQL Runner instead of Monaco Editor.

## Rationale

### Performance Issues with Monaco Editor
1. **UI Freezing**: Monaco's synchronous initialization blocked Tauri's WebView main thread, making the app completely unresponsive on page load
2. **Bundle Size**: Monaco adds ~4MB to the bundle, significantly increasing load time
3. **Web Workers**: Monaco relies on web workers which add complexity in Tauri's environment
4. **Overhead**: Monaco is designed for full IDE-like experiences (multi-file editing, diffing, etc.) which is overkill for a single query editor

### Advantages of CodeMirror 6
1. **Lightweight**: ~200KB bundle vs Monaco's 4MB (95% smaller)
2. **Performance**: Fast initialization with no blocking operations
3. **No Web Workers**: Simpler architecture, better suited for Tauri
4. **Modern API**: Clean, composable extension system
5. **TypeScript Support**: First-class TypeScript support with excellent type safety
6. **Sufficient Features**: Provides all required features (syntax highlighting, autocomplete, keybindings, themes)

### Feature Parity
CodeMirror 6 supports all features required by PRD-0006:
- ✅ Syntax highlighting (AQL language based on SQL)
- ✅ 3-tier autocomplete system
  - Layer 1: Keywords and RM types
  - Layer 2: Static RM paths (EHR, COMPOSITION)
  - Layer 3: Template-aware paths from Web Templates
- ✅ Custom keybindings (Cmd/Ctrl+Enter for execute, Shift+Alt+F for format)
- ✅ Dark theme matching app's color scheme
- ✅ Auto-format functionality

## Consequences

### Positive
- **Instant Loading**: AQL Runner page loads immediately with no freezing
- **Smaller Bundle**: Production build is significantly smaller and faster to load
- **Better UX**: Responsive editor with smooth autocomplete interactions
- **Maintainability**: Simpler codebase with fewer dependencies
- **Tauri-Friendly**: No web worker complexity or synchronous blocking

### Negative
- **Different API**: CodeMirror has different patterns than Monaco (mitigated by clean composable design)
- **Less Recognition**: Monaco has higher brand recognition as "VS Code's editor" (not relevant for end users)

### Neutral
- **Feature Set**: CodeMirror provides exactly what we need, no more, no less
- **Learning Curve**: Both editors require learning their respective APIs

## Implementation

The implementation consists of:

1. **Core Files**:
   - `src/lib/aql/codemirror-aql.ts` - AQL language definition (extends SQL)
   - `src/lib/aql/codemirror-autocomplete.ts` - 3-tier autocomplete provider
   - `src/lib/aql/formatter.ts` - SQL-style AQL formatter
   - `src/composables/useCodeMirror.ts` - CodeMirror lifecycle management

2. **Dependencies** (all pinned versions):
   - `codemirror` - Core editor
   - `@codemirror/lang-sql` - SQL language base for AQL
   - `@codemirror/autocomplete` - Autocomplete system
   - `@codemirror/view` - Editor view layer
   - `@codemirror/state` - State management
   - `@codemirror/commands` - Standard commands
   - `@codemirror/language` - Language support infrastructure

3. **Removed Dependencies**:
   - `monaco-editor` (was causing UI freezing)

## Alternatives Considered

### Monaco Editor
- **Pros**: VS Code's editor, feature-rich, well-known
- **Cons**: UI freezing in Tauri, 4MB bundle, web worker complexity, overkill for single-file editing
- **Verdict**: Rejected due to performance issues

### Plain Textarea
- **Pros**: No dependencies, simple
- **Cons**: No syntax highlighting, no autocomplete, poor UX for complex queries
- **Verdict**: Rejected, does not meet PRD-0006 requirements

### Ace Editor
- **Pros**: Lightweight, established
- **Cons**: Older codebase, less active development, not as modern as CodeMirror 6
- **Verdict**: Not evaluated (CodeMirror 6 already meets all needs)

## References

- PRD-0006: AQL Editor with Monaco/CodeMirror Autocomplete
- [CodeMirror 6 Documentation](https://codemirror.net/docs/)
- Implementation commits:
  - `65a826c` - Replace AQL textarea with Monaco Editor and 3-tier autocomplete (initial attempt)
  - `fe8cdc6` - Migrate from Monaco Editor to CodeMirror 6 (final solution)
  - `655d961` - Add auto-format functionality for AQL queries
  - `341b8ca` - Remove Monaco Editor files and fix TypeScript errors
