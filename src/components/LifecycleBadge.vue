<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  LIFECYCLE_DESCRIPTIONS,
  SPEC_LIFECYCLE_VALUES,
  classifyLifecycle,
  semanticToneFor,
  type SpecLifecycleValue,
} from "../lib/template-lifecycle";

const props = defineProps<{
  value: string | undefined | null;
}>();

const popoverOpen = ref(false);
const rootRef = ref<HTMLElement | null>(null);
const triggerRef = ref<HTMLElement | null>(null);
const popoverRef = ref<HTMLElement | null>(null);
const popoverTop = ref(0);
const popoverLeft = ref(0);

const VIEWPORT_MARGIN = 12;
const POPOVER_GAP = 8;
const ESTIMATED_POPOVER_WIDTH = 420;
const ESTIMATED_POPOVER_HEIGHT = 360;

const classification = computed(() => classifyLifecycle(props.value));

const displayValue = computed(() => {
  const raw = (props.value ?? "").trim();
  return raw.length > 0 ? raw : "(empty)";
});

const badgeToneClass = computed(() => {
  if (classification.value !== "spec") return "tone-warning";
  const normalised = (props.value ?? "").trim().toLowerCase() as SpecLifecycleValue;
  const tone = semanticToneFor(normalised);
  return `tone-${tone}`;
});

const specEntries = computed(() =>
  SPEC_LIFECYCLE_VALUES.map((key) => ({
    key,
    description: LIFECYCLE_DESCRIPTIONS[key],
    toneClass: `tone-${semanticToneFor(key)}`,
  })),
);

function updatePopoverPosition() {
  if (!triggerRef.value) return;
  const triggerRect = triggerRef.value.getBoundingClientRect();
  const viewportWidth = window.innerWidth;
  const viewportHeight = window.innerHeight;

  const popoverEl = popoverRef.value;
  const popoverWidth = popoverEl?.offsetWidth || ESTIMATED_POPOVER_WIDTH;
  const popoverHeight = popoverEl?.offsetHeight || ESTIMATED_POPOVER_HEIGHT;

  // Prefer aligning the popover's left edge to the trigger's left edge,
  // but clamp so it stays within the viewport.
  let left = triggerRect.left;
  const maxLeft = viewportWidth - popoverWidth - VIEWPORT_MARGIN;
  if (left > maxLeft) left = maxLeft;
  if (left < VIEWPORT_MARGIN) left = VIEWPORT_MARGIN;

  // Prefer placing below the trigger; flip above if there's not enough room.
  let top = triggerRect.bottom + POPOVER_GAP;
  const spaceBelow = viewportHeight - triggerRect.bottom - POPOVER_GAP - VIEWPORT_MARGIN;
  if (spaceBelow < popoverHeight) {
    const spaceAbove = triggerRect.top - POPOVER_GAP - VIEWPORT_MARGIN;
    if (spaceAbove > spaceBelow) {
      top = Math.max(VIEWPORT_MARGIN, triggerRect.top - POPOVER_GAP - popoverHeight);
    }
  }

  popoverTop.value = top;
  popoverLeft.value = left;
}

function onDocumentMouseDown(event: MouseEvent) {
  if (!popoverOpen.value) return;
  const target = event.target as Node | null;
  if (!target) return;
  const inRoot = rootRef.value?.contains(target);
  const inPopover = popoverRef.value?.contains(target);
  if (!inRoot && !inPopover) {
    closePopover();
  }
}

function onDocumentKeyDown(event: KeyboardEvent) {
  if (event.key === "Escape" && popoverOpen.value) {
    closePopover();
  }
}

function onWindowReposition() {
  if (popoverOpen.value) updatePopoverPosition();
}

function openPopover() {
  popoverOpen.value = true;
  document.addEventListener("mousedown", onDocumentMouseDown);
  document.addEventListener("keydown", onDocumentKeyDown);
  window.addEventListener("resize", onWindowReposition);
  window.addEventListener("scroll", onWindowReposition, true);
  // Position once with the estimate, then refine after the popover is rendered.
  updatePopoverPosition();
  nextTick(() => updatePopoverPosition());
}

