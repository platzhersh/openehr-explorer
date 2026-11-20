//! A for-fun GPUI port of openEHR Explorer's main screens.
//!
//! Data is in-process and mirrors `scripts/demo-assets/mock.js` so that screenshots
//! and benchmarks can be compared 1:1 with the Vue frontend.
//!
//! Without `--server` the data is in-process demo data. With `--server URL` it talks to a real openEHR CDR
//! (credentials via env: OEH_USER/OEH_PASSWORD or OEH_TOKEN; never on the command line).
//!
//! Flags:  --view dashboard|ehrs|templates|aql   --ehrs N   --rows N
//!         --server URL  [--api-prefix /rest/openehr/v1]  [--query "AQL..."]
//!         --analytics   [--analytics-host URL]   (opt-in Aptabase events; needs APTABASE_APP_KEY)
//!         --select              (open first EHR / first template detail)
//!         --run                 (execute the AQL query immediately)
//!         --bench-scroll        (auto-scroll the AQL table, print frame stats, exit)

use gpui::prelude::FluentBuilder;
use gpui::{
    div, point, px, rgb, size, uniform_list, App, Application, Bounds, AppContext, Context, Div, FontWeight, InteractiveElement,
    IntoElement, ParentElement, Render, ScrollStrategy, SharedString, StatefulInteractiveElement, Styled,
    TitlebarOptions, UniformListScrollHandle, Window, WindowBounds, WindowOptions,
};
mod analytics;
mod net;

use analytics::Analytics;
use serde_json::json;
use std::ops::Range;
use std::time::{Duration, Instant};

// ---- design tokens (src/styles/tokens.css) ----
const BG: u32 = 0x1a1a2e;
const BG2: u32 = 0x16213e;
const BG3: u32 = 0x0f3460;
const SURFACE: u32 = 0x1e2a4a;
const SURFACE_HOVER: u32 = 0x253456;
const BORDER: u32 = 0x2a3a5c;
const TEXT: u32 = 0xe0e0e0;
const TEXT2: u32 = 0xb4bedb;
const MUTED: u32 = 0x9aa6c6;
const PRIMARY: u32 = 0x64ffda;
const SUCCESS: u32 = 0x6bff8e;
const MONO: &str = "DejaVu Sans Mono";

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Dashboard,
    Ehrs,
    Templates,
    Aql,
}

struct Ehr {
    id: SharedString,
    created: SharedString,
    subject: SharedString,
    compositions: u32,
}

#[derive(Clone)]
struct Composition {
    name: SharedString,
    template: SharedString,
    composer: SharedString,
    committed: SharedString,
}

#[derive(Clone)]
struct Template {
    id: SharedString,
    concept: SharedString,
    archetype: SharedString,
}

#[derive(Clone)]
struct TreeRow {
    depth: u8,
    label: SharedString,
    rm: SharedString,
    path: SharedString,
}

#[derive(Clone)]
enum Conn {
    Demo,
    Connecting,
    Connected(SharedString),
    Failed(SharedString),
}

const PAGE: usize = 100;
/// Usable table width at the default 1440px window (window minus sidebar and padding).
const TABLE_FIT_PX: f32 = 1120.0;
/// The composition query is capped; show "100+" when the cap is hit.
const COMPS_LIMIT: usize = 100;

const DEMO_TEMPLATES: [(&str, &str, &str); 4] = [
    ("vital_signs.v1", "Vital Signs", "openEHR-EHR-COMPOSITION.encounter.v1"),
    ("IDCR - Problem List.v1", "Problem List", "openEHR-EHR-COMPOSITION.problem_list.v1"),
    ("IDCR - Medication List.v1", "Medication List", "openEHR-EHR-COMPOSITION.medication_list.v1"),
    ("IDCR - Discharge Summary.v1", "Discharge Summary", "openEHR-EHR-COMPOSITION.discharge_summary.v1"),
];

const DEMO_COMPOSITIONS: [(&str, &str, &str, &str); 3] = [
    ("Vital Signs", "vital_signs.v1", "Dr. A. Kessler", "2026-08-20T09:15:00Z"),
    ("Problem List", "IDCR - Problem List.v1", "Dr. A. Kessler", "2026-08-18T13:42:10Z"),
    ("Medication List", "IDCR - Medication List.v1", "Dr. R. Okafor", "2026-08-12T10:03:44Z"),
];

// (depth, label, rm_type, aql_path)
const DEMO_TREE: [(u8, &str, &str, &str); 11] = [
    (0, "Vital Signs", "COMPOSITION", "/"),
    (1, "Context", "EVENT_CONTEXT", "/context"),
    (2, "Start time", "DV_DATE_TIME", "/context/start_time"),
    (2, "Setting", "DV_CODED_TEXT", "/context/setting"),
    (1, "Vital Signs", "OBSERVATION", "/content[openEHR-EHR-OBSERVATION.vital_signs.v2]"),
    (2, "Systolic", "DV_QUANTITY", ".../items[at0004]/value"),
    (2, "Diastolic", "DV_QUANTITY", ".../items[at0005]/value"),
    (2, "Pulse/Heart rate", "DV_QUANTITY", ".../items[at0006]/value"),
    (2, "Respiration rate", "DV_QUANTITY", ".../items[at0007]/value"),
    (2, "SpO2", "DV_QUANTITY", ".../items[at0008]/value"),
    (2, "Body temperature", "DV_QUANTITY", ".../items[at0009]/value"),
];

