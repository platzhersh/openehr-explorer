<script setup lang="ts">
// Fullscreen screenshot gallery: click (or Enter/Space, via native
// <button> semantics) a gallery trigger to open it in a lightbox with
// prev/next navigation. Ported from the vanilla-JS `LightboxGallery`
// IIFE that used to live at the bottom of index.html, onto Vue's
// reactivity instead of manual DOM classList/attribute juggling.
import { onBeforeUnmount, onMounted, ref } from "vue";

interface Screenshot {
  src: string;
  alt: string;
  caption: string;
  width: number;
  height: number;
}

const props = defineProps<{ screenshots: Screenshot[] }>();

const dialog = ref<HTMLDialogElement | null>(null);
const closeBtn = ref<HTMLButtonElement | null>(null);
const currentIndex = ref(0);
let lastFocused: HTMLElement | null = null;

// --- Carousel -------------------------------------------------------
// A native scroll-snap track: touch swipe, trackpad and keyboard scrolling
// come for free. We only add arrows, dots, and track the active slide.
const track = ref<HTMLElement | null>(null);
const slides = ref<HTMLElement[]>([]);
const activeSlide = ref(0);
const canPrev = ref(false);
const canNext = ref(true);

function updateScrollState() {
  const el = track.value;
  if (!el) return;
  const max = el.scrollWidth - el.clientWidth;
  canPrev.value = el.scrollLeft > 4;
  canNext.value = el.scrollLeft < max - 4;
  // Active slide = the one whose left edge is closest to the track's left edge.
  let best = 0;
  let bestDist = Infinity;
  slides.value.forEach((slide, i) => {
    const dist = Math.abs(slide.offsetLeft - el.offsetLeft - el.scrollLeft);
    if (dist < bestDist) {
      bestDist = dist;
      best = i;
    }
  });
  activeSlide.value = canNext.value ? best : props.screenshots.length - 1;
}

function scrollToSlide(index: number) {
  const el = track.value;
  const slide = slides.value[index];
  if (!el || !slide) return;
  const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  el.scrollTo({ left: slide.offsetLeft - el.offsetLeft, behavior: reduced ? "auto" : "smooth" });
}

function step(dir: 1 | -1) {
  const next = Math.min(Math.max(activeSlide.value + dir, 0), props.screenshots.length - 1);
  scrollToSlide(next);
}

