// Shared sample data for Storybook stories only — not used by the app
// itself. Colocated with `storybook-args.ts` rather than duplicated inline
// per `.stories.ts` file, since several stories (JsonViewer, CompositionTree)
// legitimately want the exact same "here's a realistic openEHR composition"
// fixture to demonstrate their JSON rendering.

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
