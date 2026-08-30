<script setup lang="ts">
// Fullscreen screenshot gallery: click (or Enter/Space, via native
// <button> semantics) a gallery trigger to open it in a lightbox with
// prev/next navigation. Ported from the vanilla-JS `LightboxGallery`
// IIFE that used to live at the bottom of index.html, onto Vue's
// reactivity instead of manual DOM classList/attribute juggling.
import { ref } from "vue";

interface Screenshot {
  src: string;
  alt: string;
  caption: string;
  width: number;
  height: number;
}

defineProps<{ screenshots: Screenshot[] }>();

const dialog = ref<HTMLDialogElement | null>(null);
const closeBtn = ref<HTMLButtonElement | null>(null);
const currentIndex = ref(0);
let lastFocused: HTMLElement | null = null;

function open(index: number, screenshots: Screenshot[]) {
  lastFocused = document.activeElement as HTMLElement | null;
  currentIndex.value = ((index % screenshots.length) + screenshots.length) % screenshots.length;
  document.body.style.overflow = "hidden";
  dialog.value?.showModal();
  closeBtn.value?.focus();
}

function close() {
  dialog.value?.close();
}

function show(index: number, screenshots: Screenshot[]) {
  currentIndex.value = ((index % screenshots.length) + screenshots.length) % screenshots.length;
}

// Runs on every dialog close, whichever way it happened — a click on
// the close button, the backdrop, or the browser's own Escape handling
// — since all of them end up firing the dialog's native "close" event.
function handleClose() {
  document.body.style.overflow = "";
  lastFocused?.focus?.();
}

function handleBackdropClick(e: MouseEvent) {
  if (e.target === dialog.value) dialog.value?.close();
}

function handleKeydown(e: KeyboardEvent, screenshots: Screenshot[]) {
  // Escape is deliberately unhandled here — a native <dialog> opened via
  // showModal() already closes itself on Escape, firing "close" above.
  if (e.key === "ArrowRight") show(currentIndex.value + 1, screenshots);
  if (e.key === "ArrowLeft") show(currentIndex.value - 1, screenshots);
}
</script>

<template>
  <section class="gallery">
    <h2 class="section-heading">See it in action</h2>
    <div class="gallery-grid">
      <figure v-for="(shot, i) in screenshots" :key="shot.src" class="gallery-card">
        <button type="button" class="lightbox-trigger" aria-haspopup="dialog" @click="open(i, screenshots)">
          <img :src="shot.src" :alt="shot.alt" loading="lazy" :width="shot.width" :height="shot.height" />
        </button>
        <figcaption>{{ shot.caption }}</figcaption>
      </figure>
    </div>
  </section>

  <dialog
    class="lightbox"
    ref="dialog"
    aria-label="Screenshot preview"
    @click="handleBackdropClick"
    @keydown="handleKeydown($event, screenshots)"
    @close="handleClose"
  >
    <button type="button" class="lightbox-close" ref="closeBtn" aria-label="Close" @click="close">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
        <line x1="4" y1="4" x2="20" y2="20" /><line x1="20" y1="4" x2="4" y2="20" />
      </svg>
    </button>
    <button type="button" class="lightbox-nav lightbox-prev" aria-label="Previous screenshot" @click="show(currentIndex - 1, screenshots)">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="15 18 9 12 15 6" />
      </svg>
    </button>
    <figure class="lightbox-figure" v-if="screenshots.length">
      <img class="lightbox-img" :src="screenshots[currentIndex].src" :alt="screenshots[currentIndex].alt" />
      <figcaption class="lightbox-caption">{{ screenshots[currentIndex].caption }}</figcaption>
      <div class="lightbox-counter">{{ currentIndex + 1 }} / {{ screenshots.length }}</div>
    </figure>
    <button type="button" class="lightbox-nav lightbox-next" aria-label="Next screenshot" @click="show(currentIndex + 1, screenshots)">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="9 18 15 12 9 6" />
      </svg>
    </button>
  </dialog>
</template>

<style>
/* Unscoped: shared with the rest of index.astro's hand-authored CSS,
   ported verbatim from the old index.html <style> block. */
.gallery {
  padding: 0 24px 80px;
}
.section-heading {
  font-size: 2rem;
  font-weight: 600;
  text-align: center;
  margin-bottom: 48px;
  color: var(--text);
}
.gallery-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 24px;
  max-width: 1200px;
  margin: 0 auto;
}
.gallery-card {
  border: 1px solid var(--border);
  border-radius: 10px;
  overflow: hidden;
  background: var(--surface);
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.35);
  transition: border-color 0.2s;
}
.gallery-card:hover {
  border-color: var(--primary-dim);
}
.gallery-card img {
  display: block;
  width: 100%;
  height: auto;
  border-bottom: 1px solid var(--border);
}
.lightbox-trigger {
  display: block;
  width: 100%;
  padding: 0;
  border: none;
  background: none;
  cursor: zoom-in;
  font: inherit;
}
.lightbox-trigger:focus-visible {
  outline: 2px solid var(--primary);
  outline-offset: -2px;
}
.gallery-card figcaption {
  padding: 12px 16px;
  font-size: 0.85rem;
  color: var(--text-secondary);
}

.lightbox {
  position: fixed;
  inset: 0;
  z-index: 1000;
  margin: 0;
  padding: 24px;
  width: 100%;
  height: 100%;
  max-width: none;
  max-height: none;
  border: none;
  background: transparent;
  color: var(--text);
}
.lightbox[open] {
  display: flex;
  align-items: center;
  justify-content: center;
}
.lightbox::backdrop {
  background: rgba(10, 12, 20, 0.92);
}
.lightbox-figure {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  max-width: min(1400px, 92vw);
  max-height: 92vh;
}
.lightbox-img {
  display: block;
  max-width: 100%;
  max-height: 78vh;
  width: auto;
  height: auto;
  border-radius: 8px;
  border: 1px solid var(--border);
  box-shadow: 0 8px 60px rgba(0, 0, 0, 0.6);
}
.lightbox-caption {
  color: var(--text-secondary);
  font-size: 0.95rem;
  text-align: center;
  max-width: 800px;
}
.lightbox-counter {
  color: var(--text-muted);
  font-size: 0.85rem;
  font-family: var(--font-mono);
}
.lightbox-close,
.lightbox-nav {
  position: fixed;
  background: var(--surface);
  border: 1px solid var(--border);
  color: var(--text);
  border-radius: 50%;
  width: 44px;
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition:
    border-color 0.2s,
    color 0.2s;
}
.lightbox-close:hover,
.lightbox-nav:hover {
  border-color: var(--primary);
  color: var(--primary);
}
.lightbox-close {
  top: 20px;
  right: 20px;
}
.lightbox-nav {
  top: 50%;
  transform: translateY(-50%);
}
.lightbox-prev {
  left: 20px;
}
.lightbox-next {
  right: 20px;
}

@media (max-width: 640px) {
  .lightbox-nav {
    width: 36px;
    height: 36px;
  }
  .lightbox-prev {
    left: 8px;
  }
  .lightbox-next {
    right: 8px;
  }
  .lightbox-close {
    top: 8px;
    right: 8px;
  }
}
</style>
