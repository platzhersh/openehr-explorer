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

  // ---- in-memory "server" state, mutated by the mocked commands ----
  var state = {
    profiles: [],
  };

  var EHRS = [
    { ehr_id: "9c2c5c5e-1c1b-4b0a-9d1e-9a4b2f3c8a11", system_id: "hospital.example.org", time_created: "2026-06-02T08:14:21Z", subject_id: null },
    { ehr_id: "4a9e2d7b-6f31-4c9e-8a5a-1d2e3f4a5b6c", system_id: "hospital.example.org", time_created: "2026-08-19T10:05:17Z", subject_id: null },
    { ehr_id: "b28f6a9c-2e4d-4a1b-9c3f-7e8d9c0b1a2f", system_id: "hospital.example.org", time_created: "2026-08-11T16:38:44Z", subject_id: null },
  ];

  var VITAL_SIGNS_COMPOSITION_UID = "c1a2b3c4-0001-4a1b-9c3f-000000000001::hospital.example.org::1";

  var EHR_DETAILS = {
    "9c2c5c5e-1c1b-4b0a-9d1e-9a4b2f3c8a11": {
      ehr_id: "9c2c5c5e-1c1b-4b0a-9d1e-9a4b2f3c8a11",
      system_id: "hospital.example.org",
      time_created: "2026-06-02T08:14:21Z",
      is_modifiable: true,
      is_queryable: true,
      subject_id: "P-10234",
      subject_namespace: "uk.nhs.nhs_number",
      compositions: [
        { uid: VITAL_SIGNS_COMPOSITION_UID, template_id: "vital_signs.v1", name: "Vital Signs", composer: "Dr. A. Kessler", time_committed: "2026-08-20T09:15:00Z" },
        { uid: "c1a2b3c4-0002-4a1b-9c3f-000000000002::hospital.example.org::1", template_id: "IDCR - Problem List.v1", name: "Problem List", composer: "Dr. A. Kessler", time_committed: "2026-08-18T13:42:10Z" },
        { uid: "c1a2b3c4-0003-4a1b-9c3f-000000000003::hospital.example.org::1", template_id: "IDCR - Medication List.v1", name: "Medication List", composer: "Dr. R. Okafor", time_committed: "2026-08-12T10:03:44Z" },
      ],
    },
  };

  var TEMPLATES = [
    { template_id: "vital_signs.v1", concept: "Vital Signs", archetype_id: "openEHR-EHR-COMPOSITION.encounter.v1", created_timestamp: "2026-01-14T00:00:00Z" },
    { template_id: "IDCR - Problem List.v1", concept: "Problem List", archetype_id: "openEHR-EHR-COMPOSITION.problem_list.v1", created_timestamp: "2026-01-14T00:00:00Z" },
    { template_id: "IDCR - Medication List.v1", concept: "Medication List", archetype_id: "openEHR-EHR-COMPOSITION.medication_list.v1", created_timestamp: "2026-01-14T00:00:00Z" },
    { template_id: "IDCR - Discharge Summary.v1", concept: "Discharge Summary", archetype_id: "openEHR-EHR-COMPOSITION.discharge_summary.v1", created_timestamp: "2026-01-14T00:00:00Z" },
  ];

  // Web Template tree for "vital_signs.v1" — mirrors the real archetype
  // shape (Vital Signs OBSERVATION, an Any Event slot with the usual
  // biometrics) so the AQL Layer-3 path autocomplete has real paths to
  // offer once this template is picked as AQL context, and so the
  // CompositionTree/OPT-tree views have real labels to resolve at-codes to.
  var VITAL_SIGNS_ARCHETYPE = "openEHR-EHR-OBSERVATION.vital_signs.v2";
  function vsPath(field) {
    return "/content[openEHR-EHR-OBSERVATION.vital_signs.v2]/data[at0001]/events[at0006]/data[at0003]/items[" + field + "]/value";
  }
  var WEB_TEMPLATES = {
    "vital_signs.v1": {
      templateId: "vital_signs.v1",
      version: "1.0.0",
      tree: {
        id: "vital_signs",
        name: "Vital Signs",
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
            name: "Vital Signs",
            rmType: "OBSERVATION",
            aqlPath: "/content[openEHR-EHR-OBSERVATION.vital_signs.v2]",
            children: [
              { id: "systolic", name: "Systolic", rmType: "DV_QUANTITY", aqlPath: vsPath("at0004"), children: [] },
              { id: "diastolic", name: "Diastolic", rmType: "DV_QUANTITY", aqlPath: vsPath("at0005"), children: [] },
              { id: "pulse_heart_rate", name: "Pulse/Heart rate", rmType: "DV_QUANTITY", aqlPath: vsPath("at0006"), children: [] },
              { id: "respiration_rate", name: "Respiration rate", rmType: "DV_QUANTITY", aqlPath: vsPath("at0007"), children: [] },
              { id: "spo2", name: "SpO2", rmType: "DV_QUANTITY", aqlPath: vsPath("at0008"), children: [] },
              { id: "body_temperature", name: "Body temperature", rmType: "DV_QUANTITY", aqlPath: vsPath("at0009"), children: [] },
            ],
          },
        ],
      },
    },
  };

  var TEMPLATE_OPT = {
    "vital_signs.v1": '<?xml version="1.0" encoding="UTF-8"?>\n<template xmlns="openEHR/v1/Template">\n  <template_id><value>vital_signs.v1</value></template_id>\n  <concept><value>Vital Signs</value></concept>\n</template>\n',
  };

  // Raw canonical-JSON composition (RM-shaped) + its FLAT-format sibling,
  // for the same "Vital Signs" composition referenced from EHR_DETAILS
  // above. Values match 1:1 across both representations and against the
  // Web Template paths above, so the Pretty/JSON/FLAT tabs in the
  // Composition Viewer all show the same numbers.
  var VITAL_SIGNS_COMPOSITION = {
    _type: "COMPOSITION",
    name: { value: "Vital Signs" },
    archetype_details: {
      archetype_id: { value: "openEHR-EHR-COMPOSITION.encounter.v1" },
      template_id: { value: "vital_signs.v1" },
    },
    composer: { name: "Dr. A. Kessler" },
    context: {
      start_time: { value: "2026-08-20T09:15:00Z" },
      setting: {
        value: "other care",
        defining_code: { terminology_id: { value: "openehr" }, code_string: "238" },
      },
    },
    content: [
      {
        _type: "OBSERVATION",
        archetype_node_id: VITAL_SIGNS_ARCHETYPE,
        name: { value: "Vital Signs" },
        data: {
          _type: "HISTORY",
          origin: { value: "2026-08-20T09:15:00Z" },
          events: [
            {
              _type: "POINT_EVENT",
              archetype_node_id: "at0006",
              name: { value: "Any event" },
              time: { value: "2026-08-20T09:15:00Z" },
              data: {
                _type: "ITEM_TREE",
                items: [
                  { _type: "ELEMENT", archetype_node_id: "at0004", name: { value: "Systolic" }, value: { _type: "DV_QUANTITY", magnitude: 128, units: "mm[Hg]" } },
                  { _type: "ELEMENT", archetype_node_id: "at0005", name: { value: "Diastolic" }, value: { _type: "DV_QUANTITY", magnitude: 82, units: "mm[Hg]" } },
                  { _type: "ELEMENT", archetype_node_id: "at0006", name: { value: "Pulse/Heart rate" }, value: { _type: "DV_QUANTITY", magnitude: 76, units: "/min" } },
                  { _type: "ELEMENT", archetype_node_id: "at0007", name: { value: "Respiration rate" }, value: { _type: "DV_QUANTITY", magnitude: 16, units: "/min" } },
                  { _type: "ELEMENT", archetype_node_id: "at0008", name: { value: "SpO2" }, value: { _type: "DV_QUANTITY", magnitude: 98, units: "%" } },
                  { _type: "ELEMENT", archetype_node_id: "at0009", name: { value: "Body temperature" }, value: { _type: "DV_QUANTITY", magnitude: 36.8, units: "°C" } },
                ],
              },
            },
          ],
        },
      },
    ],
  };

  var VITAL_SIGNS_COMPOSITION_FLAT = {
    "vital_signs/context/start_time": "2026-08-20T09:15:00Z",
    "vital_signs/context/setting|code": "238",
    "vital_signs/context/setting|value": "other care",
    "vital_signs/context/setting|terminology": "openehr",
    "vital_signs/vital_signs/any_event:0/systolic|magnitude": 128,
    "vital_signs/vital_signs/any_event:0/systolic|unit": "mm[Hg]",
    "vital_signs/vital_signs/any_event:0/diastolic|magnitude": 82,
    "vital_signs/vital_signs/any_event:0/diastolic|unit": "mm[Hg]",
    "vital_signs/vital_signs/any_event:0/pulse_heart_rate|magnitude": 76,
    "vital_signs/vital_signs/any_event:0/pulse_heart_rate|unit": "/min",
    "vital_signs/vital_signs/any_event:0/respiration_rate|magnitude": 16,
    "vital_signs/vital_signs/any_event:0/respiration_rate|unit": "/min",
    "vital_signs/vital_signs/any_event:0/spo2|magnitude": 98,
    "vital_signs/vital_signs/any_event:0/spo2|unit": "%",
    "vital_signs/vital_signs/any_event:0/body_temperature|magnitude": 36.8,
    "vital_signs/vital_signs/any_event:0/body_temperature|unit": "°C",
  };

  var COMPOSITIONS = {};
  COMPOSITIONS[VITAL_SIGNS_COMPOSITION_UID] = {
    composition: VITAL_SIGNS_COMPOSITION,
    flat: VITAL_SIGNS_COMPOSITION_FLAT,
  };

  var SAVED_QUERIES = [
    { id: "q1", name: "Vital Signs — last 7 days", query: "SELECT c/uid/value FROM EHR e CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1] WHERE c/archetype_details/template_id/value = 'vital_signs.v1'", server_id: null, created_at: "2026-08-20T09:15:00Z" },
    { id: "q2", name: "Compositions per EHR", query: "SELECT e/ehr_id/value, COUNT(c) FROM EHR e CONTAINS COMPOSITION c", server_id: null, created_at: "2026-08-20T09:15:00Z" },
  ];

  var AQL_RESULT = {
    columns: [
      { name: "ehr_id", path: "/ehr_id/value" },
      { name: "systolic", path: vsPath("at0004") },
      { name: "diastolic", path: vsPath("at0005") },
      { name: "time_committed", path: "/context/start_time/value" },
    ],
    rows: [
      ["9c2c5c5e-1c1b-4b0a-9d1e-9a4b2f3c8a11", 128, 82, "2026-08-20T09:15:00Z"],
      ["4a9e2d7b-6f31-4c9e-8a5a-1d2e3f4a5b6c", 118, 76, "2026-08-19T10:05:17Z"],
      ["b28f6a9c-2e4d-4a1b-9c3f-7e8d9c0b1a2f", 135, 88, "2026-08-11T16:38:44Z"],
    ],
    total_count: 3,
    execution_time_ms: 42,
  };

  function toPublicProfile(input) {
    var auth = input.auth_method || { type: "none" };
    var publicAuth = { type: auth.type };
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
  var handlers = {
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
      var input = args.profile;
      var idx = state.profiles.findIndex(function (p) { return p.id === input.id; });
      var pub = toPublicProfile(input);
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
      var offset = args.offset || 0;
      var limit = args.limit || 20;
      var page = EHRS.slice(offset, offset + limit);
      return { ehrs: page, total: page.length + offset, offset: offset, limit: limit };
    },
    get_ehr_detail: function (args) {
      var detail = EHR_DETAILS[args.ehrId];
      if (!detail) throw "EHR not found";
      return detail;
    },

    // -- compositions --
    get_composition: function (args) {
      var entry = COMPOSITIONS[args.compositionUid];
      if (!entry) throw "Composition not found";
      return entry.composition;
    },
    get_composition_flat: function (args) {
      var entry = COMPOSITIONS[args.compositionUid];
      if (!entry) throw "Composition not found";
      return entry.flat;
    },

    // -- templates --
    list_templates: function () {
      return TEMPLATES.slice();
    },
    get_web_template: function (args) {
      var wt = WEB_TEMPLATES[args.templateId];
      if (!wt) throw "Template not found: " + args.templateId;
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
    var handler = handlers[cmd];
    if (!handler) {
      console.warn("[demo-mock] unhandled invoke:", cmd, args);
      return Promise.resolve(undefined);
    }
    try {
      return Promise.resolve(handler(args || {}));
    } catch (err) {
      return Promise.reject(err);
    }
  }

  var callbacks = {};
  var nextCallbackId = 1;

  window.__TAURI_INTERNALS__ = {
    invoke: invoke,
    transformCallback: function (callback, once) {
      var id = nextCallbackId++;
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
    primaryEhrId: "9c2c5c5e-1c1b-4b0a-9d1e-9a4b2f3c8a11",
    vitalSignsCompositionUid: VITAL_SIGNS_COMPOSITION_UID,
    vitalSignsTemplateId: "vital_signs.v1",
    vitalSignsArchetypeId: VITAL_SIGNS_ARCHETYPE,
  };
})();