fn demo_templates() -> Vec<Template> {
    DEMO_TEMPLATES.iter().map(|(id, c, a)| Template { id: (*id).into(), concept: (*c).into(), archetype: (*a).into() }).collect()
}

fn demo_comps(n: usize) -> Vec<Composition> {
    DEMO_COMPOSITIONS
        .iter()
        .take(n)
        .map(|(n, t, c, d)| Composition { name: (*n).into(), template: (*t).into(), composer: (*c).into(), committed: (*d).into() })
        .collect()
}

fn demo_tree() -> Vec<TreeRow> {
    DEMO_TREE.iter().map(|(d, l, r, p)| TreeRow { depth: *d, label: (*l).into(), rm: (*r).into(), path: (*p).into() }).collect()
}

const NAV: [(View, &str); 4] = [
    (View::Dashboard, "Dashboard"),
    (View::Ehrs, "EHR Browser"),
    (View::Templates, "Templates"),
    (View::Aql, "AQL Runner"),
];

const QUERY: &str = "SELECT c/uid/value,\n       o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude AS systolic\nFROM EHR e\nCONTAINS COMPOSITION c\nCONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.vital_signs.v2]";

/// Tiny deterministic generator so every run (and the JS side) sees the same data.
fn lcg(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *seed >> 33
}

fn fake_uuid(seed: &mut u64) -> String {
    let a = lcg(seed) as u32;
    let b = lcg(seed) as u16;
    let c = lcg(seed) as u16;
    let d = lcg(seed) as u16;
    let e = lcg(seed);
    format!("{a:08x}-{b:04x}-4{:03x}-9{:03x}-{e:012x}", c & 0xfff, d & 0xfff)
}

/// Row-major flat cell storage (no per-row allocation, so memory stays flat for very large results).
struct AqlTable {
    columns: Vec<SharedString>,
    cells: Vec<SharedString>,
    widths: Vec<f32>,
    exec_ms: f64,
}

impl AqlTable {
    fn new(columns: Vec<SharedString>, cells: Vec<SharedString>, exec_ms: f64) -> Self {
        let ncols = columns.len().max(1);
        // Column width from the header and the first 50 rows (~7.4px per mono glyph at 12px).
        let widths = (0..ncols)
            .map(|c| {
                let head = columns.get(c).map(|h| h.chars().count()).unwrap_or(0);
                let body = cells.iter().skip(c).step_by(ncols).take(50).map(|v| v.chars().count()).max().unwrap_or(0);
                (head.max(body) as f32 * 7.4 + 28.0).clamp(90.0, 460.0)
            })
            .collect::<Vec<f32>>();
        // No horizontal scrolling yet: shrink columns proportionally (min 90px) so the table fits the pane.
        let total: f32 = widths.iter().sum();
        let widths = if total > TABLE_FIT_PX { widths.iter().map(|w| (w * TABLE_FIT_PX / total).max(90.0)).collect() } else { widths };
        Self { columns, cells, widths, exec_ms }
    }
    fn ncols(&self) -> usize {
        self.columns.len().max(1)
    }
    fn row_count(&self) -> usize {
        self.cells.len() / self.ncols()
    }
    fn row(&self, i: usize) -> &[SharedString] {
        &self.cells[i * self.ncols()..(i + 1) * self.ncols()]
    }
}

struct Bench {
    start: Instant,
    last: Instant,
    frame: usize,
    total_frames: usize,
    intervals: Vec<f64>,
    render_costs: Vec<f64>,
}

struct Explorer {
    view: View,
    ehrs: Vec<Ehr>,
    selected_ehr: Option<usize>,
    selected_template: usize,
    templates: Vec<Template>,
    tree: Vec<TreeRow>,
    comps: Vec<Composition>,
    comps_loading: bool,
    server: Option<net::Profile>,
    conn: Conn,
    error: Option<SharedString>,
    ehrs_has_more: bool,
    ehrs_loading: bool,
    aql_loading: bool,
    auto_run: bool,
    auto_select: bool,
    query: SharedString,
    analytics: Analytics,
    table: Option<AqlTable>,
    row_target: usize,
    ehr_scroll: UniformListScrollHandle,
    aql_scroll: UniformListScrollHandle,
    tree_scroll: UniformListScrollHandle,
    tpl_scroll: UniformListScrollHandle,
    boot: Instant,
    first_render_reported: bool,
    run_clicked_at: Option<Instant>,
    bench: Option<Bench>,
    bench_scroll: bool,
}

/// Proportional set size (matches how the Chromium process tree is measured in bench/).
fn rss_kb() -> u64 {
    std::fs::read_to_string("/proc/self/smaps_rollup")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("Pss:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
        })
        .unwrap_or(0)
}

struct Opts {
    boot: Instant,
    view: View,
    ehr_count: usize,
    rows: usize,
    select: bool,
    run: bool,
    bench_scroll: bool,
    server: Option<net::Profile>,
    query: Option<String>,
    analytics: Analytics,
}

