import { ref, onMounted, onBeforeUnmount, watch, type Ref } from "vue";
import * as monaco from "monaco-editor";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import { registerAqlLanguage, AQL_LANGUAGE_ID } from "../lib/aql/language";
import { registerAqlTheme, AQL_THEME_ID } from "../lib/aql/theme";

// Configure Monaco environment for web workers
self.MonacoEnvironment = {
  getWorker() {
    return new editorWorker();
  },
};

// Register language and theme once
let registered = false;
function ensureRegistered() {
  if (registered) return;
  registered = true;
  registerAqlLanguage();
  registerAqlTheme();
}

export interface UseMonacoEditorOptions {
  language?: string;
  initialValue?: string;
  onExecute?: () => void;
  onChange?: (value: string) => void;
}

export function useMonacoEditor(
  container: Ref<HTMLElement | null>,
  options: UseMonacoEditorOptions = {},
) {
  const editor = ref<monaco.editor.IStandaloneCodeEditor | null>(null);

  function getValue(): string {
    return editor.value?.getValue() ?? "";
  }

  function setValue(text: string): void {
    if (editor.value && editor.value.getValue() !== text) {
      editor.value.setValue(text);
    }
  }

  function focus(): void {
    editor.value?.focus();
  }

  onMounted(async () => {
    if (!container.value) return;

    // Defer Monaco initialization to next tick to avoid blocking initial render
    await new Promise(resolve => setTimeout(resolve, 0));

    ensureRegistered();

    const instance = monaco.editor.create(container.value, {
      value: options.initialValue ?? "",
      language: options.language ?? AQL_LANGUAGE_ID,
      theme: AQL_THEME_ID,
      minimap: { enabled: false },
      fontSize: 13,
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
      lineNumbers: "on",
      lineNumbersMinChars: 3,
      scrollBeyondLastLine: false,
      automaticLayout: true,
      tabSize: 2,
      wordWrap: "on",
      padding: { top: 12, bottom: 12 },
      suggestOnTriggerCharacters: true,
      quickSuggestions: {
        other: false,
        comments: false,
        strings: false,
      },
      suggest: {
        showKeywords: true,
        showFields: true,
        showClasses: true,
      },
      overviewRulerLanes: 0,
      hideCursorInOverviewRuler: true,
      renderLineHighlight: "line",
      bracketPairColorization: { enabled: true },
      matchBrackets: "always",
      scrollbar: {
        verticalScrollbarSize: 8,
        horizontalScrollbarSize: 8,
      },
    });

    // Ctrl/Cmd+Enter to execute
    if (options.onExecute) {
      const executeHandler = options.onExecute;
      instance.addAction({
        id: "aql-execute",
        label: "Execute AQL Query",
        keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter],
        run() {
          executeHandler();
        },
      });
    }

    // Emit changes
    if (options.onChange) {
      const changeHandler = options.onChange;
      instance.onDidChangeModelContent(() => {
        changeHandler(instance.getValue());
      });
    }

    editor.value = instance;

    // Warm up Monaco by triggering lazy-loaded features
    // This prevents freezing on first user interaction
    setTimeout(() => {
      if (instance && !instance.hasTextFocus()) {
        // Trigger tokenization
        instance.getModel()?.tokenization?.getLineTokens(1);

        // Trigger layout calculation
        instance.layout();

        // Force a small edit and undo to warm up the text buffer
        const model = instance.getModel();
        if (model) {
          const position = { lineNumber: 1, column: 1 };
          model.pushEditOperations(
            [],
            [{ range: new monaco.Range(1, 1, 1, 1), text: ' ' }],
            () => null
          );
          instance.trigger('warmup', 'undo', {});
        }
      }
    }, 150);
  });

  onBeforeUnmount(() => {
    editor.value?.dispose();
    editor.value = null;
  });

  // Watch container for visibility changes
  watch(container, (el) => {
    if (el && editor.value) {
      editor.value.layout();
    }
  });

  return { editor, getValue, setValue, focus };
}
