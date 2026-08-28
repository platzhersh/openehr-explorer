import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { onUnmounted } from "vue";
import { createPinia, setActivePinia } from "pinia";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { expect, userEvent, within } from "storybook/test";
import ServerManager from "./ServerManager.vue";
import type { ServerProfile, ServerVersionInfo } from "../stores/server";

const EHRBASE_PROFILE: ServerProfile = {
  id: "profile-ehrbase",
  name: "EhrBase Sandkiste",
  base_url: "https://sandbox.ehrbase.org/ehrbase",
  server_type: "ehrbase",
  auth_method: { type: "basic", username: "ehrbase-user", has_password: true },
  admin_auth_method: null,
  terminology_url: null,
  credential_backend: "encrypted_file",
  is_default: true,
};

const FERRO_PROFILE: ServerProfile = {
  id: "profile-ferro",
  name: "FerroEHR",
  base_url: "http://localhost:8080/ferroehr",
  server_type: "ferro_ehr",
  auth_method: { type: "basic", username: "ferro-user", has_password: true },
  admin_auth_method: null,
  terminology_url: null,
  credential_backend: "encrypted_file",
  is_default: false,
};

const INSECURE_PROFILE: ServerProfile = {
  id: "profile-insecure",
  name: "Remote Demo Server",
  base_url: "http://ehrbase.hospital-demo.example.com",
  server_type: "ehrbase",
  auth_method: { type: "none" },
  admin_auth_method: null,
  terminology_url: null,
  credential_backend: "os_keychain",
  is_default: true,
};

// Keyed by profile id so the mocked get_server_version handler can return
// the right version per card (ServerManager fetches one per profile on mount).
const VERSION_BY_PROFILE: Record<string, ServerVersionInfo> = {
  [EHRBASE_PROFILE.id]: {
    server_version: "2.33.0",
    ehrbase_version: "2.33.0",
    sdk_version: null,
    archie_version: null,
    jvm_version: null,
    os_version: null,
    postgres_version: null,
  },
  [FERRO_PROFILE.id]: {
    server_version: "4.0.6-rc1",
    ehrbase_version: null,
    sdk_version: null,
    archie_version: null,
    jvm_version: null,
    os_version: null,
    postgres_version: null,
  },
};

interface StoryState {
  profiles?: ServerProfile[];
  /** `test_server_connection` rejects instead of resolving, for the play-function stories. */
  testConnectionFails?: boolean;
}

// ServerManager.vue loads everything itself in onMounted (list_server_profiles,
// then get_server_version per profile) rather than reading from props, so
// each story mocks the Tauri IPC boundary — same helper Dashboard.stories.ts
// and UpdateNotification.stories.ts use — instead of pre-seeding the store
// directly (which loadProfiles() would just overwrite on mount anyway).
function withStores(state: StoryState = {}) {
  const profiles = state.profiles ?? [];
  return () => ({
    components: { ServerManager },
    setup() {
      setActivePinia(createPinia());

      mockIPC((cmd, payload) => {
        if (cmd === "list_server_profiles") return profiles;
        if (cmd === "get_server_version") {
          const profileId = (payload as { profileId?: string } | undefined)?.profileId;
          return profileId ? (VERSION_BY_PROFILE[profileId] ?? null) : null;
        }
        if (cmd === "test_server_connection") {
          if (state.testConnectionFails) {
            return Promise.reject("Connection failed: 401 Unauthorized");
          }
          return "Connected successfully (HTTP 200)";
        }
      });
      onUnmounted(() => clearMocks());

      return {};
    },
    template: `<ServerManager />`,
  });
}

const meta: Meta<typeof ServerManager> = {
  title: "Views/ServerManager",
  component: ServerManager,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    docs: {
      description: {
        component:
          "The Server Manager (/servers) — create, edit, test, and delete server profiles for EHRBase, Better Platform, FerroEHR, or any generic openEHR REST server. Version detection runs automatically for every profile on mount.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof ServerManager>;

export const Populated: Story = {
  render: withStores({ profiles: [EHRBASE_PROFILE, FERRO_PROFILE] }),
  parameters: {
    docs: {
      description: {
        story:
          "Two connected profiles. EhrBase Sandkiste is the default (and becomes active automatically); FerroEHR is not.",
      },
    },
  },
};

export const Empty: Story = {
  render: withStores({ profiles: [] }),
  parameters: {
    docs: {
      description: {
        story: "No server profiles configured yet.",
      },
    },
  },
};

export const InsecureHttpWarning: Story = {
  render: withStores({ profiles: [INSECURE_PROFILE] }),
  parameters: {
    docs: {
      description: {
        story:
          "A profile using plain HTTP against a non-local host shows the ⚠️ HTTP badge — credentials for this profile would be sent unencrypted.",
      },
    },
  },
};

export const TestConnectionSucceeded: Story = {
  render: withStores({ profiles: [EHRBASE_PROFILE, FERRO_PROFILE] }),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const testButtons = await canvas.findAllByRole("button", { name: "Test" });
    await userEvent.click(testButtons[0]);
    await expect(await canvas.findByText(/Connected successfully/i)).toBeInTheDocument();
  },
  parameters: {
    docs: {
      description: {
        story: 'Clicks "Test" on the first profile card and shows the resulting success message.',
      },
    },
  },
};

export const TestConnectionFailed: Story = {
  render: withStores({ profiles: [EHRBASE_PROFILE, FERRO_PROFILE], testConnectionFails: true }),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const testButtons = await canvas.findAllByRole("button", { name: "Test" });
    await userEvent.click(testButtons[0]);
    await expect(await canvas.findByText(/401 Unauthorized/i)).toBeInTheDocument();
  },
  parameters: {
    docs: {
      description: {
        story: 'Clicks "Test" on the first profile card and shows the resulting error message.',
      },
    },
  },
};