impl Explorer {
    fn new(o: Opts) -> Self {
        let server = o.server;
        let mut ehrs = Vec::new();
        let mut comps = Vec::new();
        if server.is_none() {
            let mut seed = 42u64;
            ehrs = vec![
                Ehr { id: "9c2c5c5e-1c1b-4b0a-9d1e-9a4b2f3c8a11".into(), created: "2026-06-02T08:14:21Z".into(), subject: "P-10234".into(), compositions: 3 },
                Ehr { id: "4a9e2d7b-6f31-4c9e-8a5a-1d2e3f4a5b6c".into(), created: "2026-08-19T10:05:17Z".into(), subject: "—".into(), compositions: 0 },
                Ehr { id: "b28f6a9c-2e4d-4a1b-9c3f-7e8d9c0b1a2f".into(), created: "2026-08-11T16:38:44Z".into(), subject: "—".into(), compositions: 0 },
            ];
            while ehrs.len() < o.ehr_count {
                let n = lcg(&mut seed);
                ehrs.push(Ehr {
                    id: fake_uuid(&mut seed).into(),
                    created: format!("2026-{:02}-{:02}T{:02}:{:02}:00Z", 1 + n % 11, 1 + (n >> 4) % 28, (n >> 9) % 24, (n >> 14) % 60).into(),
                    subject: format!("P-{}", 10000 + n % 90000).into(),
                    compositions: (n % 7) as u32,
                });
            }
            if o.select {
                comps = demo_comps(3);
            }
        }
        let demo = server.is_none();
        Self {
            view: o.view,
            ehrs,
            selected_ehr: (demo && o.select).then_some(0),
            selected_template: 0,
            templates: if demo { demo_templates() } else { Vec::new() },
            tree: if demo { demo_tree() } else { Vec::new() },
            comps,
            comps_loading: false,
            conn: if demo { Conn::Demo } else { Conn::Connecting },
            server,
            error: None,
            ehrs_has_more: false,
            ehrs_loading: false,
            aql_loading: false,
            auto_run: o.run,
            auto_select: o.select,
            query: o.query.unwrap_or_else(|| QUERY.to_string()).into(),
            analytics: o.analytics,
            table: None,
            row_target: o.rows,
            ehr_scroll: UniformListScrollHandle::new(),
            aql_scroll: UniformListScrollHandle::new(),
            tree_scroll: UniformListScrollHandle::new(),
            tpl_scroll: UniformListScrollHandle::new(),
            boot: o.boot,
            first_render_reported: false,
            run_clicked_at: None,
            bench: None,
            bench_scroll: o.bench_scroll,
        }
    }

    fn start(&mut self, cx: &mut Context<Self>) {
        let mode = if self.server.is_some() { "server" } else { "demo" };
        self.analytics.track("app_started", Some(json!({ "mode": mode })));
        if self.server.is_some() {
            self.connect(cx);
        }
        if self.auto_run {
            self.run_query(cx);
        }
    }

