// Fixed-row-height list virtualization for a self-scrolling container.
// Renders only the rows near the viewport (plus overscan) instead of the
// full list, so long lists — thousands of OPT XML lines, hundreds/thousands
// of stored AQL queries — don't blow up the DOM node count and freeze the
// app. See src/lib/virtualList.ts for the pure range math this wires up to
// a real scroll container, and OEH-XX / the perf ADR for why this exists.
import { computed, onMounted, onUnmounted, ref, type Ref } from "vue";
import { computeVirtualRange, virtualScrollOffset } from "../lib/virtualList";

export interface UseVirtualListOptions {
  /** Fixed height of every row, in px — must match the row's actual rendered height. */
  rowHeight: number;
  overscan?: number;
}

// `containerRef` is created by the caller (rather than by this composable)
// and bound to the scroll container via `ref="..."` in the caller's own
// template — mirrors useCodeMirror.ts's `container` param. A ref returned
// from a composable and re-exported under a new name isn't reliably
// recognized by vue-tsc's template-ref usage analysis, which otherwise
// flags it as an unused local (noUnusedLocals).
export function useVirtualList<T>(
  items: Ref<T[]>,
  containerRef: Ref<HTMLElement | null>,
  options: UseVirtualListOptions,
) {
  const scrollTop = ref(0);
  const viewportHeight = ref(0);

  let resizeObserver: ResizeObserver | null = null;
  let rafId = 0;

  function onScroll() {
    if (rafId) return;
    rafId = requestAnimationFrame(() => {
      rafId = 0;
      scrollTop.value = containerRef.value?.scrollTop ?? 0;
    });
  }

  onMounted(() => {
    const el = containerRef.value;
    viewportHeight.value = el?.clientHeight ?? 0;
    if (el) {
      resizeObserver = new ResizeObserver(() => {
        viewportHeight.value = el.clientHeight;
      });
      resizeObserver.observe(el);
    }
  });

  onUnmounted(() => {
    resizeObserver?.disconnect();
    if (rafId) cancelAnimationFrame(rafId);
  });

  const range = computed(() =>
    computeVirtualRange({
      scrollTop: scrollTop.value,
      viewportHeight: viewportHeight.value,
      rowHeight: options.rowHeight,
      itemCount: items.value.length,
      overscan: options.overscan,
    }),
  );

  const startIndex = computed(() => range.value.startIndex);
  const endIndex = computed(() => range.value.endIndex);
  const visibleItems = computed(() =>
    items.value.slice(range.value.startIndex, range.value.endIndex),
  );

  /** Scrolls so that row `index` is in view (optionally centered). */
  function scrollToIndex(index: number, opts?: { center?: boolean; behavior?: ScrollBehavior }) {
    const el = containerRef.value;
    if (!el) return;
    const top = virtualScrollOffset(index, options.rowHeight, viewportHeight.value, opts?.center);
    el.scrollTo({ top, behavior: opts?.behavior ?? "auto" });
  }

  return {
    onScroll,
    startIndex,
    endIndex,
    topPadding: computed(() => range.value.topPadding),
    bottomPadding: computed(() => range.value.bottomPadding),
    visibleItems,
    scrollToIndex,
  };
}
