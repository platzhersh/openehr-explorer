import { useArgs } from "storybook/preview-api";

/**
 * Returns an event handler that writes a single value back into Storybook's
 * args via useArgs() — for wiring a v-model prop in a story's `render()` so
 * both the component and the Controls panel stay in sync, the same way a
 * real parent's own ref would.
 *
 * Must be called from the top level of `render(args)` itself, not from
 * inside Vue's `setup()` — useArgs() needs to run during the story's own
 * render, not a nested component lifecycle hook.
 */
export function updateArgOnEvent<K extends string>(key: K) {
  const [, updateArgs] = useArgs();
  return (value: unknown) => updateArgs({ [key]: value });
}
