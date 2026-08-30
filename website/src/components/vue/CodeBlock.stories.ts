import type { Meta, StoryObj } from "@storybook/vue3-vite";
import CodeBlock from "./CodeBlock.vue";

const meta: Meta<typeof CodeBlock> = {
  title: "Website/CodeBlock",
  component: CodeBlock,
  tags: ["autodocs"],
};

export default meta;
type Story = StoryObj<typeof CodeBlock>;

export const SingleCommand: Story = {
  args: {
    segments: [{ cmd: "brew install --cask platzhersh/openehr-explorer/openehr-explorer" }],
  },
};

export const MultipleCommands: Story = {
  args: {
    segments: [
      { cmd: "scoop bucket add openehr-explorer https://github.com/platzhersh/scoop-openehr-explorer" },
      { cmd: "scoop install openehr-explorer" },
    ],
  },
};

export const CommentedGroups: Story = {
  args: {
    segments: [
      { comment: "# 1. Add the signing key" },
      { cmd: "curl -fsSL https://raw.githubusercontent.com/platzhersh/apt-openehr-explorer/main/openehr-explorer.gpg | sudo gpg --dearmor -o /usr/share/keyrings/openehr-explorer.gpg" },
      { comment: "# 2. Add the repository" },
      {
        cmd: 'echo "deb [signed-by=/usr/share/keyrings/openehr-explorer.gpg] https://raw.githubusercontent.com/platzhersh/apt-openehr-explorer/main stable main" | sudo tee /etc/apt/sources.list.d/openehr-explorer.list',
      },
      { comment: "# 3. Install" },
      { cmd: "sudo apt update && sudo apt install open-ehr-explorer" },
    ],
  },
};