let resizeObserver: ResizeObserver | null = null;
onMounted(() => {
  updateScrollState();
  if (track.value && "ResizeObserver" in window) {
    resizeObserver = new ResizeObserver(updateScrollState);
    resizeObserver.observe(track.value);
  }
});
onBeforeUnmount(() => resizeObserver?.disconnect());

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
    <div class="carousel" role="region" aria-roledescription="carousel" aria-label="Screenshots">
      <button
        type="button"
        class="carousel-nav carousel-prev"
        aria-label="Previous screenshots"
        :disabled="!canPrev"
        @click="step(-1)"
      >
        <svg aria-hidden="true" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="15 18 9 12 15 6" />
        </svg>
      </button>
      <div class="carousel-track" ref="track" @scroll.passive="updateScrollState">
        <figure
          v-for="(shot, i) in screenshots"
          :key="shot.src"
          class="gallery-card carousel-slide"
          ref="slides"
          role="group"
          aria-roledescription="slide"
          :aria-label="`${i + 1} of ${screenshots.length}`"
        >
          <button
            type="button"
            class="lightbox-trigger"
            :class="{ crop: shot.width / shot.height < 2 }"
            aria-haspopup="dialog"
            @click="open(i, screenshots)"
          >
            <img :src="shot.src" :alt="shot.alt" loading="lazy" :width="shot.width" :height="shot.height" />
          </button>
          <figcaption>{{ shot.caption }}</figcaption>
        </figure>
      </div>
      <button
        type="button"
        class="carousel-nav carousel-next"
        aria-label="Next screenshots"
        :disabled="!canNext"
        @click="step(1)"
      >
        <svg aria-hidden="true" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="9 18 15 12 9 6" />
        </svg>
      </button>
    </div>
    <div class="carousel-dots">
      <button
        v-for="(shot, i) in screenshots"
        :key="shot.src"
        type="button"
        class="carousel-dot"
        :class="{ active: i === activeSlide }"
        :aria-label="`Go to screenshot ${i + 1}`"
        :aria-current="i === activeSlide ? 'true' : undefined"
        @click="scrollToSlide(i)"
      ></button>
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
      <svg aria-hidden="true" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
        <line x1="4" y1="4" x2="20" y2="20" /><line x1="20" y1="4" x2="4" y2="20" />
      </svg>
    </button>
    <button type="button" class="lightbox-nav lightbox-prev" aria-label="Previous screenshot" @click="show(currentIndex - 1, screenshots)">
      <svg aria-hidden="true" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="15 18 9 12 15 6" />
      </svg>
    </button>
    <figure class="lightbox-figure" v-if="screenshots.length">
      <img class="lightbox-img" :src="screenshots[currentIndex].src" :alt="screenshots[currentIndex].alt" />
      <figcaption class="lightbox-caption">{{ screenshots[currentIndex].caption }}</figcaption>
      <div class="lightbox-counter">{{ currentIndex + 1 }} / {{ screenshots.length }}</div>
    </figure>
    <button type="button" class="lightbox-nav lightbox-next" aria-label="Next screenshot" @click="show(currentIndex + 1, screenshots)">
      <svg aria-hidden="true" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
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
.carousel {
  position: relative;
  max-width: 1200px;
  margin: 0 auto;
}
.carousel-track {
  display: flex;
  gap: 24px;
  overflow-x: auto;
  scroll-snap-type: x mandatory;
  scrollbar-width: none;
  padding: 4px 0 8px;
}
.carousel-track::-webkit-scrollbar {
  display: none;
}
.carousel-slide {
  flex: 0 0 calc((100% - 24px) / 2.15);
  scroll-snap-align: start;
  margin: 0;
  display: flex;
  flex-direction: column;
}
.carousel-slide .lightbox-trigger {
  position: relative;
  flex: none;
  aspect-ratio: 2 / 1;
  background: var(--bg);
  border-bottom: 1px solid var(--border);
}
.carousel-slide img {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: contain;
  border-bottom: none;
}
/* Tall screenshots fill the frame and are cropped from the bottom; the
   lightbox always shows the full image. */
.carousel-slide .lightbox-trigger.crop img {
  object-fit: cover;
  object-position: top left;
}
.carousel-slide figcaption {
  min-height: 3.6em;
}
.carousel-nav {
  position: absolute;
  top: calc(50% - 40px);
  z-index: 2;
  width: 44px;
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  border: 1px solid var(--border);
  background: var(--surface);
  color: var(--text);
  cursor: pointer;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.5);
  transition:
    border-color 0.2s,
    color 0.2s,
    opacity 0.2s;
}
.carousel-nav:hover:not(:disabled) {
  border-color: var(--primary);
  color: var(--primary);
}
.carousel-nav:focus-visible,
.carousel-dot:focus-visible {
  outline: 2px solid var(--primary);
  outline-offset: 2px;
}
.carousel-nav:disabled {
  opacity: 0;
  pointer-events: none;
}
.carousel-prev {
  left: -22px;
}
.carousel-next {
  right: -22px;
}
.carousel-dots {
  display: flex;
  justify-content: center;
  gap: 4px;
  margin-top: 20px;
}
.carousel-dot {
  width: 24px;
  height: 24px;
  padding: 0;
  border: none;
  background: none;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}
.carousel-dot::after {
  content: "";
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-muted);
  opacity: 0.5;
  transition:
    background 0.2s,
    opacity 0.2s,
    transform 0.2s;
}
.carousel-dot.active::after {
  background: var(--primary);
  opacity: 1;
  transform: scale(1.25);
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

@media (max-width: 900px) {
  .carousel-slide {
    flex-basis: 85%;
  }
  .carousel-prev {
    left: 6px;
  }
  .carousel-next {
    right: 6px;
  }
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
