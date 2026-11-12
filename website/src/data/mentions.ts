// Places openEHR Explorer has been announced or listed. Single source of
// truth for the landing page's "As seen in" section and the footer links.
// Entries without a `url` render as a muted "coming soon" placeholder.
// Keep newest first.
export interface Mention {
  name: string;
  /** Short caption under the name. */
  note: string;
  url?: string;
  /** Footer link label, if this mention is also linked from the footer. */
  footerLabel?: string;
}

export const mentions: Mention[] = [
  {
    name: "openEHR Blog",
    note: "Coming soon",
  },
  {
    name: "openEHR Developer",
    note: "Listed under query tools",
    url: "https://developer.openehr.org/tooling/query-tools/#openehr-explorer",
    footerLabel: "openEHR Developer Tool Listing",
  },
  {
    name: "openEHR Discourse",
    note: "Release announcement",
    url: "https://discourse.openehr.org/t/openehr-explorer-a-cross-platform-desktop-cdr-browser-macos-windows-linux/17229",
    footerLabel: "Announcement",
  },
];