    /// Spawn `fut` on the Tokio runtime and apply its result to `self` on the GPUI thread.
    fn spawn_net<T, Fut>(&self, cx: &mut Context<Self>, fut: Fut, apply: impl FnOnce(&mut Self, Result<T, String>, &mut Context<Self>) + 'static)
    where
        T: Send + 'static,
        Fut: std::future::Future<Output = Result<T, String>> + Send + 'static,
    {
        let task = net::runtime().spawn(fut);
        cx.spawn(async move |this, cx| {
            let res = match task.await {
                Ok(r) => r,
                Err(e) => Err(format!("Network task failed: {e}")),
            };
            this.update(cx, |this, cx| {
                apply(this, res, cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn connect(&mut self, cx: &mut Context<Self>) {
        let Some(server) = self.server.clone() else { return };
        self.conn = Conn::Connecting;
        self.error = None;
        self.ehrs_loading = true;
        let fut = async move {
            let desc = server.test_connection().await?;
            let templates = server.list_templates().await?;
            let page = server.list_ehrs(0, PAGE).await?;
            Ok::<_, String>((desc, templates, page))
        };
        self.spawn_net(cx, fut, |this, res, cx| {
            this.ehrs_loading = false;
            match res {
                Ok((desc, templates, page)) => {
                    this.analytics.track("connection_result", Some(json!({ "ok": true })));
                    this.conn = Conn::Connected(desc.into());
                    this.templates = templates.into_iter().map(|t| Template { id: t.id.into(), concept: t.concept.into(), archetype: t.archetype.into() }).collect();
                    this.ehrs = page.rows.into_iter().map(|r| Ehr { id: r.id.into(), created: r.created.into(), subject: r.system_id.into(), compositions: 0 }).collect();
                    this.ehrs_has_more = page.has_more;
                    if !this.templates.is_empty() {
                        this.select_template(0, cx);
                    }
                    if this.auto_select && !this.ehrs.is_empty() {
                        this.select_ehr(0, cx);
                    }
                }
                Err(e) => {
                    this.analytics.track("connection_result", Some(json!({ "ok": false })));
                    this.conn = Conn::Failed(e.clone().into());
                    this.error = Some(e.into());
                }
            }
        });
    }

    fn load_more_ehrs(&mut self, cx: &mut Context<Self>) {
        let Some(server) = self.server.clone() else { return };
        if self.ehrs_loading {
            return;
        }
        self.ehrs_loading = true;
        let offset = self.ehrs.len();
        self.spawn_net(cx, async move { server.list_ehrs(offset, PAGE).await }, |this, res, _cx| {
            this.ehrs_loading = false;
            match res {
                Ok(page) => {
                    this.ehrs.extend(page.rows.into_iter().map(|r| Ehr { id: r.id.into(), created: r.created.into(), subject: r.system_id.into(), compositions: 0 }));
                    this.ehrs_has_more = page.has_more;
                }
                Err(e) => this.error = Some(e.into()),
            }
        });
    }

    fn select_ehr(&mut self, i: usize, cx: &mut Context<Self>) {
        self.selected_ehr = Some(i);
        let Some(server) = self.server.clone() else {
            self.comps = demo_comps(self.ehrs[i].compositions.min(3) as usize);
            return;
        };
        let id = self.ehrs[i].id.to_string();
        self.comps.clear();
        self.comps_loading = true;
        self.spawn_net(cx, async move { server.ehr_compositions(&id).await }, move |this, res, _cx| {
            // Ignore a stale answer if the user already picked another EHR.
            if this.selected_ehr != Some(i) {
                return;
            }
            this.comps_loading = false;
            match res {
                Ok(rows) => {
                    this.comps = rows.into_iter().map(|c| Composition { name: c.name.into(), template: c.template.into(), composer: c.composer.into(), committed: c.committed.into() }).collect();
                    if let Some(e) = this.ehrs.get_mut(i) {
                        e.compositions = this.comps.len() as u32;
                    }
                }
                Err(e) => this.error = Some(e.into()),
            }
        });
    }

    fn select_template(&mut self, i: usize, cx: &mut Context<Self>) {
        self.selected_template = i;
        let Some(server) = self.server.clone() else { return };
        let Some(t) = self.templates.get(i) else { return };
        let id = t.id.to_string();
        self.spawn_net(cx, async move { server.web_template(&id).await }, move |this, res, _cx| {
            if this.selected_template != i {
                return;
            }
            match res {
                Ok(rows) => this.tree = rows.into_iter().map(|r| TreeRow { depth: r.depth, label: r.label.into(), rm: r.rm_type.into(), path: r.path.into() }).collect(),
                Err(e) => {
                    this.tree.clear();
                    this.error = Some(e.into());
                }
            }
        });
    }

    /// Called by the bench driver task: advance the scroll position from the number of frames
    /// rendered so far and ask for another frame.
    fn bench_tick(&mut self, cx: &mut Context<Self>) {
        let Some(b) = self.bench.as_ref() else { return };
        let n = self.table.as_ref().map(|t| t.row_count()).unwrap_or(0);
        if n > 0 {
            let ix = (b.frame * (n / b.total_frames).max(1)).min(n - 1);
            self.aql_scroll.scroll_to_item(ix, ScrollStrategy::Top);
        }
        cx.notify();
    }

    fn run_query(&mut self, cx: &mut Context<Self>) {
        let Some(server) = self.server.clone() else {
            self.run_query_demo();
            self.analytics.track("aql_run", Some(json!({ "mode": "demo" })));
            return;
        };
        if self.aql_loading {
            return;
        }
        let q = self.query.to_string();
        self.error = None;
        self.aql_loading = true;
        let t = Instant::now();
        self.spawn_net(cx, async move { server.aql(&q).await }, move |this, res, _cx| {
            this.aql_loading = false;
            match res {
                Ok(o) => {
                    this.analytics.track("aql_run", Some(json!({ "mode": "server", "rows": o.rows.len() })));
                    let ncols = o.columns.len();
                    let cells: Vec<SharedString> = o
                        .rows
                        .into_iter()
                        .flat_map(|mut r| {
                            r.resize(ncols, String::new());
                            r.into_iter().map(SharedString::from)
                        })
                        .collect();
                    let columns = o.columns.into_iter().map(SharedString::from).collect();
                    this.table = Some(AqlTable::new(columns, cells, t.elapsed().as_secs_f64() * 1000.0));
                    this.run_clicked_at = Some(t);
                }
                Err(e) => this.error = Some(e.into()),
            }
        });
    }

    fn run_query_demo(&mut self) {
        let t = Instant::now();
        let mut seed = 7u64;
        let n = self.row_target;
        let mut cells: Vec<SharedString> = Vec::with_capacity(n * 4);
        for i in 0..n {
            let id: SharedString = if i < 3 {
                ["9c2c5c5e-1c1b-4b0a-9d1e-9a4b2f3c8a11", "4a9e2d7b-6f31-4c9e-8a5a-1d2e3f4a5b6c", "b28f6a9c-2e4d-4a1b-9c3f-7e8d9c0b1a2f"][i].into()
            } else {
                fake_uuid(&mut seed).into()
            };
            let r = lcg(&mut seed);
            cells.push(id);
            cells.push(format!("{}", 100 + r % 60).into());
            cells.push(format!("{}", 60 + (r >> 8) % 40).into());
            cells.push(format!("2026-{:02}-{:02}T{:02}:{:02}:00Z", 1 + r % 11, 1 + (r >> 4) % 28, (r >> 9) % 24, (r >> 14) % 60).into());
        }
        let columns = ["ehr_id", "systolic", "diastolic", "time_committed"].map(SharedString::from).to_vec();
        self.table = Some(AqlTable::new(columns, cells, t.elapsed().as_secs_f64() * 1000.0));
        self.run_clicked_at = Some(t);
    }

    fn page_title(title: &'static str, subtitle: String) -> Div {
        div().flex().flex_col().gap_1().pb_4().child(div().text_size(px(22.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(PRIMARY)).child(title)).child(div().text_size(px(13.)).text_color(rgb(MUTED)).child(subtitle))
    }

    fn sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut nav = div().flex().flex_col().gap_1().p_3();
        for (view, label) in NAV {
            let active = self.view == view;
            nav = nav.child(
                div()
                    .id(label)
                    .flex()
                    .items_center()
                    .h(px(38.))
                    .px_3()
                    .rounded(px(6.))
                    .cursor_pointer()
                    .text_size(px(14.))
                    .text_color(rgb(if active { PRIMARY } else { TEXT2 }))
                    .bg(rgb(if active { BG3 } else { BG2 }))
                    .hover(|s| s.bg(rgb(SURFACE_HOVER)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.view = view;
                        this.analytics.track("view_opened", Some(json!({ "view": label })));
                        cx.notify();
                    }))
                    .child(label),
            );
        }
        div()
            .w(px(240.))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .justify_between()
            .bg(rgb(BG2))
            .border_r_1()
            .border_color(rgb(BORDER))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(div().px_4().py_4().text_size(px(17.)).font_weight(FontWeight::BOLD).text_color(rgb(PRIMARY)).child("openEHR Explorer"))
                    .child(nav),
            )
            .child({
                let (dot, text) = match &self.conn {
                    Conn::Demo => (MUTED, SharedString::from("Demo data (no server)")),
                    Conn::Connecting => (0xffd93d, SharedString::from("Connecting…")),
                    Conn::Connected(d) => (SUCCESS, d.clone()),
                    Conn::Failed(_) => (0xff7b7b, SharedString::from("Connection failed")),
                };
                div()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().flex().items_center().gap_2().child(div().size(px(8.)).rounded_full().bg(rgb(dot))).child(div().text_size(px(12.)).text_color(rgb(TEXT2)).child(text)))
                    .child(div().text_size(px(12.)).text_color(rgb(MUTED)).child(if self.analytics.enabled() { "v0.10.3 · gpui · analytics on" } else { "v0.10.3 · gpui" }))
            })
    }

    fn dashboard(&self) -> Div {
        let card = |label: &'static str, value: String| {
            div()
                .flex_1()
                .p_4()
                .rounded(px(6.))
                .bg(rgb(SURFACE))
                .border_1()
                .border_color(rgb(BORDER))
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_size(px(12.)).text_color(rgb(MUTED)).child(label))
                .child(div().text_size(px(30.)).font_weight(FontWeight::BOLD).text_color(rgb(PRIMARY)).child(value))
        };
        let demo = self.server.is_none();
        let comps = if demo { self.ehrs.iter().map(|e| e.compositions).sum::<u32>().to_string() } else { "—".to_string() };
        let ehr_count = format!("{}{}", self.ehrs.len(), if self.ehrs_has_more { "+" } else { "" });
        let (status_dot, status_text): (u32, SharedString) = match &self.conn {
            Conn::Demo => (SUCCESS, "Connected — EHRBase 2.7.0, openEHR SDK 2.19.0 (demo data)".into()),
            Conn::Connecting => (0xffd93d, "Connecting…".into()),
            Conn::Connected(d) => (SUCCESS, format!("Connected — {d}").into()),
            Conn::Failed(e) => (0xff7b7b, format!("Connection failed — {e}").into()),
        };
        div()
            .flex()
            .flex_col()
            .p_6()
            .gap_4()
            .child(Self::page_title("Dashboard", if demo { "Local EHRBase · hospital.example.org".to_string() } else { self.server.as_ref().map(|s| s.base_url.clone()).unwrap_or_default() }))
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(card("EHRs", ehr_count))
                    .child(card("Compositions", comps))
                    .child(card("Templates", self.templates.len().to_string())),
            )
            .child(
                div()
                    .p_4()
                    .rounded(px(6.))
                    .bg(rgb(SURFACE))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().size(px(8.)).rounded_full().bg(rgb(status_dot)))
                    .child(div().text_size(px(13.)).text_color(rgb(TEXT)).child(status_text)),
            )
    }

