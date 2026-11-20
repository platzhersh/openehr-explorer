// Minimal EHRBase-like openEHR server for testing the GPUI PoC's network layer (no real CDR available in CI/sandbox).
// Also records Aptabase events at /api/v0/events so analytics can be verified without any external service.
// usage: node mock-cdr.mjs <port> <logfile> [ehrCount=2500]
// Behaviours mirrored from EHRBase: Basic auth, ORDER BY on EHR attributes rejected (set MOCK_ACCEPT_ORDER=1 to allow).
import http from "node:http";
import fs from "node:fs";

const [port = "18080", logfile = "/dev/null", ehrCount = "2500"] = process.argv.slice(2);
const N = Number(ehrCount);
const AUTH = "Basic " + Buffer.from("ehrbase-user:ehrbase-password").toString("base64");
const log = (o) => fs.appendFileSync(logfile, JSON.stringify(o) + "\n");

const uuid = (i) => {
  const h = (n) => (n >>> 0).toString(16).padStart(8, "0");
  return `${h(i * 2654435761)}-1c1b-4b0a-9d1e-${h(i * 40503).padStart(8, "0")}${(i % 65536).toString(16).padStart(4, "0")}`;
};
const created = (i) => `2026-${String(1 + (i % 11)).padStart(2, "0")}-${String(1 + (i % 28)).padStart(2, "0")}T10:${String(i % 60).padStart(2, "0")}:00Z`;

// Per-EHR attributes used by search/sort/directory: subject on even indexes, alternate system every 5th, etc.
const subjectOf = (i) => (i % 2 === 0 ? `P-${10000 + (i % 500)}` : null);
const systemOf = (i) => (i % 5 === 0 ? "other.example.org" : "hospital.example.org");
const modifiableOf = (i) => i % 3 !== 0;
const hasDirectory = (i) => i % 4 === 0;
const ALL = Array.from({ length: N }, (_, i) => ({ i, id: uuid(i), created: created(i) }));
const BY_ID = new Map(ALL.map((e) => [e.id, e.i]));

const TEMPLATES = [
  { template_id: "vital_signs.v1", concept: "Vital Signs", archetype_id: "openEHR-EHR-COMPOSITION.encounter.v1" },
  { template_id: "IDCR - Problem List.v1", concept: "Problem List", archetype_id: "openEHR-EHR-COMPOSITION.problem_list.v1" },
  { template_id: "IDCR - Medication List.v1", concept: "Medication List", archetype_id: "openEHR-EHR-COMPOSITION.medication_list.v1" },
];
const leaf = (id, name, rmType, aqlPath) => ({ id, name, rmType, aqlPath, children: [] });
const WEB_TEMPLATE = (id) => ({
  templateId: id,
  tree: {
    id: "vital_signs", name: "Vital Signs", rmType: "COMPOSITION", aqlPath: "",
    children: [
      { id: "context", name: "Context", rmType: "EVENT_CONTEXT", aqlPath: "/context", children: [leaf("start_time", "Start time", "DV_DATE_TIME", "/context/start_time"), leaf("setting", "Setting", "DV_CODED_TEXT", "/context/setting")] },
      { id: "vital_signs", name: "Vital Signs", rmType: "OBSERVATION", aqlPath: "/content[openEHR-EHR-OBSERVATION.vital_signs.v2]",
        children: ["Systolic", "Diastolic", "Pulse/Heart rate", "Respiration rate", "SpO2", "Body temperature"].map((n, k) => leaf(n.toLowerCase(), n, "DV_QUANTITY", `.../items[at000${4 + k}]/value`)) },
    ],
  },
});

