import { onUnmounted, toValue, watch, type MaybeRefOrGetter } from "vue";

type KeyTarget = Pick<Window, "addEventListener" | "removeEventListener">;

// Currently-open dialogs, oldest first. Only the topmost reacts to Escape so
// a dialog stacked over another one doesn't close both with a single press.
const openHandlers: Array<() => void> = [];

/**
 * Framework-free core: calls `onEscape` when Escape is pressed anywhere on
 * `target` (not only while focus is inside the dialog), unless another
 * handler already called `preventDefault()` on the event or a later-opened
 * dialog is on top. Returns a stop function.
 */
export function trackEscapeKey(target: KeyTarget, onEscape: () => void): () => void {
  const onKeydown = (e: KeyboardEvent) => {
    if (e.key !== "Escape" || e.defaultPrevented) return;
    if (openHandlers[openHandlers.length - 1] !== onEscape) return;
    onEscape();
  };

  openHandlers.push(onEscape);
  target.addEventListener("keydown", onKeydown as EventListener);
  return () => {
    const index = openHandlers.lastIndexOf(onEscape);
    if (index !== -1) openHandlers.splice(index, 1);
    target.removeEventListener("keydown", onKeydown as EventListener);
  };
}

/**
 * Vue binding of `trackEscapeKey` for the window: the listener is attached
 * while `isOpen` is true and removed when it turns false or the component
 * unmounts. Pair it with `@keydown.esc.prevent` on the dialog markup — that
 * keeps the element keyboard-accessible for linters, and the `preventDefault`
 * stops this listener from closing the dialog a second time.
 */
export function useEscapeKey(isOpen: MaybeRefOrGetter<boolean>, close: () => void) {
  let stop: (() => void) | null = null;
  watch(
    () => toValue(isOpen),
    (open) => {
      stop?.();
      stop = open ? trackEscapeKey(window, close) : null;
    },
    { immediate: true },
  );
  onUnmounted(() => stop?.());
}