    fn ehr_browser(&mut self, cx: &mut Context<Self>) -> Div {
        let count = self.ehrs.len();
        let list = uniform_list(
            "ehr-list",
            count,
            cx.processor(|this, range: Range<usize>, _w, cx| {
                range
                    .map(|i| {
                        let e = &this.ehrs[i];
                        let sel = this.selected_ehr == Some(i);
                        div()
                            .id(("ehr", i))
                            .w_full()
                            .h(px(56.))
                            .px_3()
                            .flex()
                            .flex_col()
                            .justify_center()
                            .gap_1()
                            .cursor_pointer()
                            .border_b_1()
                            .border_color(rgb(BORDER))
                            .bg(rgb(if sel { BG3 } else { SURFACE }))
                            .hover(|s| s.bg(rgb(SURFACE_HOVER)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_ehr(i, cx);
                                cx.notify();
                            }))
                            .child(div().font_family(MONO).text_size(px(12.)).text_color(rgb(TEXT)).child(e.id.clone()))
                            .child(div().text_size(px(11.)).text_color(rgb(MUTED)).child(format!("{} · {}", e.subject, e.created)))
                    })
                    .collect()
            }),
        )
        .track_scroll(self.ehr_scroll.clone())
        .w_full()
        .flex_1();

        let list_pane = div()
            .w(px(420.))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(rgb(BORDER))
            .child(div().p_4().child(Self::page_title("EHR Browser", format!("{count}{} EHRs", if self.ehrs_has_more { "+" } else { "" }))))
            .child(list)
            .children(self.server.is_some().then(|| {
                let label = if self.ehrs_loading { "Loading…" } else if self.ehrs_has_more { "Load more" } else { "End of list" };
                div().p_3().border_t_1().border_color(rgb(BORDER)).child(
                    div()
                        .id("load-more")
                        .h(px(32.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(6.))
                        .text_size(px(13.))
                        .text_color(rgb(if self.ehrs_has_more { PRIMARY } else { MUTED }))
                        .bg(rgb(SURFACE))
                        .when(self.ehrs_has_more && !self.ehrs_loading, |d| d.cursor_pointer().hover(|s| s.bg(rgb(SURFACE_HOVER))).on_click(cx.listener(|this, _, _, cx| {
                            this.load_more_ehrs(cx);
                            cx.notify();
                        })))
                        .child(label),
                )
            }));

        let detail = match self.selected_ehr {
            None => div().flex_1().flex().items_center().justify_center().text_color(rgb(MUTED)).child("Select an EHR"),
            Some(i) => {
                let e = &self.ehrs[i];
                let kv = |k: &'static str, v: SharedString| {
                    div()
                        .flex()
                        .gap_3()
                        .py_1()
                        .child(div().w(px(130.)).flex_none().text_size(px(13.)).text_color(rgb(MUTED)).child(k))
                        .child(div().text_size(px(13.)).font_family(MONO).text_color(rgb(TEXT)).child(v))
                };
                let mut comps = div().flex().flex_col().gap_2();
                for c in self.comps.iter() {
                    comps = comps.child(
                        div()
                            .p_3()
                            .rounded(px(6.))
                            .bg(rgb(SURFACE))
                            .border_1()
                            .border_color(rgb(BORDER))
                            .flex()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(div().text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(TEXT)).child(c.name.clone()))
                                    .child(div().text_size(px(12.)).text_color(rgb(MUTED)).child(c.template.clone())),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_end()
                                    .gap_1()
                                    .child(div().text_size(px(12.)).text_color(rgb(TEXT2)).child(c.composer.clone()))
                                    .child(div().text_size(px(12.)).text_color(rgb(MUTED)).child(c.committed.clone())),
                            ),
                    );
                }
                div()
                    .flex_1()
                    .p_6()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(div().text_size(px(18.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(PRIMARY)).child("EHR Detail"))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(kv("EHR ID", e.id.clone()))
                            .child(kv("System ID", if self.server.is_some() { e.subject.clone() } else { "hospital.example.org".into() }))
                            .child(kv("Created", e.created.clone()))
                            .child(kv("Subject", if self.server.is_some() { "—".into() } else { e.subject.clone() })),
                    )
                    .child(div().text_size(px(15.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(TEXT)).child(if self.comps_loading { "Compositions (loading…)".to_string() } else { format!("Compositions ({}{})", self.comps.len(), if self.comps.len() >= COMPS_LIMIT { "+" } else { "" }) }))
                    .child(comps)
            }
        };
        div().flex().size_full().child(list_pane).child(detail)
    }

    fn template_browser(&mut self, cx: &mut Context<Self>) -> Div {
        let list = uniform_list(
            "tpl-list",
            self.templates.len(),
            cx.processor(|this, range: Range<usize>, _w, cx| {
                range
                    .map(|i| {
                        let t = this.templates[i].clone();
                        let sel = this.selected_template == i;
                        div().w_full().h(px(104.)).px_4().py_1().child(
                            div()
                                .id(("tpl", i))
                                .size_full()
                                .p_3()
                                .rounded(px(6.))
                                .overflow_hidden()
                                .cursor_pointer()
                                .border_1()
                                .border_color(rgb(if sel { PRIMARY } else { BORDER }))
                                .bg(rgb(if sel { BG3 } else { SURFACE }))
                                .hover(|s| s.bg(rgb(SURFACE_HOVER)))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.select_template(i, cx);
                                    cx.notify();
                                }))
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(div().text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(TEXT)).whitespace_nowrap().text_ellipsis().overflow_hidden().child(t.concept))
                                .child(div().text_size(px(12.)).font_family(MONO).text_color(rgb(MUTED)).whitespace_nowrap().text_ellipsis().overflow_hidden().child(t.id))
                                .child(div().text_size(px(11.)).font_family(MONO).text_color(rgb(MUTED)).whitespace_nowrap().text_ellipsis().overflow_hidden().child(t.archetype)),
                        )
                    })
                    .collect()
            }),
        )
        .track_scroll(self.tpl_scroll.clone())
        .w_full()
        .flex_1();
        let concept: SharedString = self.templates.get(self.selected_template).map(|t| t.concept.clone()).unwrap_or_else(|| "No template".into());
        let tree = uniform_list(
            "tpl-tree",
            self.tree.len(),
            cx.processor(|this, range: Range<usize>, _w, _cx| {
                range
                    .map(|i| {
                        let TreeRow { depth, label, rm, path } = this.tree[i].clone();
                        div()
                            .w_full()
                            .h(px(32.))
                            .flex()
                            .items_center()
                            .gap_3()
                            .pl(px(12. + depth as f32 * 22.))
                            .border_b_1()
                            .border_color(rgb(BORDER))
                            .child(div().text_size(px(13.)).text_color(rgb(TEXT)).w(px(170.)).flex_none().whitespace_nowrap().text_ellipsis().overflow_hidden().child(label))
                            .child(div().text_size(px(11.)).font_family(MONO).text_color(rgb(PRIMARY)).w(px(110.)).flex_none().whitespace_nowrap().overflow_hidden().child(rm))
                            .child(div().text_size(px(11.)).font_family(MONO).text_color(rgb(MUTED)).whitespace_nowrap().text_ellipsis().overflow_hidden().min_w_0().child(path))
                    })
                    .collect()
            }),
        )
        .track_scroll(self.tree_scroll.clone())
        .w_full()
        .flex_1();
        div()
            .flex()
            .size_full()
            .child(
                div()
                    .w(px(420.))
                    .flex_none()
                    .h_full()
                    .flex()
                    .flex_col()
                    .border_r_1()
                    .border_color(rgb(BORDER))
                    .child(div().p_4().child(Self::page_title("Templates", format!("{} templates", self.templates.len()))))
                    .child(list),
            )
            .child(
                div()
                    .flex_1()
                    .p_6()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(div().text_size(px(18.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(PRIMARY)).child(format!("{concept} — Web Template")))
                    .child(div().flex().flex_col().flex_1().rounded(px(6.)).border_1().border_color(rgb(BORDER)).bg(rgb(SURFACE)).child(tree)),
            )
    }

    fn aql_runner(&mut self, cx: &mut Context<Self>) -> Div {
        let editor = div()
            .p_3()
            .rounded(px(6.))
            .bg(rgb(0x11182e))
            .border_1()
            .border_color(rgb(BORDER))
            .font_family(MONO)
            .text_size(px(13.))
            .text_color(rgb(TEXT))
            .whitespace_nowrap()
            .children(self.query.lines().map(|l| {
                let mut line = div().h(px(20.));
                if l.trim_start().starts_with("SELECT") || l.trim_start().starts_with("FROM") || l.trim_start().starts_with("CONTAINS") {
                    line = line.text_color(rgb(PRIMARY));
                }
                line.child(l.replace(' ', "\u{00a0}"))
            }));
        let run = div()
            .id("run")
            .px_4()
            .h(px(34.))
            .flex()
            .items_center()
            .rounded(px(6.))
            .cursor_pointer()
            .bg(rgb(PRIMARY))
            .text_color(rgb(BG))
            .font_weight(FontWeight::SEMIBOLD)
            .text_size(px(13.))
            .on_click(cx.listener(|this, _, _, cx| {
                this.run_query(cx);
                cx.notify();
            }))
            .child(if self.aql_loading { "Running…" } else { "Run (Ctrl+Enter)" });
        let status = match &self.table {
            Some(t) => format!("{} rows · {} in {:.1} ms", t.row_count(), if self.server.is_some() { "fetched" } else { "built" }, t.exec_ms),
            None => "No results yet".to_string(),
        };
        let results = match &self.table {
            None => div().flex_1().flex().items_center().justify_center().text_color(rgb(MUTED)).child("Run a query to see results"),
            Some(t) => {
                let header = div().flex().h(px(34.)).items_center().bg(rgb(BG3)).border_b_1().border_color(rgb(BORDER)).children(
                    t.columns.iter().enumerate().map(|(i, c)| div().w(px(t.widths[i])).flex_none().px_3().text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(PRIMARY)).child(c.clone())),
                );
                let n = t.row_count();
                let body = uniform_list(
                    "aql-rows",
                    n,
                    cx.processor(move |this, range: Range<usize>, _w, _cx| {
                        let t = this.table.as_ref().unwrap();
                        range
                            .map(|i| {
                                let r = t.row(i);
                                div()
                                    .w_full()
                                    .h(px(30.))
                                    .flex()
                                    .items_center()
                                    .border_b_1()
                                    .border_color(rgb(BORDER))
                                    .bg(rgb(if i % 2 == 0 { SURFACE } else { BG2 }))
                                    .font_family(MONO)
                                    .text_size(px(12.))
                                    .text_color(rgb(TEXT))
                                    .children(r.iter().enumerate().map(|(c, v)| div().w(px(t.widths[c])).flex_none().px_3().overflow_hidden().whitespace_nowrap().text_ellipsis().child(v.clone())))
                            })
                            .collect()
                    }),
                )
                .track_scroll(self.aql_scroll.clone())
                .w_full()
                .flex_1();
                div().flex().flex_col().flex_1().rounded(px(6.)).border_1().border_color(rgb(BORDER)).overflow_hidden().child(header).child(body)
            }
        };
        div()
            .flex()
            .flex_col()
            .size_full()
            .p_6()
            .gap_3()
            .child(Self::page_title("AQL Runner", "Template context: vital_signs.v1".into()))
            .child(editor)
            .child(div().flex().items_center().gap_4().child(run).child(div().text_size(px(13.)).text_color(rgb(TEXT2)).child(status)))
            .child(results)
    }
}