function closePopover() {
  popoverOpen.value = false;
  document.removeEventListener("mousedown", onDocumentMouseDown);
  document.removeEventListener("keydown", onDocumentKeyDown);
  window.removeEventListener("resize", onWindowReposition);
  window.removeEventListener("scroll", onWindowReposition, true);
}

function togglePopover() {
  if (popoverOpen.value) {
    closePopover();
  } else {
    openPopover();
  }
}

onBeforeUnmount(() => {
  document.removeEventListener("mousedown", onDocumentMouseDown);
  document.removeEventListener("keydown", onDocumentKeyDown);
  window.removeEventListener("resize", onWindowReposition);
  window.removeEventListener("scroll", onWindowReposition, true);
});

async function openSpec() {
  await openUrl(
    "https://specifications.openehr.org/releases/AM/latest/AOM2.html#_aom_lifecycle_state_mappings",
  );
}

const warningTooltip =
  "This value is not part of the openEHR AOM 2 lifecycle vocabulary. It was likely set by an older authoring tool (e.g. Ocean Template Designer uses 'Initial'). The template is still usable.";
</script>

<template>
  <span ref="rootRef" class="lifecycle-badge-wrapper">
    <span class="badge lifecycle-badge" :class="badgeToneClass">
      {{ displayValue }}
      <span
        v-if="classification !== 'spec'"
        class="warn-icon"
        :title="warningTooltip"
        aria-label="Non-standard lifecycle value"
      >
        ⚠
      </span>
    </span>

    <button
      ref="triggerRef"
      type="button"
      class="info-btn"
      :class="{ active: popoverOpen }"
      :aria-expanded="popoverOpen"
      aria-label="About template lifecycle states"
      title="About template lifecycle states"
      @click="togglePopover"
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width="14"
        height="14"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <circle cx="12" cy="12" r="10" />
        <line x1="12" y1="16" x2="12" y2="12" />
        <line x1="12" y1="8" x2="12.01" y2="8" />
      </svg>
    </button>

    <Teleport to="body">
      <div
        v-if="popoverOpen"
        ref="popoverRef"
        class="lifecycle-popover"
        role="dialog"
        :style="{ top: `${popoverTop}px`, left: `${popoverLeft}px` }"
      >
        <div class="popover-header">
          <h4>Template Lifecycle States</h4>
          <button type="button" class="popover-close" aria-label="Close" @click="closePopover">
            ×
          </button>
        </div>
        <div class="popover-body">
          <p class="popover-intro">
            The lifecycle state records the governance or publication stage of the template as set
            by its author. It is a free-text string in the OPT XML and is
            <strong>not enforced by the CDR</strong> — any value may appear here. The openEHR AOM 2
            specification defines seven canonical values:
          </p>
          <table class="lifecycle-table">
            <thead>
              <tr>
                <th>Value</th>
                <th>Meaning</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="entry in specEntries" :key="entry.key">
                <td>
                  <span class="badge lifecycle-badge" :class="entry.toneClass">
                    {{ entry.key }}
                  </span>
                </td>
                <td>{{ entry.description }}</td>
              </tr>
            </tbody>
          </table>
          <div class="popover-note">
            Note: lifecycle state is informational metadata. CDRs typically do not enforce it — an
            OPT can be uploaded and used regardless of its lifecycle value.
          </div>
          <div class="popover-footer">
            <button type="button" class="learn-more-link" @click="openSpec">Learn more →</button>
          </div>
        </div>
      </div>
    </Teleport>
  </span>
</template>

<style scoped>
.lifecycle-badge-wrapper {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.lifecycle-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: 3px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  line-height: 1.4;
}

.tone-positive {
  background: #28a745;
  color: #fff;
}

.tone-accent {
  background: rgba(100, 255, 218, 0.15);
  color: #64ffda;
  border: 1px solid rgba(100, 255, 218, 0.4);
}

.tone-neutral {
  background: #6c757d;
  color: #fff;
}

.tone-muted {
  background: rgba(255, 193, 7, 0.15);
  color: #e0a800;
  border: 1px solid rgba(255, 193, 7, 0.35);
}

.tone-warning {
  background: rgba(255, 153, 0, 0.18);
  color: #ffae42;
  border: 1px solid rgba(255, 153, 0, 0.5);
}

.warn-icon {
  cursor: help;
  font-size: 12px;
  line-height: 1;
}

