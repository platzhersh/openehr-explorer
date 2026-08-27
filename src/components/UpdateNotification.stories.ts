import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { onUnmounted } from "vue";
import { createPinia, setActivePinia } from "pinia";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import type { Update } from "@tauri-apps/plugin-updater";
import UpdateNotification from "./UpdateNotification.vue";
import { useSettingsStore } from "../stores/settings";
import { useUpdateStore } from "../stores/update";

interface StoryUpdateState {
  version?: string;
  currentVersion?: string;
  downloading?: boolean;
  downloadedBytes?: number;
  totalBytes?: number;
  error?: string | null;
}

// UpdateNotification reads its state from the `settings` and `update` Pinia
// stores rather than props (see src/stores/update.ts), so there's nothing to
// pass via `args`. Each story instead seeds a fresh Pinia instance with the
// state it wants to display before mounting the component.
function withStores(state: StoryUpdateState) {
  return () => ({
    components: { UpdateNotification },
    setup() {
      setActivePinia(createPinia());

      // Storybook runs outside a Tauri runtime, but the rendered banner is
      // still clickable: "Download & Install" ends in a relaunch() call and
      // "What's new →" calls openUrl(), both of which invoke real Tauri IPC
      // commands. Intercept those with Tauri's own mock-IPC helper (the same
      // one it recommends for tests) instead of letting them hit a missing
      // backend and reject. For the changelog link specifically, open the
      // URL in a real browser tab — Storybook *is* a browser, so this is a
      // truer stand-in for what openUrl() does in the actual app than a
      // silent no-op.
      mockIPC((cmd, payload) => {
        if (cmd === "plugin:opener|open_url") {
          const url = (payload as { url?: string } | undefined)?.url;
          if (url) window.open(url, "_blank", "noopener,noreferrer");
          return null;
        }
        if (cmd === "plugin:process|restart") {
          return null;
        }
      });
      onUnmounted(() => clearMocks());

      const settingsStore = useSettingsStore();
      settingsStore.loaded = true;
      // Skip the real update check the component fires in onMounted — there's
      // no Tauri runtime backing `@tauri-apps/plugin-updater` in Storybook, so
      // we set the desired state directly instead of letting it run.
      settingsStore.settings.check_updates_on_startup = false;

      const updateStore = useUpdateStore();
      updateStore.update = {
        version: state.version ?? "1.4.0",
        currentVersion: state.currentVersion ?? "1.3.2",
        // Storybook runs outside a Tauri runtime, so the real Update
        // instance's downloadAndInstall isn't available. Stub it as a no-op
        // so clicking "Download & Install" in the Available story doesn't
        // throw "downloadAndInstall is not a function" — it just resolves
        // and the store's own error handling takes it from there.
        downloadAndInstall: async () => {},
      } as Update;
      updateStore.dismissed = false;
      updateStore.downloading = state.downloading ?? false;
      updateStore.downloadedBytes = state.downloadedBytes ?? 0;
      updateStore.totalBytes = state.totalBytes ?? 0;
      updateStore.error = state.error ?? null;

      return {};
    },
    template: `<UpdateNotification />`,
  });
}

const meta: Meta<typeof UpdateNotification> = {
  title: "Components/UpdateNotification",
  component: UpdateNotification,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Startup banner shown when a newer app version is available. Includes a 'What's new' link to the changelog so users can see what changed before updating.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof UpdateNotification>;

export const Available: Story = {
  render: withStores({ version: "1.4.0", currentVersion: "1.3.2" }),
  parameters: {
    docs: {
      description: {
        story:
          "**Later** doesn't decline the update — it just calls `updateStore.dismiss()`, which hides the banner for the rest of the session. That flag isn't persisted, so the banner comes back the next time an update check finds this (or a newer) version: on app restart, or from a manual 'Check for Updates' in Settings, since `checkForUpdates()` resets `dismissed` to `false` whenever it finds an available update. **Download & Install** starts the real download via the Tauri updater.",
      },
    },
  },
};

export const Downloading: Story = {
  render: withStores({ downloading: true, downloadedBytes: 4_500_000, totalBytes: 12_000_000 }),
};

export const DownloadFailed: Story = {
  render: withStores({ error: "Failed to download update: network error" }),
};
