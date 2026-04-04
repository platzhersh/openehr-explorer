import { ref, computed } from "vue";
import { defineStore } from "pinia";
import { listen } from "@tauri-apps/api/event";

export interface RequestLogEntry {
  id: string;
  timestamp_ms: number;
  method: string;
  url: string;
  request_headers: Record<string, string>;
  request_body: string | null;
  status: number;
  response_headers: Record<string, string>;
  response_body: string | null;
  duration_ms: number;
  body_truncated: boolean;
}

const MAX_ENTRIES = 500;

export const useInspectorStore = defineStore("inspector", () => {
  const entries = ref<RequestLogEntry[]>([]);
  const selectedId = ref<string | null>(null);
  const filterText = ref("");
  const filterMethods = ref<string[]>([]);
  const filterStatusClass = ref<string[]>([]);

  const selectedEntry = computed(() =>
    entries.value.find((e) => e.id === selectedId.value) ?? null
  );

  const filteredEntries = computed(() => {
    return entries.value.filter((entry) => {
      if (filterMethods.value.length > 0 && !filterMethods.value.includes(entry.method)) {
        return false;
      }
      if (filterStatusClass.value.length > 0) {
        const cls = `${Math.floor(entry.status / 100)}xx`;
        if (!filterStatusClass.value.includes(cls)) return false;
      }
      if (filterText.value) {
        const search = filterText.value.toLowerCase();
        const path = extractPath(entry.url);
        if (!path.toLowerCase().includes(search)) return false;
      }
      return true;
    });
  });

  const hasErrors = computed(() => {
    if (entries.value.length === 0) return false;
    const latest = entries.value[0];
    return latest.status >= 400;
  });

  function addEntry(entry: RequestLogEntry) {
    entries.value.unshift(entry);
    if (entries.value.length > MAX_ENTRIES) {
      entries.value = entries.value.slice(0, MAX_ENTRIES);
    }
    // Auto-select latest entry
    selectedId.value = entry.id;
  }

  function selectEntry(id: string) {
    selectedId.value = id;
  }

  function clear() {
    entries.value = [];
    selectedId.value = null;
  }

  function reset() {
    clear();
    filterText.value = "";
    filterMethods.value = [];
    filterStatusClass.value = [];
  }

  let unlisten: (() => void) | null = null;

  async function startListening() {
    if (unlisten) return;
    unlisten = await listen<RequestLogEntry>("cdr-inspector-entry", (event) => {
      addEntry(event.payload);
    });
  }

  function stopListening() {
    if (unlisten) {
      unlisten();
      unlisten = null;
    }
  }

  return {
    entries,
    selectedId,
    selectedEntry,
    filteredEntries,
    filterText,
    filterMethods,
    filterStatusClass,
    hasErrors,
    addEntry,
    selectEntry,
    clear,
    reset,
    startListening,
    stopListening,
  };
});

export function extractPath(url: string): string {
  try {
    return new URL(url).pathname;
  } catch {
    return url;
  }
}

export function generateCurl(entry: RequestLogEntry): string {
  const parts = [`curl -X ${entry.method}`];
  parts.push(`'${entry.url}'`);

  for (const [key, value] of Object.entries(entry.request_headers)) {
    parts.push(`-H '${key}: ${value}'`);
  }

  if (entry.request_body) {
    const escaped = entry.request_body.replace(/'/g, "'\\''");
    parts.push(`-d '${escaped}'`);
  }

  return parts.join(" \\\n  ");
}

export function formatTimestamp(ms: number): string {
  return new Date(ms).toISOString();
}

export function statusClass(status: number): string {
  if (status >= 500) return "status-5xx";
  if (status >= 400) return "status-4xx";
  if (status >= 300) return "status-3xx";
  if (status >= 200) return "status-2xx";
  return "status-other";
}

export function methodClass(method: string): string {
  switch (method.toUpperCase()) {
    case "GET": return "method-get";
    case "POST": return "method-post";
    case "PUT": return "method-put";
    case "DELETE": return "method-delete";
    default: return "method-other";
  }
}