.info-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  padding: 0;
  background: none;
  border: none;
  border-radius: 50%;
  color: var(--color-text-muted);
  cursor: pointer;
  transition:
    color 0.15s,
    background 0.15s;
}

.info-btn:hover,
.info-btn.active {
  color: #64ffda;
  background: rgba(100, 255, 218, 0.1);
}

.learn-more-link {
  background: none;
  border: none;
  color: #64ffda;
  font-size: 12px;
  cursor: pointer;
  padding: 0;
  text-decoration: none;
}

.learn-more-link:hover {
  text-decoration: underline;
}
</style>

<!--
  The popover is teleported to <body>, so its styles cannot be scoped to this
  component. Keep these styles global and namespaced under .lifecycle-popover.
-->
<style>
.lifecycle-popover {
  position: fixed;
  z-index: 1000;
  width: min(420px, calc(100vw - 24px));
  max-height: min(70vh, 520px);
  overflow-y: auto;
  background: var(--color-surface, #1a1f24);
  border: 1px solid var(--color-border, #2a3138);
  border-radius: var(--radius, 6px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  font-size: 12px;
  color: var(--color-text, #e6edf3);
}

.lifecycle-popover .popover-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  border-bottom: 1px solid var(--color-border, #2a3138);
}

.lifecycle-popover .popover-header h4 {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text, #e6edf3);
}

.lifecycle-popover .popover-close {
  background: none;
  border: none;
  color: var(--color-text-muted, #8b98a5);
  font-size: 20px;
  line-height: 1;
  cursor: pointer;
  padding: 0 4px;
}

.lifecycle-popover .popover-close:hover {
  color: #64ffda;
}

.lifecycle-popover .popover-body {
  padding: 12px 14px;
}

.lifecycle-popover .popover-intro {
  margin: 0 0 10px 0;
  line-height: 1.5;
  color: var(--color-text-secondary, #c9d1d9);
}

.lifecycle-popover .lifecycle-table {
  width: 100%;
  border-collapse: collapse;
  margin-bottom: 10px;
}

.lifecycle-popover .lifecycle-table th,
.lifecycle-popover .lifecycle-table td {
  text-align: left;
  padding: 6px 8px;
  border-bottom: 1px solid var(--color-border, #2a3138);
  vertical-align: top;
}

.lifecycle-popover .lifecycle-table th {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  color: var(--color-text-muted, #8b98a5);
  letter-spacing: 0.04em;
}

.lifecycle-popover .lifecycle-table td {
  color: var(--color-text-secondary, #c9d1d9);
  line-height: 1.45;
}

.lifecycle-popover .lifecycle-table tr:last-child td {
  border-bottom: none;
}

.lifecycle-popover .lifecycle-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: 3px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  line-height: 1.4;
  white-space: nowrap;
}

.lifecycle-popover .lifecycle-badge.tone-positive {
  background: #28a745;
  color: #fff;
}

.lifecycle-popover .lifecycle-badge.tone-accent {
  background: rgba(100, 255, 218, 0.15);
  color: #64ffda;
  border: 1px solid rgba(100, 255, 218, 0.4);
}

.lifecycle-popover .lifecycle-badge.tone-neutral {
  background: #6c757d;
  color: #fff;
}

.lifecycle-popover .lifecycle-badge.tone-muted {
  background: rgba(255, 193, 7, 0.15);
  color: #e0a800;
  border: 1px solid rgba(255, 193, 7, 0.35);
}

.lifecycle-popover .popover-note {
  margin-top: 4px;
  padding: 8px 10px;
  font-size: 11px;
  line-height: 1.5;
  color: var(--color-text-muted, #8b98a5);
  background: rgba(255, 255, 255, 0.03);
  border-left: 2px solid rgba(100, 255, 218, 0.4);
  border-radius: 2px;
}

.lifecycle-popover .popover-footer {
  margin-top: 12px;
  display: flex;
  justify-content: flex-end;
}

.lifecycle-popover .learn-more-link {
  background: none;
  border: none;
  color: #64ffda;
  font-size: 12px;
  cursor: pointer;
  padding: 0;
  text-decoration: none;
}

.lifecycle-popover .learn-more-link:hover {
  text-decoration: underline;
}
</style>
