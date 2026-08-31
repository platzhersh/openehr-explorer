import type { Meta, StoryObj } from "@storybook/vue3-vite";
import DocsSidebar from "./DocsSidebar.vue";

const meta: Meta<typeof DocsSidebar> = {
  title: "Website/DocsSidebar",
  component: DocsSidebar,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Reaches outside its own template into sibling `.docs-content section[id]` elements for search and active-section tracking (see the component's own comment) — this story renders a small fake content column alongside it so that behavior is actually demonstrable: try typing \"AQL\" into the search box, or scrolling the content pane.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof DocsSidebar>;

export const Default: Story = {
  args: {
    groups: [
      {
        title: "Getting Started",
        links: [
          { href: "#prerequisites", label: "Prerequisites" },
          { href: "#installation", label: "Installation" },
        ],
      },
      {
        title: "Features",
        links: [
          { href: "#aql-runner", label: "AQL Runner" },
          { href: "#flat-path-panel", label: "FLAT Path Panel" },
        ],
      },
    ],
  },
  render: (args) => ({
    components: { DocsSidebar },
    setup() {
      return { args };
    },
    template: `
      <div style="display: flex; height: 400px; font-family: var(--font-sans); color: var(--text); background: var(--bg);">
        <DocsSidebar v-bind="args" style="height: 100%;" />
        <main class="docs-content" style="flex: 1; overflow-y: auto; padding: 24px; max-width: 500px;">
          <section id="prerequisites">
            <h2 style="margin-bottom: 8px;">Prerequisites</h2>
            <p>openEHR Explorer is a native desktop application with no runtime dependencies.</p>
          </section>
          <section id="installation" style="margin-top: 32px;">
            <h2 style="margin-bottom: 8px;">Installation</h2>
            <p>Download the .dmg, .exe, .deb, or .AppImage from GitHub Releases.</p>
          </section>
          <section id="aql-runner" style="margin-top: 32px;">
            <h2 style="margin-bottom: 8px;">AQL Runner</h2>
            <p>Write and execute AQL queries with syntax highlighting and autocomplete.</p>
          </section>
          <section id="flat-path-panel" style="margin-top: 32px;">
            <h2 style="margin-bottom: 8px;">FLAT Path Panel</h2>
            <p>One-click copy of any FLAT path from the Web Template tree.</p>
          </section>
        </main>
      </div>
    `,
  }),
};
