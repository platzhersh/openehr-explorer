import * as monaco from "monaco-editor";
import { AQL_KEYWORDS, AQL_RM_TYPES } from "./keywords";
import { RM_PATH_MAP } from "./rmPaths";
import { parseAliases, findAlias } from "./aliasParser";
import type { AqlPathEntry } from "./aqlPathIndex";
import { AQL_LANGUAGE_ID } from "./language";

export interface CompletionContext {
  /** Layer 3: aqlPath entries from the selected template, keyed by archetype ID */
  templatePaths: Map<string, AqlPathEntry[]>;
  /** Layer 3: all template paths (when no specific archetype match) */
  allTemplatePaths: AqlPathEntry[];
}

/**
 * Check whether the cursor position is inside a string literal.
 */
function isInsideString(model: monaco.editor.ITextModel, position: monaco.Position): boolean {
  const lineContent = model.getLineContent(position.lineNumber);
  const textBefore = lineContent.substring(0, position.column - 1);

  let inSingle = false;
  let inDouble = false;
  for (const ch of textBefore) {
    if (ch === "'" && !inDouble) inSingle = !inSingle;
    if (ch === '"' && !inSingle) inDouble = !inDouble;
  }
  return inSingle || inDouble;
}

/**
 * Get the word/alias immediately before a `/` at the cursor position.
 * Returns the alias text and range, or null if the cursor is not in a path context.
 */
function getAliasBeforeSlash(
  model: monaco.editor.ITextModel,
  position: monaco.Position,
): { alias: string; slashColumn: number } | null {
  const lineContent = model.getLineContent(position.lineNumber);
  const textBefore = lineContent.substring(0, position.column - 1);

  // Look for pattern: alias/ at the end of textBefore
  const match = textBefore.match(/\b([a-zA-Z_]\w*)\/([a-zA-Z_/\[\]@\d]*)$/);
  if (match) {
    return {
      alias: match[1],
      slashColumn: position.column - match[2].length - 1,
    };
  }

  return null;
}

export function createCompletionProvider(
  contextRef: { value: CompletionContext },
): monaco.languages.CompletionItemProvider {
  return {
    triggerCharacters: ["/", " ", "\n"],

    provideCompletionItems(
      model: monaco.editor.ITextModel,
      position: monaco.Position,
    ): monaco.languages.ProviderResult<monaco.languages.CompletionList> {
      // Never complete inside strings
      if (isInsideString(model, position)) {
        return { suggestions: [] };
      }

      const suggestions: monaco.languages.CompletionItem[] = [];
      const word = model.getWordUntilPosition(position);
      const range = new monaco.Range(
        position.lineNumber,
        word.startColumn,
        position.lineNumber,
        position.column,
      );

      // Check if we're in a path context (alias/)
      const aliasInfo = getAliasBeforeSlash(model, position);

      if (aliasInfo) {
        // Layer 2 & 3: Path completions
        const fullText = model.getValue();
        const bindings = parseAliases(fullText);
        const binding = findAlias(bindings, aliasInfo.alias);

        if (binding) {
          // Calculate range for the path portion after alias/
          const pathRange = new monaco.Range(
            position.lineNumber,
            aliasInfo.slashColumn + 1,
            position.lineNumber,
            position.column,
          );

          // Layer 3: Template-aware paths for archetype-constrained aliases
          if (binding.archetypeId) {
            const ctx = contextRef.value;
            const templateEntries = ctx.templatePaths.get(binding.archetypeId);
            if (templateEntries && templateEntries.length > 0) {
              for (const entry of templateEntries) {
                suggestions.push({
                  label: entry.label,
                  kind: monaco.languages.CompletionItemKind.Field,
                  detail: entry.rmType,
                  documentation: entry.aqlPath,
                  insertText: entry.aqlPath,
                  range: pathRange,
                  sortText: `0_${entry.label}`,
                });
              }
              return { suggestions };
            }
          }

          // Layer 2: Static RM path completions
          const rmPaths = RM_PATH_MAP[binding.rmType];
          if (rmPaths) {
            for (const entry of rmPaths) {
              suggestions.push({
                label: `${aliasInfo.alias}/${entry.path}`,
                kind: monaco.languages.CompletionItemKind.Field,
                detail: entry.rmType,
                documentation: entry.description,
                insertText: entry.path,
                range: pathRange,
                sortText: `0_${entry.path}`,
              });
            }
            return { suggestions };
          }

          // If we have template paths but no archetype filter, offer all template paths
          if (contextRef.value.allTemplatePaths.length > 0) {
            for (const entry of contextRef.value.allTemplatePaths) {
              suggestions.push({
                label: entry.label,
                kind: monaco.languages.CompletionItemKind.Field,
                detail: entry.rmType,
                documentation: entry.aqlPath,
                insertText: entry.aqlPath,
                range: pathRange,
                sortText: `1_${entry.label}`,
              });
            }
          }

          return { suggestions };
        }
      }

      // Layer 1: Keyword completions
      const snippetRule = monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet;

      for (const kw of AQL_KEYWORDS) {
        suggestions.push({
          label: kw.label,
          kind: monaco.languages.CompletionItemKind.Keyword,
          documentation: kw.documentation,
          insertText: kw.insertText,
          insertTextRules: kw.isSnippet ? snippetRule : undefined,
          range,
          sortText: `1_${kw.label}`,
        });
      }

      // RM type completions (useful in FROM/CONTAINS context)
      for (const rm of AQL_RM_TYPES) {
        suggestions.push({
          label: rm.label,
          kind: monaco.languages.CompletionItemKind.Class,
          documentation: rm.documentation,
          insertText: rm.insertText,
          insertTextRules: rm.isSnippet ? snippetRule : undefined,
          range,
          sortText: `2_${rm.label}`,
        });
      }

      return { suggestions };
    },
  };
}

export function registerCompletionProvider(
  contextRef: { value: CompletionContext },
): monaco.IDisposable {
  return monaco.languages.registerCompletionItemProvider(
    AQL_LANGUAGE_ID,
    createCompletionProvider(contextRef),
  );
}
