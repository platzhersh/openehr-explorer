// Stubs the Tauri v2 IPC boundary (window.__TAURI_INTERNALS__.invoke) with
// realistic canned fixture data, so the real Vue frontend can be driven in a
// plain browser (Playwright/Chromium) for screenshot/video generation — no
// Rust backend, no EHRBase, no native Tauri shell required. Injected via
// page.addInitScript() before any app script runs.
//
// This is the single source of truth for the fixture data used by every
// marketing asset on the landing page (static screenshots, the hero video,
// and the README gif) — see capture-screenshots.js and record-video.js in
// this directory. Keeping them all fed by the same fixtures is what makes
// the EHR IDs, clinician names, and template structure line up across every
// asset instead of looking like unrelated demo data.
(function () {
  "use strict";

  // Identifiers/timestamps/RM-type strings repeated across the fixtures
  // below — named once here rather than copy-pasted at each use site.
  const SYSTEM_ID = "hospital.example.org";
  const VITAL_SIGNS_TEMPLATE_ID = "vital_signs.v1";
  const VITAL_SIGNS_NAME = "Vital Signs";
  const VITAL_SIGNS_TIME = "2026-08-20T09:15:00Z";
  const DR_KESSLER = "Dr. A. Kessler";
  const ARCHETYPE_CREATED = "2026-01-14T00:00:00Z";
  const RM_ELEMENT = "ELEMENT";
  const RM_DV_QUANTITY = "DV_QUANTITY";

  // ---- in-memory "server" state, mutated by the mocked commands ----
  const state = {
    profiles: [],
  };

  // The EHR the demo always drills into — referenced from EHR_DETAILS,
  // AQL_RESULT, and the exported __DEMO_FIXTURES__ below.
  const PRIMARY_EHR_ID = "9c2c5c5e-1c1b-4b0a-9d1e-9a4b2f3c8a11";
  const SECOND_EHR_ID = "4a9e2d7b-6f31-4c9e-8a5a-1d2e3f4a5b6c";
  const THIRD_EHR_ID = "b28f6a9c-2e4d-4a1b-9c3f-7e8d9c0b1a2f";

  const EHRS = [
    { ehr_id: PRIMARY_EHR_ID, system_id: SYSTEM_ID, time_created: "2026-06-02T08:14:21Z", subject_id: null },
    { ehr_id: SECOND_EHR_ID, system_id: SYSTEM_ID, time_created: "2026-08-19T10:05:17Z", subject_id: null },
    { ehr_id: THIRD_EHR_ID, system_id: SYSTEM_ID, time_created: "2026-08-11T16:38:44Z", subject_id: null },
  ];

  const VITAL_SIGNS_COMPOSITION_UID = `c1a2b3c4-0001-4a1b-9c3f-000000000001::${SYSTEM_ID}::1`;

  const EHR_DETAILS = {
    [PRIMARY_EHR_ID]: {
      ehr_id: PRIMARY_EHR_ID,
      system_id: SYSTEM_ID,
      time_created: "2026-06-02T08:14:21Z",
      is_modifiable: true,
      is_queryable: true,
      subject_id: "P-10234",
      subject_namespace: "uk.nhs.nhs_number",
      compositions: [
        { uid: VITAL_SIGNS_COMPOSITION_UID, template_id: VITAL_SIGNS_TEMPLATE_ID, name: VITAL_SIGNS_NAME, composer: DR_KESSLER, time_committed: VITAL_SIGNS_TIME },
        { uid: `c1a2b3c4-0002-4a1b-9c3f-000000000002::${SYSTEM_ID}::1`, template_id: "IDCR - Problem List.v1", name: "Problem List", composer: DR_KESSLER, time_committed: "2026-08-18T13:42:10Z" },
        { uid: `c1a2b3c4-0003-4a1b-9c3f-000000000003::${SYSTEM_ID}::1`, template_id: "IDCR - Medication List.v1", name: "Medication List", composer: "Dr. R. Okafor", time_committed: "2026-08-12T10:03:44Z" },
      ],
    },
  };

  const TEMPLATES = [
    { template_id: VITAL_SIGNS_TEMPLATE_ID, concept: VITAL_SIGNS_NAME, archetype_id: "openEHR-EHR-COMPOSITION.encounter.v1", created_timestamp: ARCHETYPE_CREATED },
    { template_id: "IDCR - Problem List.v1", concept: "Problem List", archetype_id: "openEHR-EHR-COMPOSITION.problem_list.v1", created_timestamp: ARCHETYPE_CREATED },
    { template_id: "IDCR - Medication List.v1", concept: "Medication List", archetype_id: "openEHR-EHR-COMPOSITION.medication_list.v1", created_timestamp: ARCHETYPE_CREATED },
    { template_id: "IDCR - Discharge Summary.v1", concept: "Discharge Summary", archetype_id: "openEHR-EHR-COMPOSITION.discharge_summary.v1", created_timestamp: ARCHETYPE_CREATED },
  ];

  // Web Template tree for "vital_signs.v1" — mirrors the real archetype
  // shape (Vital Signs OBSERVATION, an Any Event slot with the usual
  // biometrics) so the AQL Layer-3 path autocomplete has real paths to
  // offer once this template is picked as AQL context, and so the
  // CompositionTree/OPT-tree views have real labels to resolve at-codes to.
  const VITAL_SIGNS_ARCHETYPE = "openEHR-EHR-OBSERVATION.vital_signs.v2";
  function vsPath(field) {
    return `/content[${VITAL_SIGNS_ARCHETYPE}]/data[at0001]/events[at0006]/data[at0003]/items[${field}]/value`;
  }
  // One entry per vital-sign field, shared between the Web Template tree
  // (below) and the raw composition/FLAT fixtures (further down) so the
  // archetype code, label, magnitude, and unit for e.g. "Systolic" are
  // only ever written once.
  const VITAL_SIGNS_FIELDS = [
    { code: "at0004", flatKey: "systolic", label: "Systolic", magnitude: 128, units: "mm[Hg]" },
    { code: "at0005", flatKey: "diastolic", label: "Diastolic", magnitude: 82, units: "mm[Hg]" },
    { code: "at0006", flatKey: "pulse_heart_rate", label: "Pulse/Heart rate", magnitude: 76, units: "/min" },
    { code: "at0007", flatKey: "respiration_rate", label: "Respiration rate", magnitude: 16, units: "/min" },
    { code: "at0008", flatKey: "spo2", label: "SpO2", magnitude: 98, units: "%" },
    { code: "at0009", flatKey: "body_temperature", label: "Body temperature", magnitude: 36.8, units: "°C" },
  ];

  const WEB_TEMPLATES = {
    [VITAL_SIGNS_TEMPLATE_ID]: {
      templateId: VITAL_SIGNS_TEMPLATE_ID,
      version: "1.0.0",
      tree: {
        id: "vital_signs",
        name: VITAL_SIGNS_NAME,
        rmType: "COMPOSITION",
        aqlPath: "",
        children: [
          {
            id: "context",
            name: "Context",
            rmType: "EVENT_CONTEXT",
            aqlPath: "/context",
            children: [
              { id: "start_time", name: "Start time", rmType: "DV_DATE_TIME", aqlPath: "/context/start_time", children: [] },
              { id: "setting", name: "Setting", rmType: "DV_CODED_TEXT", aqlPath: "/context/setting", children: [] },
            ],
          },
          {
            id: VITAL_SIGNS_ARCHETYPE,
            name: VITAL_SIGNS_NAME,
            rmType: "OBSERVATION",
            aqlPath: `/content[${VITAL_SIGNS_ARCHETYPE}]`,
            children: VITAL_SIGNS_FIELDS.map(function (field) {
              return { id: field.flatKey, name: field.label, rmType: RM_DV_QUANTITY, aqlPath: vsPath(field.code), children: [] };
            }),
          },
        ],
      },
    },
  };

  const TEMPLATE_OPT = {
    [VITAL_SIGNS_TEMPLATE_ID]: `<?xml version="1.0" encoding="UTF-8"?>\n<template xmlns="openEHR/v1/Template">\n  <template_id><value>${VITAL_SIGNS_TEMPLATE_ID}</value></template_id>\n  <concept><value>${VITAL_SIGNS_NAME}</value></concept>\n</template>\n`,
  };

  function vitalSignsElement(field) {
    return {
      _type: RM_ELEMENT,
      archetype_node_id: field.code,
      name: { value: field.label },
      value: { _type: RM_DV_QUANTITY, magnitude: field.magnitude, units: field.units },
    };
  }

  // Raw canonical-JSON composition (RM-shaped) + its FLAT-format sibling,
  // for the same "Vital Signs" composition referenced from EHR_DETAILS
  // above. Values match 1:1 across both representations (and against
  // VITAL_SIGNS_FIELDS/the Web Template paths above), so the Pretty/JSON/
  // FLAT tabs in the Composition Viewer all show the same numbers.
  const VITAL_SIGNS_COMPOSITION = {
    _type: "COMPOSITION",
    name: { value: VITAL_SIGNS_NAME },
    archetype_details: {
      archetype_id: { value: "openEHR-EHR-COMPOSITION.encounter.v1" },
      template_id: { value: VITAL_SIGNS_TEMPLATE_ID },
    },
    composer: { name: DR_KESSLER },
    context: {
      start_time: { value: VITAL_SIGNS_TIME },
      setting: {
        value: "other care",
        defining_code: { terminology_id: { value: "openehr" }, code_string: "238" },
      },
    },
    content: [
      {
        _type: "OBSERVATION",
        archetype_node_id: VITAL_SIGNS_ARCHETYPE,
        name: { value: VITAL_SIGNS_NAME },
        data: {
          _type: "HISTORY",
          origin: { value: VITAL_SIGNS_TIME },
          events: [
            {
              _type: "POINT_EVENT",
              archetype_node_id: "at0006",
              name: { value: "Any event" },
              time: { value: VITAL_SIGNS_TIME },
              data: {
                _type: "ITEM_TREE",
                items: VITAL_SIGNS_FIELDS.map(vitalSignsElement),
              },
            },
          ],
        },
      },
    ],
  };

  const VITAL_SIGNS_COMPOSITION_FLAT = Object.assign(
    {
      "vital_signs/context/start_time": VITAL_SIGNS_TIME,
      "vital_signs/context/setting|code": "238",
      "vital_signs/context/setting|value": "other care",
      "vital_signs/context/setting|terminology": "openehr",
    },
    ...VITAL_SIGNS_FIELDS.map(function (field) {
      const prefix = `vital_signs/vital_signs/any_event:0/${field.flatKey}`;
      return { [`${prefix}|magnitude`]: field.magnitude, [`${prefix}|unit`]: field.units };
    }),
  );

  const COMPOSITIONS = {};
  COMPOSITIONS[VITAL_SIGNS_COMPOSITION_UID] = {
    composition: VITAL_SIGNS_COMPOSITION,
    flat: VITAL_SIGNS_COMPOSITION_FLAT,
  };

  const SAVED_QUERIES = [
    { id: "q1", name: "Vital Signs — last 7 days", query: `SELECT c/uid/value FROM EHR e CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1] WHERE c/archetype_details/template_id/value = '${VITAL_SIGNS_TEMPLATE_ID}'`, server_id: null, created_at: VITAL_SIGNS_TIME },
    { id: "q2", name: "Compositions per EHR", query: "SELECT e/ehr_id/value, COUNT(c) FROM EHR e CONTAINS COMPOSITION c", server_id: null, created_at: VITAL_SIGNS_TIME },
  ];

  const AQL_RESULT = {
    columns: [
      { name: "ehr_id", path: "/ehr_id/value" },
      { name: "systolic", path: vsPath("at0004") },
      { name: "diastolic", path: vsPath("at0005") },
      { name: "time_committed", path: "/context/start_time/value" },
    ],
    rows: [
      [PRIMARY_EHR_ID, 128, 82, VITAL_SIGNS_TIME],
      [SECOND_EHR_ID, 118, 76, "2026-08-19T10:05:17Z"],
      [THIRD_EHR_ID, 135, 88, "2026-08-11T16:38:44Z"],
    ],
    total_count: 3,
    execution_time_ms: 42,
  };

  function toPublicProfile(input) {
    const auth = input.auth_method || { type: "none" };
    const publicAuth = { type: auth.type };
    if (auth.type === "basic") {
      publicAuth.username = auth.username;
      publicAuth.has_password = !!auth.password;
    } else if (auth.type === "bearer") {
      publicAuth.has_token = !!auth.token;
    }
    return {
      id: input.id,
      name: input.name,
      base_url: input.base_url,
      server_type: input.server_type,
      auth_method: publicAuth,
      admin_auth_method: input.admin_auth_method ? { type: input.admin_auth_method.type } : null,
      terminology_url: input.terminology_url || null,
      credential_backend: "memory",
    };
  }

  // command name -> function(args) -> value | Promise<value>. Throw a
  // string to reject the invoke() call the way the real backend does
  // (Tauri surfaces `Err(String)` as the rejection reason).
  const handlers = {
    // -- boot --
    get_settings: function () {
      return {
        version: 1,
        terminology_server_url: null,
        check_updates_on_startup: false,
        analytics_enabled: false,
        analytics_consent_asked: true,
      };
    },
    get_app_version: function () {
      return "0.5.0";
    },
    "plugin:updater|check": function () {
      return null;
    },
    "plugin:aptabase|track_event": function () {
      return undefined;
    },
    "plugin:event|listen": function () {
      return Math.floor(Math.random() * 1e6);
    },
    "plugin:event|unlisten": function () {
      return undefined;
    },

    // -- servers --
    list_server_profiles: function () {
      return state.profiles.slice();
    },
    save_server_profile: function (args) {
      const input = args.profile;
      const idx = state.profiles.findIndex(function (p) { return p.id === input.id; });
      const pub = toPublicProfile(input);
      if (idx === -1) state.profiles.push(pub);
      else state.profiles[idx] = pub;
      return state.profiles.slice();
    },
    delete_server_profile: function (args) {
      state.profiles = state.profiles.filter(function (p) { return p.id !== args.id; });
      return state.profiles.slice();
    },
    test_unsaved_connection: function () {
      return "Connected successfully (HTTP 200)";
    },
    test_server_connection: function () {
      return "Connected successfully (HTTP 200)";
    },
    get_server_version: function () {
      return {
        server_version: null,
        ehrbase_version: "2.7.0",
        sdk_version: null,
        archie_version: null,
        jvm_version: null,
        os_version: null,
        postgres_version: null,
      };
    },
    get_credential_backend: function () {
      return "memory";
    },

    // -- ehrs --
    list_ehrs: function (args) {
      const offset = args.offset || 0;
      const limit = args.limit || 20;
      const page = EHRS.slice(offset, offset + limit);
      return { ehrs: page, total: page.length + offset, offset: offset, limit: limit };
    },
    get_ehr_detail: function (args) {
      const detail = EHR_DETAILS[args.ehrId];
      if (!detail) throw new Error("EHR not found");
      return detail;
    },

    // -- compositions --
    get_composition: function (args) {
      const entry = COMPOSITIONS[args.compositionUid];
      if (!entry) throw new Error("Composition not found");
      return entry.composition;
    },
    get_composition_flat: function (args) {
      const entry = COMPOSITIONS[args.compositionUid];
      if (!entry) throw new Error("Composition not found");
      return entry.flat;
    },

    // -- templates --
    list_templates: function () {
      return TEMPLATES.slice();
    },
    get_web_template: function (args) {
      const wt = WEB_TEMPLATES[args.templateId];
      if (!wt) throw new Error("Template not found: " + args.templateId);
      return wt;
    },
    get_template_opt: function (args) {
      return TEMPLATE_OPT[args.templateId] || "<template/>";
    },
    get_term_bindings: function () {
      return [];
    },

    // -- aql --
    list_saved_queries: function () {
      return SAVED_QUERIES.slice();
    },
    execute_aql: function () {
      return AQL_RESULT;
    },
  };

  function invoke(cmd, args) {
    const handler = handlers[cmd];
    if (!handler) {
      console.warn("[demo-mock] unhandled invoke:", cmd, args);
      return Promise.resolve(undefined);
    }
    try {
      return Promise.resolve(handler(args || {}));
    } catch (err) {
      // Real Tauri commands reject with a plain string (Rust's Err(String)
      // crosses the IPC boundary as-is) — the frontend does things like
      // `String(e)` expecting no "Error: " prefix. Handlers above throw
      // real Error objects (better practice, and what static analysis
      // wants), so unwrap .message here to keep the app-visible rejection
      // shape identical to the real backend's.
      return Promise.reject(err instanceof Error ? err.message : err);
    }
  }

  const callbacks = {};
  let nextCallbackId = 1;

  window.__TAURI_INTERNALS__ = {
    invoke: invoke,
    transformCallback: function (callback, once) {
      const id = nextCallbackId++;
      callbacks[id] = function (result) {
        if (once) delete callbacks[id];
        return callback && callback(result);
      };
      return id;
    },
    unregisterCallback: function (id) {
      delete callbacks[id];
    },
    convertFileSrc: function (filePath) {
      return filePath;
    },
  };

  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener: function () {},
  };

  // Exposed so capture-screenshots.js / record-video.js can reference the
  // fixture IDs (e.g. the EHR ID to click, the composition UID to open)
  // without duplicating them.
  window.__DEMO_FIXTURES__ = {
    primaryEhrId: PRIMARY_EHR_ID,
    vitalSignsCompositionUid: VITAL_SIGNS_COMPOSITION_UID,
    vitalSignsTemplateId: VITAL_SIGNS_TEMPLATE_ID,
    vitalSignsArchetypeId: VITAL_SIGNS_ARCHETYPE,
  };
})();
