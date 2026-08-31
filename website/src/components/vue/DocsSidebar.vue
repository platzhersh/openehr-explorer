<script setup lang="ts">
// The docs page sidebar: active-section tracking (IntersectionObserver)
// plus an in-page search box that filters links and highlights matches
// in the corresponding <section>. Ported from the two vanilla-JS IIFEs
// that used to live at the bottom of docs.html. The sidebar owns its
// own markup here, but reaches into the rest of the page's DOM (the
// `.docs-content section[id]` elements rendered by docs.astro) exactly
// like the original script did — that content isn't itself a Vue
// island, so this stays the simplest way to keep both pieces in sync.
import { onMounted, onBeforeUnmount, ref } from "vue";

interface NavGroup {
  title: string;
  links: { href: string; label: string }[];
}

defineProps<{ groups: NavGroup[] }>();

const query = ref("");
const hiddenLinks = ref(new Set<string>());
const hiddenGroups = ref(new Set<string>());
const noResults = ref(false);
const activeHref = ref("");
const searchInput = ref<HTMLInputElement | null>(null);

let observer: IntersectionObserver | null = null;

function clearHighlights() {
  document.querySelectorAll(".docs-content mark").forEach((mark) => {
    const parent = mark.parentNode;
    if (!parent) return;
    parent.replaceChild(document.createTextNode(mark.textContent || ""), mark);
    parent.normalize();
  });
}

function highlightInSection(sectionEl: Element, q: string) {
  const walker = document.createTreeWalker(sectionEl, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      if (!node.nodeValue?.toLowerCase().includes(q)) return NodeFilter.FILTER_REJECT;
      if (node.parentElement?.closest("script, style")) return NodeFilter.FILTER_REJECT;
      return NodeFilter.FILTER_ACCEPT;
    },
  });
  const nodes: Text[] = [];
  let n: Node | null;
  while ((n = walker.nextNode())) nodes.push(n as Text);

  nodes.forEach((node) => {
    const text = node.nodeValue || "";
    const lower = text.toLowerCase();
    const frag = document.createDocumentFragment();
    let last = 0;
    let idx: number;
    while ((idx = lower.indexOf(q, last)) !== -1) {
      frag.appendChild(document.createTextNode(text.slice(last, idx)));
      const mark = document.createElement("mark");
      mark.textContent = text.slice(idx, idx + q.length);
      frag.appendChild(mark);
      last = idx + q.length;
    }
    frag.appendChild(document.createTextNode(text.slice(last)));
    node.parentNode?.replaceChild(frag, node);
  });
}

// Returns the set of sidebar-link hrefs whose own text or matching
// section's text contains `q`.
function findMatchingLinks(links: HTMLAnchorElement[], q: string): Set<string> {
  const matching = new Set<string>();
  for (const link of links) {
    const href = link.getAttribute("href") || "";
    const section = document.getElementById(href.slice(1));
    const text = `${link.textContent} ${section ? section.textContent : ""}`.toLowerCase();
    if (text.includes(q)) matching.add(href);
  }
  return matching;
}

// A group heading is hidden once none of the sidebar links following it
// (up to the next heading) are still visible.
function groupHasVisibleLink(group: Element, stillVisible: Set<string>): boolean {
  let next = group.nextElementSibling;
  while (next && !next.classList.contains("sidebar-group-title")) {
    if (next.classList.contains("sidebar-link") && stillVisible.has(next.getAttribute("href") || "")) {
      return true;
    }
    next = next.nextElementSibling;
  }
  return false;
}

function findHiddenGroups(stillVisible: Set<string>): Set<string> {
  const hidden = new Set<string>();
  for (const group of document.querySelectorAll(".sidebar-group-title")) {
    if (!groupHasVisibleLink(group, stillVisible)) hidden.add(group.textContent || "");
  }
  return hidden;
}

function highlightMatchingSections(sections: Element[], q: string) {
  for (const section of sections) {
    if (section.textContent?.toLowerCase().includes(q)) highlightInSection(section, q);
  }
}

function resetSearch() {
  hiddenLinks.value = new Set();
  hiddenGroups.value = new Set();
  noResults.value = false;
}

function runSearch() {
  const q = query.value.trim().toLowerCase();
  clearHighlights();
  if (!q) {
    resetSearch();
    return;
  }

  const links = Array.from(document.querySelectorAll<HTMLAnchorElement>(".sidebar-link"));
  const stillVisible = findMatchingLinks(links, q);

  hiddenLinks.value = new Set(links.map((l) => l.getAttribute("href") || "").filter((href) => !stillVisible.has(href)));
  hiddenGroups.value = findHiddenGroups(stillVisible);
  noResults.value = stillVisible.size === 0;

  if (stillVisible.size > 0) {
    const sections = Array.from(document.querySelectorAll(".docs-content section[id]"));
    highlightMatchingSections(sections, q);
  }
}

