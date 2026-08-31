<script setup lang="ts">
// Detects the visitor's OS and swaps the primary CTA's label to match
// (e.g. "Download for macOS"), ported 1:1 from the vanilla-JS IIFE that
// used to live at the bottom of index.html.
import { onMounted, ref } from "vue";

const label = ref("Download");

function platformFromString(value: string): string | null {
  if (value.includes("win")) return "Windows";
  if (value.includes("mac")) return "macOS";
  if (value.includes("linux")) return "Linux";
  return null;
}

function isMobileUA(ua: string): boolean {
  if (/iphone|ipad|ipod|android/.test(ua)) return true;
  if (navigator.maxTouchPoints <= 1) return false;
  // iPadOS's "desktop" Safari and Chrome's Android "desktop site" mode
  // both report as a plain desktop UA (Macintosh / X11 Linux) but,
  // unlike a real PC, expose multiple touch points.
  return ua.includes("macintosh") || ua.includes("linux");
}

function detectFromUserAgentData(uaData: { mobile?: boolean; platform?: string } | undefined): string | null {
  if (!uaData || uaData.mobile) return null; // phone/tablet — keep the default label
  return platformFromString((uaData.platform || "").toLowerCase());
}

function detectOS(): string | null {
  const ua = (navigator.userAgent || "").toLowerCase();
  if (isMobileUA(ua)) return null; // phone/tablet — keep the default label
  const uaData = (navigator as Navigator & { userAgentData?: { mobile?: boolean; platform?: string } }).userAgentData;
  return detectFromUserAgentData(uaData) || platformFromString(ua);
}

onMounted(() => {
  const os = detectOS();
  if (os) label.value = `Download for ${os}`;
});
</script>

<template>
  <a href="https://github.com/platzhersh/openehr-explorer/releases" class="btn btn-primary">{{ label }}</a>
</template>

<style scoped>
/*
 * .btn-primary's chrome is defined here rather than relied on from
 * index.astro's page-level styles: Astro does stamp its scope
 * attribute onto this component's SSR'd root element (this component
 * has exactly one root, so Vue's attribute fallthrough carries it
 * along), which is why the real page renders correctly today — but
 * that's an incidental mechanism, not something to depend on, and it
 * doesn't apply at all in Storybook (see ADR-0026), which never loads
 * index.astro's CSS. Self-contained styling matches the other website
 * Vue components (see ADR-0025).
 */
.btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 12px 28px;
  border-radius: var(--radius);
  font-size: 1rem;
  font-weight: 500;
  font-family: var(--font-sans);
  cursor: pointer;
  transition: all 0.2s;
  text-decoration: none;
}
.btn:hover {
  text-decoration: none;
}
.btn-primary {
  background: var(--primary-dim);
  color: #fff;
  border: none;
}
.btn-primary:hover {
  background: var(--primary);
  color: var(--bg);
}
</style>
