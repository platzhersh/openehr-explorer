// Shared Storybook render scaffolding for views/components that read
// entirely from Pinia stores and Tauri `invoke()` calls rather than props
// (Dashboard.vue, ServerManager.vue, UpdateNotification.vue, …) — not used
// by the app itself. Colocated with `storybook-args.ts` / `storybook-
// fixtures.ts` rather than duplicated per `.stories.ts` file.

import { onUnmounted } from "vue";
import { createPinia, setActivePinia } from "pinia";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import type { InvokeArgs } from "@tauri-apps/api/core";

/**
 * Call from a story's own `setup()`: activates a fresh Pinia instance,
 * mocks the Tauri IPC boundary for the component's lifetime (unmocked on
 * unmount), then runs `setupStores` — for seeding whatever Pinia state
 * isn't itself the result of a mocked `invoke()` call (e.g. connection
 * status, which nothing invokes).
 *
 * `handleInvoke` mirrors `mockIPC`'s own callback signature. Throw a real
 * `Error` (never `Promise.reject`/`throw` a bare string) to simulate a
 * rejected `invoke()` call — the app-visible message is the same either
 * way (call sites do `String(e)`), and a bare string throw is what SonarCloud
 * flags as a reliability issue.
 */
export function mockTauriStores(
  handleInvoke: (cmd: string, payload?: InvokeArgs) => unknown,
  setupStores?: () => void,
) {
  setActivePinia(createPinia());
  mockIPC(handleInvoke);
  onUnmounted(() => clearMocks());
  setupStores?.();
}
