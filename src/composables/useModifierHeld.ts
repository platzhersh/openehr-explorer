import { onMounted, onUnmounted, ref } from "vue";

type KeyTarget = Pick<Window, "addEventListener" | "removeEventListener">;

/**
 * Framework-free core: calls `onChange(true)` once Cmd (macOS) / Ctrl
 * (elsewhere) has been held for `delayMs`, and `onChange(false)` on keyup or
 * window blur (so it can't stick after Cmd-Tab). The delay keeps ordinary
 * Cmd+key use from flashing the shortcut hints. Returns a stop function.
 */
export function trackModifierHeld(
  target: KeyTarget,
  isMac: boolean,
  delayMs: number,
  onChange: (held: boolean) => void,
): () => void {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let held = false;

  const isModifier = (e: KeyboardEvent) => e.key === (isMac ? "Meta" : "Control");
  const reset = () => {
    if (timer) clearTimeout(timer);
    timer = null;
    if (held) {
      held = false;
      onChange(false);
    }
  };
  const onKeydown = (e: KeyboardEvent) => {
    if (!isModifier(e) || e.repeat || held || timer) return;
    timer = setTimeout(() => {
      timer = null;
      held = true;
      onChange(true);
    }, delayMs);
  };
  const onKeyup = (e: KeyboardEvent) => {
    if (isModifier(e)) reset();
  };

  target.addEventListener("keydown", onKeydown as EventListener);
  target.addEventListener("keyup", onKeyup as EventListener);
  target.addEventListener("blur", reset);
  return () => {
    reset();
    target.removeEventListener("keydown", onKeydown as EventListener);
    target.removeEventListener("keyup", onKeyup as EventListener);
    target.removeEventListener("blur", reset);
  };
}

/** Vue binding of `trackModifierHeld` for the window. */
export function useModifierHeld(isMac: boolean, delayMs = 400) {
  const held = ref(false);
  let stop: (() => void) | null = null;
  onMounted(() => {
    stop = trackModifierHeld(window, isMac, delayMs, (v) => (held.value = v));
  });
  onUnmounted(() => stop?.());
  return { held };
}