function onSearchKeydown(e: KeyboardEvent) {
  if (e.key !== "Enter") return;
  const firstVisible = Array.from(document.querySelectorAll<HTMLAnchorElement>(".sidebar-link")).find(
    (l) => !hiddenLinks.value.has(l.getAttribute("href") || ""),
  );
  if (firstVisible) {
    e.preventDefault();
    window.location.hash = firstVisible.getAttribute("href") || "";
  }
}

function onGlobalKeydown(e: KeyboardEvent) {
  const active = document.activeElement;
  if (e.key === "/" && active !== searchInput.value) {
    const tag = (active && (active as HTMLElement).tagName) || "";
    if (tag !== "INPUT" && tag !== "TEXTAREA") {
      e.preventDefault();
      searchInput.value?.focus();
    }
  }
  if (e.key === "Escape" && active === searchInput.value) {
    query.value = "";
    runSearch();
    searchInput.value?.blur();
  }
}

onMounted(() => {
  const sections = Array.from(document.querySelectorAll(".docs-content section[id]"));
  observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) activeHref.value = `#${entry.target.id}`;
      });
    },
    { rootMargin: "-60px 0px -80% 0px", threshold: 0 },
  );
  sections.forEach((section) => observer?.observe(section));

  document.addEventListener("keydown", onGlobalKeydown);
});

onBeforeUnmount(() => {
  observer?.disconnect();
  document.removeEventListener("keydown", onGlobalKeydown);
});
</script>

<template>
  <nav class="sidebar" aria-label="Documentation navigation">
    <div class="sidebar-search">
      <input
        id="docs-search"
        ref="searchInput"
        v-model="query"
        @input="runSearch"
        @keydown="onSearchKeydown"
        type="search"
        class="sidebar-search-input"
        placeholder="Search docs… (Press /)"
        aria-label="Search documentation"
        autocomplete="off"
      />
    </div>
    <p id="docs-search-no-results" class="sidebar-no-results" v-show="noResults">No matching topics</p>

    <template v-for="group in groups" :key="group.title">
      <div class="sidebar-group-title" v-show="!hiddenGroups.has(group.title)">{{ group.title }}</div>
      <a
        v-for="link in group.links"
        :key="link.href"
        :href="link.href"
        class="sidebar-link"
        :class="{ active: activeHref === link.href }"
        v-show="!hiddenLinks.has(link.href)"
        >{{ link.label }}</a
      >
    </template>
  </nav>
</template>

<style>
.sidebar {
  width: 260px;
  position: sticky;
  top: 60px;
  height: calc(100vh - 60px);
  overflow-y: auto;
  background: var(--bg-secondary);
  border-right: 1px solid var(--border);
  padding: 24px 0;
  flex-shrink: 0;
}
.sidebar-group-title {
  padding: 8px 24px;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--text-muted);
  margin-top: 20px;
  font-weight: 600;
}
.sidebar-group-title:first-child {
  margin-top: 0;
}
.sidebar-search {
  padding: 0 24px 16px;
}
.sidebar-search-input {
  width: 100%;
  padding: 8px 10px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text);
  font-family: var(--font-sans);
  font-size: 14px;
}
.sidebar-search-input::placeholder {
  color: var(--text-muted);
}
.sidebar-search-input:focus-visible {
  outline: 2px solid var(--primary);
  outline-offset: 1px;
}
.sidebar-no-results {
  padding: 8px 24px;
  color: var(--text-muted);
  font-size: 13px;
}
.sidebar-link {
  display: block;
  padding: 7px 24px;
  color: var(--text-secondary);
  text-decoration: none;
  font-size: 14px;
  border-left: 3px solid transparent;
  transition: all 0.2s;
}
.sidebar-link:hover {
  color: var(--text);
  background: rgba(100, 255, 218, 0.03);
  text-decoration: none;
}
.sidebar-link.active {
  color: var(--primary);
  border-left-color: var(--primary);
  background: rgba(100, 255, 218, 0.05);
}

@media (max-width: 768px) {
  .sidebar {
    position: static;
    width: 100%;
    height: auto;
    border-right: none;
    border-bottom: 1px solid var(--border);
    padding: 12px 0;
    display: flex;
    flex-wrap: wrap;
    gap: 0;
    overflow-x: auto;
  }
  .sidebar-group-title {
    width: 100%;
    margin-top: 8px;
    padding: 4px 16px;
  }
  .sidebar-link {
    border-left: none;
    border-bottom: 2px solid transparent;
    padding: 6px 12px;
    white-space: nowrap;
  }
  .sidebar-link.active {
    border-left-color: transparent;
    border-bottom-color: var(--primary);
  }
}
</style>
