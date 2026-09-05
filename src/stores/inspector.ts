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

  const selectedEntry = computed(
    () => entries.value.find((e) => e.id === selectedId.value) ?? null,
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

// Standard reason phrases (RFC 7231 / IANA HTTP status code registry) for
// the codes actually seen from openEHR CDRs, plus the common general-web
// ones. Unknown/nonstandard codes (proxies, custom gateways) fall back to
// the empty string rather than guessing a phrase.
const HTTP_STATUS_TEXT: Record<number, string> = {
  100: "Continue",
  101: "Switching Protocols",
  200: "OK",
  201: "Created",
  202: "Accepted",
  203: "Non-Authoritative Information",
  204: "No Content",
  205: "Reset Content",
  206: "Partial Content",
  300: "Multiple Choices",
  301: "Moved Permanently",
  302: "Found",
  303: "See Other",
  304: "Not Modified",
  307: "Temporary Redirect",
  308: "Permanent Redirect",
  400: "Bad Request",
  401: "Unauthorized",
  402: "Payment Required",
  403: "Forbidden",
  404: "Not Found",
  405: "Method Not Allowed",
  406: "Not Acceptable",
  407: "Proxy Authentication Required",
  408: "Request Timeout",
  409: "Conflict",
  410: "Gone",
  411: "Length Required",
  412: "Precondition Failed",
  413: "Payload Too Large",
  414: "URI Too Long",
  415: "Unsupported Media Type",
  416: "Range Not Satisfiable",
  417: "Expectation Failed",
  422: "Unprocessable Entity",
  423: "Locked",
  424: "Failed Dependency",
  428: "Precondition Required",
  429: "Too Many Requests",
  431: "Request Header Fields Too Large",
  451: "Unavailable For Legal Reasons",
  500: "Internal Server Error",
  501: "Not Implemented",
  502: "Bad Gateway",
  503: "Service Unavailable",
  504: "Gateway Timeout",
  505: "HTTP Version Not Supported",
};

export function statusText(status: number): string {
  return HTTP_STATUS_TEXT[status] ?? "";
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
    case "GET":
      return "method-get";
    case "POST":
      return "method-post";
    case "PUT":
      return "method-put";
    case "DELETE":
      return "method-delete";
    default:
      return "method-other";
  }
}
