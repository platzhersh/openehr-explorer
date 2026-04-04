import { ref, onMounted, onBeforeUnmount, watch, type Ref } from "vue";
import { EditorView, keymap } from "@codemirror/view";
import { EditorState, type Extension } from "@codemirror/state";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { syntaxHighlighting, defaultHighlightStyle, bracketMatching } from "@codemirror/language";
import { aql } from "../lib/aql/codemirror-aql";
import { aqlAutocomplete, type AqlCompletionConfig } from "../lib/aql/codemirror-autocomplete";

export interface UseCodeMirrorOptions {
  initialValue?: string;
  onExecute?: () => void;
  onChange?: (value: string) => void;
  onFormat?: () => void;
  completionConfig: { value: AqlCompletionConfig };
}

/**
 * Dark theme for CodeMirror matching the app's color scheme
 */
const darkTheme = EditorView.theme({
  "&": {
    backgroundColor: "var(--color-bg)",
    color: "var(--color-text)",
    height: "100%",
  },
  ".cm-content": {
    caretColor: "var(--color-text)",
    fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
    fontSize: "13px",
    padding: "12px 0",
  },
  ".cm-cursor": {
    borderLeftColor: "var(--color-text)",
  },
  ".cm-selectionBackground, ::selection": {
    backgroundColor: "var(--color-surface) !important",
  },
  "&.cm-focused .cm-selectionBackground, &.cm-focused ::selection": {
    backgroundColor: "var(--color-surface) !important",
  },
  ".cm-activeLine": {
    backgroundColor: "rgba(255, 255, 255, 0.03)",
  },
  ".cm-gutters": {
    backgroundColor: "var(--color-bg-secondary)",
    color: "var(--color-text-muted)",
    border: "none",
    fontSize: "12px",
  },
  ".cm-activeLineGutter": {
    backgroundColor: "rgba(255, 255, 255, 0.05)",
  },
  ".cm-lineNumbers .cm-gutterElement": {
    padding: "0 8px",
  },
}, { dark: true });

export function useCodeMirror(
  container: Ref<HTMLElement | null>,
  options: UseCodeMirrorOptions,
) {
  const view = ref<EditorView | null>(null);

  function getValue(): string {
    return view.value?.state.doc.toString() ?? "";
  }

  function setValue(text: string): void {
    if (view.value && view.value.state.doc.toString() !== text) {
      view.value.dispatch({
        changes: { from: 0, to: view.value.state.doc.length, insert: text },
      });
    }
  }

  function focus(): void {
    view.value?.focus();
  }

  onMounted(() => {
    if (!container.value) return;

    const extensions: Extension[] = [
      aql(),
      aqlAutocomplete(options.completionConfig),
      history(),
      syntaxHighlighting(defaultHighlightStyle),
      bracketMatching(),
      EditorView.lineWrapping,
      darkTheme,
      keymap.of([
        ...defaultKeymap,
        ...historyKeymap,
      ]),
      EditorView.updateListener.of((update) => {
        if (update.docChanged && options.onChange) {
          options.onChange(update.state.doc.toString());
        }
      }),
    ];

    // Add custom keybindings
    const customKeys = [];

    // Ctrl/Cmd+Enter for execute
    if (options.onExecute) {
      const executeHandler = options.onExecute;
      customKeys.push({
        key: "Mod-Enter",
        run: () => {
          executeHandler();
          return true;
        },
      });
    }

    // Shift+Alt+F for format (VS Code style)
    if (options.onFormat) {
      const formatHandler = options.onFormat;
      customKeys.push({
        key: "Shift-Alt-f",
        run: () => {
          formatHandler();
          return true;
        },
      });
    }

    if (customKeys.length > 0) {
      extensions.push(keymap.of(customKeys));
    }

    const state = EditorState.create({
      doc: options.initialValue ?? "",
      extensions,
    });

    const editorView = new EditorView({
      state,
      parent: container.value,
    });

    view.value = editorView;
  });

  onBeforeUnmount(() => {
    view.value?.destroy();
    view.value = null;
  });

  watch(container, (el) => {
    if (el && view.value) {
      // CodeMirror auto-resizes, no manual layout needed
    }
  });

  return { view, getValue, setValue, focus };
}
