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

  // DIRECTORY (EHR_STATUS FOLDER hierarchy) for the primary EHR — a root
  // folder with a couple of subfolders, each pointing back at one of the
  // compositions above via an OBJECT_REF, so DirectoryTree.vue has real
  // nested structure to render instead of an empty state.
  const DIRECTORY_BY_EHR = {
    [PRIMARY_EHR_ID]: {
      name: { value: "Patient Record" },
      uid: { value: `dir0001-0000-4a1b-9c3f-000000000000::${SYSTEM_ID}::1` },
      items: [],
      folders: [
        {
          name: { value: "Vitals" },
          items: [
            { id: { value: VITAL_SIGNS_COMPOSITION_UID }, namespace: SYSTEM_ID, type: "COMPOSITION" },
          ],
          folders: [],
        },
        {
          name: { value: "Problems & Medications" },
          items: [
            { id: { value: `c1a2b3c4-0002-4a1b-9c3f-000000000002::${SYSTEM_ID}::1` }, namespace: SYSTEM_ID, type: "COMPOSITION" },
            { id: { value: `c1a2b3c4-0003-4a1b-9c3f-000000000003::${SYSTEM_ID}::1` }, namespace: SYSTEM_ID, type: "COMPOSITION" },
          ],
          folders: [],
        },
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
    [VITAL_SIGNS_TEMPLATE_ID]: `<?xml version="1.0" encoding="UTF-8"?>\n<template xmlns="openEHR/v1/Template">\n  <template_id><value>${VITAL_SIGNS_TEMPLATE_ID}</value></template_id>\n  <concept><value>${VITAL_SIGNS_NAME}</value></concept>\n` +
      `  <description>\n` +
      `    <original_author id="name">${DR_KESSLER}</original_author>\n` +
      `    <original_author id="organisation">${SYSTEM_ID}</original_author>\n` +
      `    <lifecycle_state>published</lifecycle_state>\n` +
      `    <details>\n` +
      `      <language><terminology_id><value>ISO_639-1</value></terminology_id><code_string>en</code_string></language>\n` +
      `      <purpose>Records a routine set of vital signs observations during an encounter.</purpose>\n` +
      `    </details>\n` +
      `  </description>\n` +
      `</template>\n`,
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
    // `_type` on every nested RM object, as in real canonical JSON — the
    // Pretty tab only recurses into typed objects, so an untyped one would
    // render as "[object Object]".
    composer: { _type: "PARTY_IDENTIFIED", name: DR_KESSLER },
    context: {
      _type: "EVENT_CONTEXT",
      start_time: { _type: "DV_DATE_TIME", value: VITAL_SIGNS_TIME },
      setting: {
        _type: "DV_CODED_TEXT",
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
          origin: { _type: "DV_DATE_TIME", value: VITAL_SIGNS_TIME },
          events: [
            {
              _type: "POINT_EVENT",
              archetype_node_id: "at0006",
              name: { value: "Any event" },
              time: { _type: "DV_DATE_TIME", value: VITAL_SIGNS_TIME },
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

  // STORED_QUERY: query definitions registered server-side (distinct from
  // SAVED_QUERIES above, which are persisted locally). Keyed by
  // "qualified_query_name|version" for get_stored_query_definition lookups.
  const STORED_QUERY_NAME = "org.openehr::vital_signs_report";
  const STORED_QUERY_VERSION = "1.0.0";
  const STORED_QUERIES = [
    { qualified_query_name: STORED_QUERY_NAME, version: STORED_QUERY_VERSION, query_type: "AQL", saved_time: VITAL_SIGNS_TIME },
  ];
  const STORED_QUERY_DEFINITIONS = {
    [`${STORED_QUERY_NAME}|${STORED_QUERY_VERSION}`]: {
      qualified_query_name: STORED_QUERY_NAME,
      version: STORED_QUERY_VERSION,
      query_type: "AQL",
      q: `SELECT c/uid/value, ${vsPath("at0004")}/magnitude AS systolic FROM EHR e[ehr_id/value=$ehrId] CONTAINS COMPOSITION c CONTAINS OBSERVATION o[${VITAL_SIGNS_ARCHETYPE}]`,
      saved_time: VITAL_SIGNS_TIME,
    },
  };

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
      credential_backend: "os_keychain",
    };
  }

  // Just an opaque bookkeeping id for "plugin:event|listen" below — no
  // randomness needed (or wanted; a plain counter also keeps recordings
  // reproducible), so a counter instead of Math.random() avoids flagging
  // Sonar's PRNG-safety rule for something that was never a security
  // concern in the first place.
  let nextEventListenerId = 1;

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
      // Set by the capture scripts from package.json (see
      // capture-screenshots.js / record-video.js) so the sidebar shows the
      // version actually being released instead of a stale hardcoded one.
      return window.__DEMO_APP_VERSION__ || "0.0.0";
    },
    "plugin:updater|check": function () {
      return null;
    },
    "plugin:aptabase|track_event": function () {
      return undefined;
    },
    "plugin:event|listen": function () {
      return nextEventListenerId++;
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
      return "os_keychain";
    },

    // -- ehrs --
    list_ehrs: function (args) {
      const offset = args.offset || 0;
      const limit = args.limit || 20;
      const page = EHRS.slice(offset, offset + limit);
      return {
        ehrs: page,
        total: page.length + offset,
        offset: offset,
        limit: limit,
        sort_applied: true,
        has_more: offset + limit < EHRS.length,
      };
    },
    get_ehr_detail: function (args) {
      const detail = EHR_DETAILS[args.ehrId];
      if (!detail) throw new Error("EHR not found");
      return detail;
    },
    get_directory: function (args) {
      // Real backend returns `null` (not a rejection) when the EHR has no
      // DIRECTORY set — see the comment on useEhrStore().fetchDirectory().
      return DIRECTORY_BY_EHR[args.ehrId] || null;
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
    list_stored_queries: function () {
      return STORED_QUERIES.slice();
    },
    get_stored_query_definition: function (args) {
      const key = `${args.qualifiedQueryName}|${args.version || STORED_QUERY_VERSION}`;
      const def = STORED_QUERY_DEFINITIONS[key];
      if (!def) throw new Error("Stored query not found: " + args.qualifiedQueryName);
      return def;
    },
    execute_stored_query: function () {
      return AQL_RESULT;
    },

    // -- dashboard (OEH-17) --
    get_dashboard_counts: function () {
      const compositionCount = Object.keys(EHR_DETAILS).reduce(function (sum, ehrId) {
        return sum + EHR_DETAILS[ehrId].compositions.length;
      }, 0);
      return {
        ehr_count: EHRS.length,
        composition_count: compositionCount,
        template_count: TEMPLATES.length,
      };
    },
  };

  // ---- Request Inspector (ADR-0011) ----
  // The real backend emits a `cdr-inspector-entry` event for every HTTP call
  // it makes to the CDR. Mirror that here: each mocked command that would
  // hit the CDR logs the request the Rust side would have sent (method,
  // URL, headers, body) plus the fixture data it "got back", so the
  // Request Inspector drawer has a realistic log to show.
  const eventListeners = {};
  let nextRequestId = 1;
  const REDACTED = "[REDACTED]";
  const JSON_RESPONSE_HEADERS = { "content-type": "application/json" };

  function activeBaseUrl() {
    const profile = state.profiles[state.profiles.length - 1];
    return (profile && profile.base_url) || "http://localhost:8080/ehrbase";
  }

  function emitEvent(event, payload) {
    (eventListeners[event] || []).forEach(function (handlerId) {
      const cb = callbacks[handlerId];
      if (cb) cb({ event: event, id: 0, payload: payload });
    });
  }

  function aqlResultSet(query, result) {
    return {
      meta: { _type: "RESULTSET", _executed_aql: query },
      q: query,
      columns: result.columns,
      rows: result.rows,
    };
  }

  // Maps a mocked command to the HTTP exchange the real backend performs
  // for it (see src-tauri/src/commands/*.rs for the actual URLs). Returns
  // null for commands that never touch the CDR (profiles, settings, ...).
  function describeRequest(cmd, args, result) {
    const rest = "/rest/openehr/v1";
    const json = function (method, path, status, body, requestBody, extraRequestHeaders) {
      return {
        method: method,
        path: path,
        status: status,
        request_headers: Object.assign({ accept: "application/json" }, extraRequestHeaders || {}),
        request_body: requestBody === undefined ? null : JSON.stringify(requestBody, null, 2),
        response_headers: JSON_RESPONSE_HEADERS,
        response_body: body === null ? null : JSON.stringify(body, null, 2),
      };
    };
    const postJson = { "content-type": "application/json" };
    switch (cmd) {
      case "test_server_connection":
      case "test_unsaved_connection":
        return json("GET", "/rest/status", 200, { ehrbase_version: "2.7.0", openehr_sdk_version: "2.19.0" });
      case "list_ehrs": {
        const q = "SELECT e/ehr_id/value, e/time_created/value, e/system_id/value FROM EHR e ORDER BY e/time_created/value DESC OFFSET 0 LIMIT 21";
        const body = aqlResultSet(q, {
          columns: [{ name: "#0", path: "/ehr_id/value" }, { name: "#1", path: "/time_created/value" }, { name: "#2", path: "/system_id/value" }],
          rows: result.ehrs.map(function (e) { return [e.ehr_id, e.time_created, e.system_id]; }),
        });
        return json("POST", rest + "/query/aql", 200, body, { q: q }, postJson);
      }
      case "get_ehr_detail":
        return json("GET", rest + "/ehr/" + args.ehrId, 200, {
          system_id: { value: result.system_id },
          ehr_id: { value: result.ehr_id },
          time_created: { value: result.time_created },
          ehr_status: { id: { value: result.ehr_id + "::" + result.system_id + "::1" }, namespace: "local", type: "EHR_STATUS" },
        });
      case "get_directory":
        return result
          ? json("GET", rest + "/ehr/" + args.ehrId + "/directory", 200, Object.assign({ _type: "FOLDER" }, result))
          : json("GET", rest + "/ehr/" + args.ehrId + "/directory", 204, null);
      case "get_composition":
        return json("GET", rest + "/ehr/" + args.ehrId + "/composition/" + args.compositionUid, 200, result);
      case "get_composition_flat":
        return json("GET", rest + "/ehr/" + args.ehrId + "/composition/" + args.compositionUid + "?format=FLAT", 200, result);
      case "list_templates":
        return json("GET", rest + "/definition/template/adl1.4", 200, result);
      case "get_web_template":
        return json("GET", rest + "/definition/template/adl1.4/" + encodeURIComponent(args.templateId), 200, result, undefined, {
          accept: "application/openehr.wt+json",
        });
      case "get_template_opt": {
        const entry = json("GET", rest + "/definition/template/adl1.4/" + encodeURIComponent(args.templateId), 200, null, undefined, {
          accept: "application/xml",
        });
        entry.response_headers = { "content-type": "application/xml" };
        entry.response_body = result;
        return entry;
      }
      case "execute_aql":
        return json("POST", rest + "/query/aql", 200, aqlResultSet(args.query, result), { q: args.query }, postJson);
      default:
        return null;
    }
  }

  function logRequest(cmd, args, result) {
    const req = describeRequest(cmd, args, result);
    if (!req) return;
    const id = nextRequestId++;
    emitEvent("cdr-inspector-entry", {
      id: "req-" + id,
      timestamp_ms: Date.now(),
      method: req.method,
      url: activeBaseUrl() + req.path,
      request_headers: Object.assign({ authorization: REDACTED, "user-agent": "openEHR-Explorer" }, req.request_headers),
      request_body: req.request_body,
      status: req.status,
      response_headers: req.response_headers,
      response_body: req.response_body,
      // Deterministic but varied, so the log doesn't read as obviously fake.
      duration_ms: 18 + ((id * 37) % 90),
      body_truncated: false,
    });
  }

  function invoke(cmd, args) {
    if (cmd === "plugin:event|listen") {
      (eventListeners[args.event] = eventListeners[args.event] || []).push(args.handler);
      return Promise.resolve(args.handler);
    }
    if (cmd === "plugin:event|unlisten") {
      return Promise.resolve(undefined);
    }
    const handler = handlers[cmd];
    if (!handler) {
      console.warn("[demo-mock] unhandled invoke:", cmd, args);
      return Promise.resolve(undefined);
    }
    try {
      const result = handler(args || {});
      logRequest(cmd, args || {}, result);
      return Promise.resolve(result);
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
        return callback?.(result);
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