function aql(q) {
  const limit = Number(/LIMIT\s+(\d+)/i.exec(q)?.[1] ?? 50);
  const offset = Number(/OFFSET\s+(\d+)/i.exec(q)?.[1] ?? 0);
  const ob = /ORDER BY e\/(time_created|ehr_id|system_id)\/value (ASC|DESC)/i.exec(q);
  if (ob && !process.env.MOCK_ACCEPT_ORDER) {
    return [400, { error: "Bad Request", message: "AQL ORDER_BY on EHR attributes is not implemented" }];
  }
  if (/SELECT e\/ehr_id\/value, e\/time_created\/value, e\/system_id\/value FROM EHR e/i.test(q)) {
    let rows = ALL.map((e) => [e.id, e.created, systemOf(e.i)]);
    if (ob) {
      const col = { time_created: 1, ehr_id: 0, system_id: 2 }[ob[1].toLowerCase()];
      const sign = ob[2].toUpperCase() === "DESC" ? -1 : 1;
      rows.sort((a, b) => sign * String(a[col]).localeCompare(String(b[col])) || String(a[0]).localeCompare(String(b[0])));
    }
    rows = rows.slice(offset, offset + limit);
    return [200, { columns: [{ name: "#0", path: "e/ehr_id/value" }, { name: "#1", path: "e/time_created/value" }, { name: "#2", path: "e/system_id/value" }], rows }];
  }
  // Search: "... AS subject_id, ... AS subject_namespace, ... AS modifiable, ... AS queryable, ... AS system_id FROM EHR e CONTAINS ..."
  if (/AS subject_id/i.test(q) && /FROM EHR e CONTAINS (EHR_STATUS s|COMPOSITION c)/i.test(q)) {
    let list = ALL.slice();
    const like = /\/subject\/external_ref\/id\/value LIKE '%(.*?)%'/i.exec(q);
    if (like) list = list.filter((e) => (subjectOf(e.i) ?? "").includes(like[1].replace(/''/g, "'")));
    const ns = /external_ref\/namespace = '(.*?)'/i.exec(q);
    if (ns) list = list.filter((e) => subjectOf(e.i) !== null && ns[1] === "nhs");
    const sys = /e\/system_id\/value = '(.*?)'/i.exec(q);
    if (sys) list = list.filter((e) => systemOf(e.i) === sys[1]);
    const mod = /is_modifiable = (true|false)/i.exec(q);
    if (mod) list = list.filter((e) => modifiableOf(e.i) === (mod[1] === "true"));
    const eq = /e\/ehr_id\/value = '([0-9a-f-]+)'/i.exec(q);
    if (eq) list = list.filter((e) => e.id === eq[1]);
    if (/CONTAINS COMPOSITION c/i.test(q)) list = list.filter((e) => e.i % 7 === 0); // only some EHRs have compositions
    const rows = list.slice(offset, offset + limit).map((e) => [e.id, e.created, subjectOf(e.i), subjectOf(e.i) === null ? null : "nhs", modifiableOf(e.i), true, systemOf(e.i)]);
    return [200, { columns: [], rows }];
  }
  const m = /ehr_id\/value='([0-9a-f-]+)'/i.exec(q);
  if (m) {
    const first = m[1] === uuid(0);
    const rows = first ? [
      ["c1a2b3c4-0001::hospital.example.org::1", "Vital Signs", "vital_signs.v1", "Dr. A. Kessler", "2026-08-20T09:15:00Z"],
      ["c1a2b3c4-0002::hospital.example.org::1", "Problem List", "IDCR - Problem List.v1", "Dr. A. Kessler", "2026-08-18T13:42:10Z"],
      ["c1a2b3c4-0003::hospital.example.org::1", "Medication List", "IDCR - Medication List.v1", "Dr. R. Okafor", "2026-08-12T10:03:44Z"],
    ] : [];
    return [200, { columns: [], rows }];
  }
  const n = limit;
  const rows = Array.from({ length: n }, (_, i) => [uuid(i), 100 + (i % 60), 60 + (i % 40), created(i), i % 7 === 0 ? null : { magnitude: 1.5, units: "kg" }]);
  return [200, { columns: [{ name: "ehr_id", path: "/ehr_id/value" }, { name: "systolic", path: "/systolic" }, { name: "diastolic", path: "/diastolic" }, { name: "time", path: "/t" }, { name: "weight", path: "/w" }], rows }];
}

http.createServer((req, res) => {
  let body = "";
  req.on("data", (c) => (body += c));
  req.on("end", () => {
    const send = (code, obj) => { res.writeHead(code, { "content-type": "application/json" }); res.end(JSON.stringify(obj)); };
    const path = req.url.split("?")[0];
    if (path === "/api/v0/events") { // Aptabase ingest: record headers + body, no auth
      log({ aptabase: true, app_key: req.headers["app-key"], body: JSON.parse(body || "[]") });
      return send(200, {});
    }
    const ok = req.headers.authorization === AUTH;
    log({ method: req.method, path, auth_ok: ok, accept: req.headers.accept, q: body ? JSON.parse(body).q : undefined });
    if (!ok) return send(401, { error: "Unauthorized" });
    if (path === "/rest/status") return send(200, { ehrbase_version: "2.7.0", openehr_sdk_version: "2.19.0" });
    const R = "/rest/openehr/v1";
    if (path === `${R}/definition/template/adl1.4`) return send(200, TEMPLATES);
    if (path.startsWith(`${R}/definition/template/adl1.4/`)) return send(200, WEB_TEMPLATE(decodeURIComponent(path.split("/").pop())));
    const dm = new RegExp(`^${R}/ehr/([^/]+)/directory$`).exec(path);
    if (dm && req.method === "GET") {
      const idx = BY_ID.get(dm[1]);
      return idx !== undefined && hasDirectory(idx) ? send(200, { _type: "FOLDER", name: { value: "Patient Record" } }) : send(404, { error: "Not Found" });
    }
    const cm = new RegExp(`^${R}/ehr/([^/]+)/composition/([^/]+)$`).exec(path);
    if (cm && req.method === "GET") {
      const flat = /flat/.test(req.headers.accept || "");
      if (flat) return send(200, { "vital_signs/context/start_time": "2026-08-20T09:15:00Z", "vital_signs/vital_signs/systolic|magnitude": 128, "vital_signs/vital_signs/systolic|unit": "mm[Hg]", "vital_signs/composer|name": "Dr. A. Kessler" });
      return send(200, { _type: "COMPOSITION", name: { _type: "DV_TEXT", value: "Vital Signs" }, uid: { _type: "OBJECT_VERSION_ID", value: decodeURIComponent(cm[2]) }, archetype_details: { template_id: { value: "vital_signs.v1" } }, composer: { _type: "PARTY_IDENTIFIED", name: "Dr. A. Kessler" },
        content: [{ _type: "OBSERVATION", name: { value: "Vital Signs" }, data: { events: [{ data: { items: [{ name: { value: "Systolic" }, value: { magnitude: 128, units: "mm[Hg]" } }] } }] } }] });
    }
    if (path === `${R}/query/aql` && req.method === "POST") { const [c, o] = aql(JSON.parse(body).q); return send(c, o); }
    send(404, { error: "Not Found", path });
  });
}).listen(Number(port), "127.0.0.1", () => console.error(`mock CDR on :${port}`));