impl Render for Explorer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let render_start = Instant::now();

        if !self.first_render_reported {
            self.first_render_reported = true;
            let epoch_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
            eprintln!("first_render_ms={:.1} first_render_epoch_ms={} pss_kb={}", self.boot.elapsed().as_secs_f64() * 1000.0, epoch_ms, rss_kb());
            if self.bench_scroll {
                self.bench = Some(Bench { start: Instant::now(), last: Instant::now(), frame: 0, total_frames: 300, intervals: vec![], render_costs: vec![] });
            }
        }
        if let Some(t) = self.run_clicked_at.take() {
            eprintln!("run_to_render_ms={:.1}", t.elapsed().as_secs_f64() * 1000.0);
        }

        let content = match self.view {
            View::Dashboard => self.dashboard(),
            View::Ehrs => self.ehr_browser(cx),
            View::Templates => self.template_browser(cx),
            View::Aql => self.aql_runner(cx),
        };
        let root = div()
            .flex()
            .size_full()
            .bg(rgb(BG))
            .text_color(rgb(TEXT))
            .child(self.sidebar(cx))
            .child({
                let banner = self.error.clone().map(|e| {
                    div()
                        .id("error-banner")
                        .absolute()
                        .bottom_4()
                        .left_4()
                        .right_4()
                        .p_3()
                        .rounded(px(6.))
                        .bg(rgb(0x3a1f2b))
                        .border_1()
                        .border_color(rgb(0xff7b7b))
                        .text_size(px(13.))
                        .text_color(rgb(0xff7b7b))
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.error = None;
                            cx.notify();
                        }))
                        .child(format!("{e}  (click to dismiss)"))
                });
                div().flex_1().h_full().min_w_0().relative().child(content).children(banner)
            });

        if let Some(b) = self.bench.as_mut() {
            let now = Instant::now();
            if b.frame > 0 {
                b.intervals.push(now.duration_since(b.last).as_secs_f64() * 1000.0);
                b.render_costs.push(render_start.elapsed().as_secs_f64() * 1000.0);
            }
            b.last = now;
            b.frame += 1;
            if b.frame >= b.total_frames {
                let mut iv = b.intervals.clone();
                iv.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let mut rc = b.render_costs.clone();
                rc.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let p = |v: &Vec<f64>, q: f64| v[((v.len() as f64 - 1.0) * q) as usize];
                let total = b.start.elapsed().as_secs_f64();
                println!(
                    "{{\"frames\":{},\"avg_fps\":{:.1},\"frame_ms_p50\":{:.2},\"frame_ms_p95\":{:.2},\"frame_ms_max\":{:.2},\"build_ms_p50\":{:.3},\"build_ms_p95\":{:.3},\"pss_kb\":{}}}",
                    b.frame, b.frame as f64 / total, p(&iv, 0.5), p(&iv, 0.95), iv.last().unwrap(), p(&rc, 0.5), p(&rc, 0.95), rss_kb()
                );
                cx.quit();
            }
        }
        let _ = window;
        root
    }
}

