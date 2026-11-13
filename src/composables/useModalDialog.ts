import { nextTick, onBeforeUnmount, watch, type Ref } from "vue";

/**
 * Gives a `<dialog>` real modal behaviour: `showModal()` puts it in the
 * browser's top layer, makes the rest of the document inert (no keyboard
 * focus, no pointer events, hidden from assistive tech), traps Tab /
 * Shift+Tab inside the dialog, and wires up the native `cancel` (Escape)
 * and `close` events.
 *
 * The dialog is opened as soon as the template ref resolves to an element
 * (i.e. when its `v-if` turns true or its component mounts). The element
 * that had focus at that moment is remembered, and focus goes back to it
 * once the dialog goes away — whether it was closed via `dialog.close()` or
 * simply unmounted.
 *
 * Usage:
 *   const dialogRef = ref<HTMLDialogElement | null>(null);
 *   useModalDialog(dialogRef);
 *   <dialog ref="dialogRef" @cancel.prevent="..." @close="...">
 */
export function useModalDialog(dialogRef: Ref<HTMLDialogElement | null>) {
  let opener: HTMLElement | null = null;

  function restoreFocus() {
    const target = opener;
    opener = null;
    // The opener can be gone by now (e.g. a "Take the tour" button inside a
    // dialog that has since unmounted) — only restore to a live element.
    if (target?.isConnected) {
      void nextTick(() => target.focus());
    }
  }

  watch(
    dialogRef,
    (el, previous) => {
      if (el) {
        if (!el.open) {
          opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
          el.showModal();
        }
      } else if (previous) {
        restoreFocus();
      }
    },
    // "post": the element must be connected to the document before
    // showModal(), which throws on a detached dialog.
    { flush: "post" },
  );

  // A component that owns the dialog directly is torn down with its template
  // refs, and its watchers are stopped first — so the watcher above never
  // sees the `null`. Restore focus here for that case.
  onBeforeUnmount(restoreFocus);
}
