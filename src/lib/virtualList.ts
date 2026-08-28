// Pure fixed-row-height virtualization math, used by
// src/composables/useVirtualList.ts to keep long lists (OPT XML lines,
// stored AQL queries, ...) from rendering every row's DOM at once — the
// cause of the app freezing/leaking memory on large OPT XML documents
// (thousands of <span> per line, none of them ever removed from the DOM).
// Kept separate from the Vue composable so the range math is unit-testable
// without mounting anything, mirroring src/lib/xml.ts's split from
// XmlViewer.vue.

export interface VirtualRangeInput {
  /** Current scrollTop of the virtualized container, in px. */
  scrollTop: number;
  /** Visible height of the virtualized container, in px. */
  viewportHeight: number;
  /** Fixed height of every row, in px. */
  rowHeight: number;
  itemCount: number;
  /** Extra rows rendered above/below the visible window, to avoid blank flashes on fast scroll. */
  overscan?: number;
}

export interface VirtualRange {
  /** First rendered item index, inclusive. */
  startIndex: number;
  /** Last rendered item index, exclusive. */
  endIndex: number;
  /** Spacer height (px) to place before the rendered rows, standing in for the skipped items above. */
  topPadding: number;
  /** Spacer height (px) to place after the rendered rows, standing in for the skipped items below. */
  bottomPadding: number;
}

export function computeVirtualRange({
  scrollTop,
  viewportHeight,
  rowHeight,
  itemCount,
  overscan = 8,
}: VirtualRangeInput): VirtualRange {
  if (itemCount <= 0 || rowHeight <= 0) {
    return { startIndex: 0, endIndex: 0, topPadding: 0, bottomPadding: 0 };
  }

  const rawStart = Math.floor(scrollTop / rowHeight) - overscan;
  const startIndex = Math.min(Math.max(0, rawStart), itemCount - 1);

  const visibleRows = Math.ceil(viewportHeight / rowHeight) + overscan * 2;
  const endIndex = Math.min(itemCount, startIndex + Math.max(visibleRows, 1));

  return {
    startIndex,
    endIndex,
    topPadding: startIndex * rowHeight,
    bottomPadding: (itemCount - endIndex) * rowHeight,
  };
}

/** Scroll offset (px) that brings row `index` into view, optionally centered in the viewport. */
export function virtualScrollOffset(
  index: number,
  rowHeight: number,
  viewportHeight: number,
  center = false,
): number {
  let top = index * rowHeight;
  if (center) top -= viewportHeight / 2 - rowHeight / 2;
  return Math.max(0, top);
}