fn arg(name: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1).cloned())
}
fn flag(name: &str) -> bool {
    std::env::args().any(|x| x == name)
}

/// Build the server profile from flags/env. Secrets come from the environment only, never argv.
fn server_from_env() -> Option<net::Profile> {
    let url = arg("--server")?;
    let auth = if let Ok(t) = std::env::var("OEH_TOKEN") {
        net::Auth::Bearer(t)
    } else if let (Ok(user), Ok(password)) = (std::env::var("OEH_USER"), std::env::var("OEH_PASSWORD")) {
        net::Auth::Basic { user, password }
    } else {
        net::Auth::None
    };
    Some(net::Profile::new(&url, arg("--api-prefix"), auth))
}

fn main() {
    let boot = Instant::now();
    env_logger::init();
    let view = match arg("--view").as_deref() {
        Some("ehrs") => View::Ehrs,
        Some("templates") => View::Templates,
        Some("aql") => View::Aql,
        _ => View::Dashboard,
    };
    let ehrs: usize = arg("--ehrs").and_then(|s| s.parse().ok()).unwrap_or(3);
    let rows: usize = arg("--rows").and_then(|s| s.parse().ok()).unwrap_or(3);
    let (select, run, bench_scroll) = (flag("--select"), flag("--run"), flag("--bench-scroll"));
    let server = server_from_env();
    let query = arg("--query");
    // Opt-in only; never during benchmarks.
    let analytics = Analytics::init(flag("--analytics") && !bench_scroll, arg("--analytics-host"));
    let max_seconds: Option<u64> = arg("--max-seconds").and_then(|s| s.parse().ok()).or(bench_scroll.then_some(120));
    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds { origin: point(px(0.), px(0.)), size: size(px(1440.), px(960.)) };
        let opts = Opts { boot, view, ehr_count: ehrs, rows, select, run, bench_scroll, server, query, analytics: analytics.clone() };
        let handle = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions { title: Some("openEHR Explorer".into()), ..Default::default() }),
                    ..Default::default()
                },
                |_, cx| {
                    cx.new(|cx| {
                        let mut this = Explorer::new(opts);
                        this.start(cx);
                        this
                    })
                },
            )
            .unwrap();
        let flush = analytics.clone();
        cx.on_app_quit(move |_| {
            flush.track("app_exited", None);
            flush.flush_blocking();
            async {}
        })
        .detach();
        if bench_scroll {
            cx.spawn(async move |cx| loop {
                cx.background_executor().timer(Duration::from_millis(2)).await;
                if handle.update(cx, |this, _w, cx| this.bench_tick(cx)).is_err() {
                    break;
                }
            })
            .detach();
        }
        cx.activate(true);
        if let Some(secs) = max_seconds {
            // Safety net so scripted runs can never hang.
            cx.spawn(async move |cx| {
                cx.background_executor().timer(Duration::from_secs(secs)).await;
                cx.update(|cx| cx.quit()).ok();
            })
            .detach();
        }
    });
}
