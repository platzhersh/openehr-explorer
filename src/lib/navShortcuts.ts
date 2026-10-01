// Single source of truth for the sidebar tabs and their Ctrl/Cmd+N shortcuts
// (OEH-98). The sidebar renders from this list and App.vue's key handler
// resolves shortcuts against it, so the two can't drift apart.

export interface NavItem {
  path: string;
  label: string;
  icon: string;
}

export const navItems: readonly NavItem[] = [
  { path: "/dashboard", label: "Overview", icon: "O" },
  { path: "/ehrs", label: "EHR Browser", icon: "H" },
  { path: "/templates", label: "Templates", icon: "T" },
  { path: "/aql", label: "AQL Runner", icon: "Q" },
  { path: "/terminology", label: "Terminology", icon: "V" },
  { path: "/servers", label: "Servers", icon: "S" },
];

/** Settings lives in the sidebar footer, bound to Ctrl/Cmd+, */
export const SETTINGS_SHORTCUT_KEY = ",";

/** The key (without modifier) that activates a tab: "1" for the first, etc. */
export function shortcutKeyForIndex(index: number): string {
  return String(index + 1);
}

/** Route path for a pressed key, or null when it isn't a tab shortcut. */
export function pathForShortcutKey(key: string): string | null {
  if (key === SETTINGS_SHORTCUT_KEY) return "/settings";
  if (!/^[1-9]$/.test(key)) return null;
  return navItems[Number(key) - 1]?.path ?? null;
}

/** "1–6" style range for help text, derived from the current tab count. */
export function shortcutRangeLabel(): string {
  return `1–${navItems.length}`;
}

export function isMacPlatform(): boolean {
  return typeof navigator !== "undefined" && /mac/i.test(navigator.platform || "");
}
