import { nextTick, onMounted, ref } from "vue";
import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { expect, userEvent, within } from "storybook/test";
import ServerFormDialog from "./ServerFormDialog.vue";
import type { ServerProfile } from "../stores/server";
import { mockTauriStores } from "../lib/storybook-tauri";

const CADASTO_PROFILE: ServerProfile = {
  id: "profile-cadasto",
  name: "Cadasto (production)",
  base_url: "https://cdr.example.com",
  server_type: "generic",
  auth_method: { type: "none" },
  admin_auth_method: null,
  terminology_url: null,
  api_path_prefix: "/openehr/v1",
  credential_backend: "encrypted_file",
  is_default: false,
};

const API_ROOT_PROFILE: ServerProfile = {
  ...CADASTO_PROFILE,
  id: "profile-api-root",
  name: "API root in Base URL",
  base_url: "https://cdr.example.com/openehr/v1",
  api_path_prefix: "",
};

function render(profile: ServerProfile | null) {
  return () => ({
    components: { ServerFormDialog },
    setup() {
      mockTauriStores((cmd) => {
        if (cmd === "get_credential_backend") return "encrypted_file";
        if (cmd === "test_unsaved_connection") return "Connected successfully (HTTP 200)";
      });
      // The dialog (re)initialises its form when `open` flips false → true, so
      // mount it closed and open it on the next tick, as the app does.
      const open = ref(false);
      onMounted(async () => {
        await nextTick();
        open.value = true;
      });
      return { profile, open };
    },
    template: `<ServerFormDialog :open="open" :profile="profile" @close="open = false" />`,
  });
}

const meta: Meta<typeof ServerFormDialog> = {
  title: "Components/ServerFormDialog",
  component: ServerFormDialog,
  tags: ["autodocs"],
  parameters: {
    layout: "fullscreen",
    docs: {
      description: {
        component:
          "Create/edit dialog for a server profile. The optional **API Path Prefix** controls where the openEHR REST API lives relative to the Base URL (default `/rest/openehr/v1`), with a live preview of the resulting request URL.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof ServerFormDialog>;

export const NewProfileAdvancedCollapsed: Story = {
  render: render(null),
  parameters: {
    docs: {
      description: {
        story:
          "Default state: the API Path Prefix lives under a collapsed **Advanced settings** disclosure.",
      },
    },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const toggle = await canvas.findByRole("button", { name: /advanced settings/i });
    await expect(toggle).toHaveAttribute("aria-expanded", "false");
    await expect(canvas.queryByTestId("api-root-preview")).toBeNull();
  },
};

export const EscapeClosesDialog: Story = {
  render: render(null),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    // Escape inside the dialog closes it (keyboard counterpart of the overlay click).
    await userEvent.click(await canvas.findByLabelText("Name"));
    await userEvent.keyboard("{Escape}");
    await expect(canvas.queryByText("Add Server Profile")).toBeNull();
  },
};

export const NewProfileAdvancedExpanded: Story = {
  render: render(null),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(await canvas.findByRole("button", { name: /advanced settings/i }));
    await expect(await canvas.findByTestId("api-root-preview")).toHaveTextContent(
      "http://localhost:8080/ehrbase/rest/openehr/v1/ehr",
    );
  },
};

export const CustomPrefixForCadasto: Story = {
  render: render(CADASTO_PROFILE),
  parameters: {
    docs: {
      description: {
        story:
          "A Generic profile for a CDR that serves the API at `<server>/openehr/v1` — no `/rest/` segment. Because the profile overrides the default prefix, Advanced settings starts expanded.",
      },
    },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByTestId("api-root-preview")).toHaveTextContent(
      "https://cdr.example.com/openehr/v1/ehr",
    );
  },
};

export const PreviewFollowsInput: Story = {
  render: render(null),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(await canvas.findByRole("button", { name: /advanced settings/i }));
    const prefix = await canvas.findByLabelText("API Path Prefix (optional)");
    await userEvent.type(prefix, "openehr/v1");
    await expect(canvas.getByTestId("api-root-preview")).toHaveTextContent(
      "http://localhost:8080/ehrbase/openehr/v1/ehr",
    );
    await userEvent.clear(prefix);
    await userEvent.type(prefix, "/");
    await expect(canvas.getByTestId("api-root-preview")).toHaveTextContent(
      "http://localhost:8080/ehrbase/ehr",
    );
  },
};

export const EmptyPrefixShownAsSlash: Story = {
  render: render(API_ROOT_PROFILE),
  parameters: {
    docs: {
      description: {
        story:
          "A saved empty prefix (the Base URL already is the API root) is shown as `/`, not as the blank default state, so re-saving the profile doesn't reset it.",
      },
    },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByLabelText("API Path Prefix (optional)")).toHaveValue("/");
    await expect(canvas.getByTestId("api-root-preview")).toHaveTextContent(
      "https://cdr.example.com/openehr/v1/ehr",
    );
  },
};
