import * as monaco from "monaco-editor";

export const AQL_THEME_ID = "aql-explorer-dark";

export function registerAqlTheme(): void {
  monaco.editor.defineTheme(AQL_THEME_ID, {
    base: "vs-dark",
    inherit: true,
    rules: [
      { token: "keyword", foreground: "569cd6", fontStyle: "bold" },
      { token: "keyword.function", foreground: "dcdcaa" },
      { token: "type.rm", foreground: "4ec9b0" },
      { token: "type.archetype", foreground: "4ec9b0" },
      { token: "string", foreground: "ce9178" },
      { token: "number", foreground: "b5cea8" },
      { token: "comment", foreground: "6a9955", fontStyle: "italic" },
      { token: "tag.path", foreground: "9cdcfe" },
      { token: "identifier", foreground: "e0e0e0" },
      { token: "operator", foreground: "d4d4d4" },
      { token: "delimiter", foreground: "d4d4d4" },
    ],
    colors: {
      "editor.background": "#1a1a2e",
      "editor.foreground": "#e0e0e0",
      "editor.lineHighlightBackground": "#1e2a4a",
      "editor.selectionBackground": "#253456",
      "editor.inactiveSelectionBackground": "#1e2a4a80",
      "editorCursor.foreground": "#64ffda",
      "editorLineNumber.foreground": "#5a6a8a",
      "editorLineNumber.activeForeground": "#8892b0",
      "editorIndentGuide.background": "#2a3a5c",
      "editorWidget.background": "#16213e",
      "editorWidget.border": "#2a3a5c",
      "editorSuggestWidget.background": "#16213e",
      "editorSuggestWidget.border": "#2a3a5c",
      "editorSuggestWidget.selectedBackground": "#1e2a4a",
      "editorSuggestWidget.highlightForeground": "#64ffda",
      "input.background": "#16213e",
      "input.border": "#2a3a5c",
      "input.foreground": "#e0e0e0",
      "scrollbarSlider.background": "#2a3a5c80",
      "scrollbarSlider.hoverBackground": "#2a3a5ca0",
      "scrollbarSlider.activeBackground": "#2a3a5cc0",
    },
  });
}
