import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { onUnmounted } from "vue";
import { createPinia, setActivePinia } from "pinia";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import Dashboard from "./Dashboard.vue";
import { useServerStore, type ServerProfile, type ServerVersionInfo } from "../stores/server";
import type { DashboardCounts } from "../stores/dashboard";

const PROFILE: ServerProfile = {
  id: "profile-1",
  name: "EhrBase Sandkiste",
  base_url: "https://sandbox.ehrbase.org/ehrbase",
  server_type: "ehrbase",
  auth_method: { type: "basic", username: "ehrbase-user", has_password: true },
  admin_auth_method: null,
  terminology_url: null,
  credential_backend: "keychain",
  is_default: true,
};

// For EHRBase specifically, the backend mirrors ehrbase_version into
// server_version too (see get_server_version in server.rs) — that's the
// field Dashboard.vue's "Version" row actually reads.
const VERSION_INFO: ServerVersionInfo = {
  server_version: "2.33.0",
  ehrbase_version: "2.33.0",
  sdk_version: null,
  archie_version: null,
  jvm_version: null,
  os_version: null,
  postgres_version: null,
};

const COUNTS: DashboardCounts = {
  ehr_count: 1384,
  composition_count: 11415,
  template_count: 485,
};

interface StoryState {
  /** No server profiles configured at all — the "Welcome" onboarding card. */
  empty?: boolean;
  /** A profile exists but nothing is selected as active — the rarer of the two empty-state copies. */
  noActiveServer?: boolean;
  /** `get_dashboard_counts` rejects instead of resolving. */
  countsError?: string;
  /** `get_dashboard_counts` never resolves, to freeze the view in its loading state. */
  stayLoading?: boolean;
}

// Dashboard.vue reads everything from the `server`/`dashboard` Pinia stores
// and fetches live via Tauri `invoke()` in its own onMounted/watch hooks —
// there's nothing to pass as component props. Each story instead mocks the
// Tauri IPC boundary (same helper UpdateNotification.stories.ts uses) so
// those real invoke() calls resolve to canned data, and seeds the bits of
// store state (profiles, connection status) that aren't themselves the
// result of an invoke() call the component makes.
function withStores(state: StoryState = {}) {
  return () => ({
    components: { Dashboard },
    setup() {
      setActivePinia(createPinia());

      mockIPC((cmd) => {
        if (cmd === "get_dashboard_counts") {
          if (state.countsError) return Promise.reject(state.countsError);
          if (state.stayLoading) return new Promise(() => {}); // never resolves
          return COUNTS;
        }
        if (cmd === "get_server_version") {
          return VERSION_INFO;
        }
        if (cmd === "list_server_profiles") {
          return [];
        }
      });
      onUnmounted(() => clearMocks());

      if (!state.empty) {
        const serverStore = useServerStore();
        serverStore.profiles = [PROFILE];
        if (!state.noActiveServer) {
          serverStore.activeServerId = PROFILE.id;
          serverStore.connectionStatus[PROFILE.id] = "connected";
        }
      }

      return {};
    },
    template: `<Dashboard />`,
  });
}

const meta: Meta<typeof Dashboard> = {
  title: "Views/Dashboard",
  component: Dashboard,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    docs: {
      description: {
        component:
          "The Overview tab (OEH-17) — the app's landing view. Shows live EHR/composition/template counts (AQL `COUNT` queries, refetched fresh on every load/refresh, never cached) plus the connected server's info. Renders under `/dashboard`, which `/` redirects to.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof Dashboard>;

export const Populated: Story = {
  render: withStores(),
  parameters: {
    docs: {
      description: {
        story:
          "Connected server with live counts. Each stat card links through to the EHR Browser or Template Browser; the Connected Server card links to the Server Manager.",
      },
    },
  },
};

export const Loading: Story = {
  render: withStores({ stayLoading: true }),
  parameters: {
    docs: {
      description: {
        story:
          "Counts still in flight — stat cards dim and show a placeholder until the AQL queries return.",
      },
    },
  },
};

export const CountsFailed: Story = {
  render: withStores({
    countsError:
      'AQL error (HTTP 400): {"error":"Bad Request","message":"Not implemented: selecting the full EHR object (e)"}',
  }),
  parameters: {
    docs: {
      description: {
        story:
          "The AQL COUNT query failed — surfaced as an inline error banner rather than silently showing zeroes.",
      },
    },
  },
};

export const NoServerConfigured: Story = {
  render: withStores({ empty: true }),
  parameters: {
    docs: {
      description: {
        story:
          'First-run state: no server profiles exist yet. A welcoming card with a prominent call to action, rather than the same muted "no results" empty state used elsewhere in the app.',
      },
    },
  },
};

export const NoServerSelected: Story = {
  render: withStores({ noActiveServer: true }),
  parameters: {
    docs: {
      description: {
        story:
          "A profile exists but none is active — normally unreachable (the server store auto-selects one as soon as any profile is loaded), included for completeness.",
      },
    },
  },
};
