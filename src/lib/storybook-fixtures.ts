// Shared sample data for Storybook stories only — not used by the app
// itself. Colocated with `storybook-args.ts` rather than duplicated inline
// per `.stories.ts` file, since several stories (JsonViewer, CompositionTree)
// legitimately want the exact same "here's a realistic openEHR composition"
// fixture to demonstrate their JSON rendering.

import type { ServerProfile, ServerVersionInfo } from "../stores/server";

/**
 * A realistic connected EHRBase profile — shared by Dashboard.stories.ts and
 * ServerManager.stories.ts, both of which want the same "here's a live
 * server" fixture rather than two near-identical copies.
 */
export const SAMPLE_EHRBASE_PROFILE: ServerProfile = {
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

// For EHRBase specifically, the backend mirrors ehrbase_version into
// server_version too (see get_server_version in server.rs) — that's the
// field the app's "Version" displays actually read.
export const SAMPLE_EHRBASE_VERSION: ServerVersionInfo = {
  server_version: "2.33.0",
  ehrbase_version: "2.33.0",
  sdk_version: null,
  archie_version: null,
  jvm_version: null,
  os_version: null,
  postgres_version: null,
};

/**
 * A trimmed but realistic openEHR-shaped composition fragment: an
 * OBSERVATION with a coded finding (DV_CODED_TEXT, external terminology)
 * and two DV_QUANTITY values.
 */
export const SAMPLE_COMPOSITION = {
  _type: "OBSERVATION",
  archetype_node_id: "openEHR-EHR-OBSERVATION.blood_pressure.v2",
  name: { value: "Blood pressure" },
  data: {
    _type: "HISTORY",
    origin: { value: "2026-08-27T09:00:00Z" },
    events: [
      {
        _type: "POINT_EVENT",
        archetype_node_id: "at0006",
        name: { value: "Any event" },
        data: {
          _type: "ITEM_TREE",
          items: [
            {
              _type: "ELEMENT",
              archetype_node_id: "at0004",
              name: { value: "Systolic" },
              value: { _type: "DV_QUANTITY", magnitude: 120, units: "mm[Hg]" },
            },
            {
              _type: "ELEMENT",
              archetype_node_id: "at0005",
              name: { value: "Diastolic" },
              value: { _type: "DV_QUANTITY", magnitude: 80, units: "mm[Hg]" },
            },
            {
              _type: "ELEMENT",
              archetype_node_id: "at1000",
              name: { value: "Position" },
              value: {
                _type: "DV_CODED_TEXT",
                value: "Standing",
                defining_code: {
                  terminology_id: { value: "SNOMED-CT" },
                  code_string: "10904000",
                },
              },
            },
          ],
        },
      },
    ],
  },
};
