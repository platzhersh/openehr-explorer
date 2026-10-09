import { ref } from "vue";
import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { expect, userEvent, within } from "storybook/test";
import CollapsibleSection from "./CollapsibleSection.vue";

const meta: Meta<typeof CollapsibleSection> = {
  title: "Components/CollapsibleSection",
  component: CollapsibleSection,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Reusable disclosure: a toggle row with a rotating caret that shows/hides its content. Use it for any collapse/expand pattern (e.g. **Advanced settings** in the server form) so they all look and behave alike. Bind with `v-model:open`, or leave unbound for self-managed state. The body isn't rendered while collapsed.",
      },
    },
  },
  args: { title: "Advanced settings" },
  render: (args) => ({
    components: { CollapsibleSection },
    setup: () => ({ args }),
    template: `
      <div style="padding: 16px; max-width: 480px">
        <CollapsibleSection v-bind="args">
          <p style="margin: 0; font-size: 13px; color: var(--color-text-secondary)">
            Section content, only rendered while expanded.
          </p>
        </CollapsibleSection>
      </div>`,
  }),
};

export default meta;
type Story = StoryObj<typeof CollapsibleSection>;

export const Collapsed: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByRole("button", { name: /advanced settings/i })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
    await expect(canvas.queryByText(/section content/i)).toBeNull();
  },
};

export const Expanded: Story = {
  args: { open: true },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByRole("button", { name: /advanced settings/i })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    await expect(canvas.getByText(/section content/i)).toBeInTheDocument();
  },
};

export const TogglesOnClick: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const toggle = await canvas.findByRole("button", { name: /advanced settings/i });
    await userEvent.click(toggle);
    await expect(toggle).toHaveAttribute("aria-expanded", "true");
    await expect(canvas.getByText(/section content/i)).toBeInTheDocument();
    await userEvent.click(toggle);
    await expect(toggle).toHaveAttribute("aria-expanded", "false");
    await expect(canvas.queryByText(/section content/i)).toBeNull();
  },
};

export const KeyboardToggle: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const toggle = await canvas.findByRole("button", { name: /advanced settings/i });
    toggle.focus();
    await userEvent.keyboard("{Enter}");
    await expect(toggle).toHaveAttribute("aria-expanded", "true");
    await userEvent.keyboard(" ");
    await expect(toggle).toHaveAttribute("aria-expanded", "false");
  },
};

export const CustomTitleSlot: Story = {
  render: () => ({
    components: { CollapsibleSection },
    setup: () => ({ open: ref(true) }),
    template: `
      <div style="padding: 16px; max-width: 480px">
        <CollapsibleSection v-model:open="open">
          <template #title>More details <span class="badge">3</span></template>
          <p style="margin: 0; font-size: 13px">Body</p>
        </CollapsibleSection>
      </div>`,
  }),
};

export const WithSummary: Story = {
  args: { summary: "/rest/openehr/v1 (default)" },
  parameters: {
    docs: {
      description: {
        story:
          "`summary` shows the current value on the toggle row while collapsed, so the setting is visible without opening the section. It disappears when expanded.",
      },
    },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText("/rest/openehr/v1 (default)")).toBeInTheDocument();
    await userEvent.click(await canvas.findByRole("button", { name: /advanced settings/i }));
    await expect(canvas.queryByText("/rest/openehr/v1 (default)")).toBeNull();
  },
};

export const ControlledBodyStaysReferenced: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const toggle = await canvas.findByRole("button", { name: /advanced settings/i });
    // aria-controls always points at a real element, even while collapsed.
    const id = toggle.getAttribute("aria-controls");
    await expect(id).toBeTruthy();
    await expect(canvasElement.querySelector(`#${id}`)).not.toBeNull();
  },
};
