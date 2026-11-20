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
    div, point, px, rgb, size, uniform_list, App, AppContext, Application, Bounds, ClickEvent, Context, Div, ElementId, FocusHandle, FontWeight, InteractiveElement, KeyDownEvent, Keystroke, MouseButton, Stateful,
    IntoElement, ParentElement, Render, ScrollStrategy, SharedString, StatefulInteractiveElement, Styled,
    TitlebarOptions, UniformListScrollHandle, Window, WindowBounds, WindowOptions,
};
mod analytics;
// The very same file the Tauri app compiles, so the keychain/encrypted-store formats cannot drift.
#[allow(dead_code)]
#[path = "../../src-tauri/src/credentials.rs"]
mod credentials;
mod net;
mod profiles;

use analytics::Analytics;
use serde_json::json;
use std::collections::{HashMap, HashSet};
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
    Composition,
    Templates,
    Aql,
    Servers,
}

struct Ehr {
    id: SharedString,
    created: SharedString,
    subject: SharedString,
    compositions: u32,
}

#[derive(Clone)]
struct Composition {
    uid: SharedString,
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum CompTab {
    Json,
    Flat,
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
        .map(|(n, t, c, d)| Composition { uid: SharedString::default(), name: (*n).into(), template: (*t).into(), composer: (*c).into(), committed: (*d).into() })
        .collect()
}

fn demo_tree() -> Vec<TreeRow> {
    DEMO_TREE.iter().map(|(d, l, r, p)| TreeRow { depth: *d, label: (*l).into(), rm: (*r).into(), path: (*p).into() }).collect()
}

const NAV: [(View, &str); 5] = [
    (View::Dashboard, "Dashboard"),
    (View::Ehrs, "EHR Browser"),
    (View::Templates, "Templates"),
    (View::Aql, "AQL Runner"),
    (View::Servers, "Servers"),
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
    counts: Option<net::Counts>,
    counts_error: Option<SharedString>,
    server_version: Option<SharedString>,
    comp_sel: Option<usize>,
    comp_tab: CompTab,
    comp_json: Vec<SharedString>,
    comp_flat: Vec<SharedString>,
    comp_json_err: Option<SharedString>,
    comp_flat_err: Option<SharedString>,
    comp_json_loading: bool,
    comp_flat_loading: bool,
    comp_scroll: UniformListScrollHandle,
    aql_editor: Option<Field>,
    server: Option<net::Profile>,
    server_name: Option<SharedString>,
    active_profile: Option<String>,
    profiles_ui: ProfilesUi,
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

// ---------------------------------------------------------------------------
// Server profile form: a minimal single-line text field (GPUI ships none) + form state
// ---------------------------------------------------------------------------

const F_NAME: usize = 0;
const F_URL: usize = 1;
const F_PREFIX: usize = 2;
const F_TERM: usize = 3;
const F_USER: usize = 4;
const F_PASS: usize = 5;
const F_TOKEN: usize = 6;

#[derive(Clone, Copy, PartialEq, Eq)]
enum AuthKind {
    None,
    Basic,
    Bearer,
}

/// Text input (single-line, or multi-line for the AQL editor). Cursor is counted in chars.
/// No IME/selection; supports typing, paste, arrows, Home/End and (multi-line) Enter/Up/Down/Tab.
struct Field {
    text: String,
    cursor: usize,
    masked: bool,
    multiline: bool,
    placeholder: &'static str,
    focus: FocusHandle,
}

impl Field {
    fn new(cx: &mut App, text: &str, masked: bool, placeholder: &'static str) -> Self {
        Self { cursor: text.chars().count(), text: text.to_string(), masked, multiline: false, placeholder, focus: cx.focus_handle().tab_stop(true) }
    }
    fn multiline(mut self) -> Self {
        self.multiline = true;
        self
    }
    fn byte(&self, chars: usize) -> usize {
        self.text.char_indices().nth(chars).map(|(i, _)| i).unwrap_or(self.text.len())
    }
    fn len(&self) -> usize {
        self.text.chars().count()
    }
    fn insert(&mut self, s: &str) {
        let normalized = s.replace("\r\n", "\n").replace('\r', "\n");
        let clean: String = normalized.chars().filter(|c| !c.is_control() || (self.multiline && *c == '\n')).collect();
        let at = self.byte(self.cursor);
        self.text.insert_str(at, &clean);
        self.cursor += clean.chars().count();
    }
    fn backspace(&mut self) {
        if self.cursor > 0 {
            let (a, b) = (self.byte(self.cursor - 1), self.byte(self.cursor));
            self.text.replace_range(a..b, "");
            self.cursor -= 1;
        }
    }
    fn delete(&mut self) {
        if self.cursor < self.len() {
            let (a, b) = (self.byte(self.cursor), self.byte(self.cursor + 1));
            self.text.replace_range(a..b, "");
        }
    }
    /// (line, column) of the cursor.
    fn line_col(&self) -> (usize, usize) {
        let before: String = self.text.chars().take(self.cursor).collect();
        (before.matches('\n').count(), before.rsplit('\n').next().map(|l| l.chars().count()).unwrap_or(0))
    }
    fn line_len(&self, line: usize) -> usize {
        self.text.split('\n').nth(line).map(|l| l.chars().count()).unwrap_or(0)
    }
    fn move_vertical(&mut self, delta: isize) {
        let lines: Vec<usize> = self.text.split('\n').map(|l| l.chars().count()).collect();
        let (line, col) = self.line_col();
        let target = (line as isize + delta).clamp(0, lines.len() as isize - 1) as usize;
        let start: usize = lines[..target].iter().map(|n| n + 1).sum();
        self.cursor = start + col.min(lines[target]);
    }
    fn home(&mut self) {
        let col = if self.multiline { self.line_col().1 } else { self.cursor };
        self.cursor -= col;
    }
    fn end(&mut self) {
        if self.multiline {
            let (line, col) = self.line_col();
            self.cursor += self.line_len(line) - col;
        } else {
            self.cursor = self.len();
        }
    }
    /// Apply one keystroke. `paste` is the clipboard text when the key is Ctrl/Cmd+V.
    fn handle_key(&mut self, ks: &Keystroke, paste: Option<String>) {
        let m = &ks.modifiers;
        match ks.key.as_str() {
            "backspace" => self.backspace(),
            "delete" => self.delete(),
            "left" => self.cursor = self.cursor.saturating_sub(1),
            "right" => self.cursor = (self.cursor + 1).min(self.len()),
            "home" => self.home(),
            "end" => self.end(),
            "up" if self.multiline => self.move_vertical(-1),
            "down" if self.multiline => self.move_vertical(1),
            "enter" if self.multiline => self.insert("\n"),
            "tab" if self.multiline => self.insert("  "),
            "u" if m.control => {
                self.text.clear();
                self.cursor = 0;
            }
            _ => {
                if let Some(text) = paste {
                    // Pasted secrets/URLs often carry a trailing newline.
                    self.insert(if self.multiline { &text } else { text.trim_end_matches(['\r', '\n']) });
                } else if !m.control && !m.platform && !m.alt {
                    if let Some(ch) = &ks.key_char {
                        self.insert(ch);
                    }
                }
            }
        }
    }
}

struct ProfileForm {
    /// `Some(id)` when editing a stored profile; `None` for a new one.
    editing: Option<String>,
    id: String,
    fields: Vec<Field>,
    server_type: profiles::ServerType,
    auth: AuthKind,
    /// A secret is already stored for this profile (empty password/token then means "keep").
    has_secret: bool,
    /// Admin auth is not editable here; it is kept exactly as stored.
    admin: Option<profiles::StoredAuthMethod>,
    result: Option<Result<String, String>>,
    testing: bool,
}

#[derive(Default)]
struct ProfilesUi {
    profiles: Vec<profiles::StoredProfile>,
    test: HashMap<String, Result<String, String>>,
    testing: HashSet<String>,
    confirm_delete: Option<String>,
    form: Option<ProfileForm>,
}

fn btn<F>(id: impl Into<ElementId>, label: impl Into<SharedString>, accent: bool, on_click: F) -> Stateful<Div>
where
    F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    div()
        .id(id)
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .rounded(px(6.))
        .cursor_pointer()
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(if accent { BG } else { TEXT2 }))
        .bg(rgb(if accent { PRIMARY } else { SURFACE_HOVER }))
        .hover(|s| s.opacity(0.85))
        .on_click(on_click)
        .child(label.into())
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
    server_name: Option<String>,
    active_profile: Option<String>,
    startup_error: Option<String>,
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
            counts: None,
            counts_error: None,
            server_version: None,
            comp_sel: None,
            comp_tab: CompTab::Json,
            comp_json: Vec::new(),
            comp_flat: Vec::new(),
            comp_json_err: None,
            comp_flat_err: None,
            comp_json_loading: false,
            comp_flat_loading: false,
            comp_scroll: UniformListScrollHandle::new(),
            aql_editor: None,
            conn: if demo { Conn::Demo } else { Conn::Connecting },
            server,
            server_name: o.server_name.map(SharedString::from),
            active_profile: o.active_profile,
            profiles_ui: ProfilesUi { profiles: profiles::load_stored(), ..Default::default() },
            error: o.startup_error.map(SharedString::from),
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
        self.aql_editor = Some(Field::new(cx, &self.query, false, "AQL query").multiline());
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
            let counts = server.dashboard_counts().await;
            Ok::<_, String>((desc, templates, page, counts))
        };
        self.spawn_net(cx, fut, |this, res, cx| {
            this.ehrs_loading = false;
            match res {
                Ok((desc, templates, page, counts)) => {
                    // A COUNT failure must not break the rest of the load, but it is shown on the dashboard.
                    match counts {
                        Ok(c) => {
                            this.counts = Some(c);
                            this.counts_error = None;
                        }
                        Err(e) => {
                            this.counts = None;
                            this.counts_error = Some(e.into());
                        }
                    }
                    this.server_version = Some(desc.clone().into());
                    this.analytics.track("connection_result", Some(json!({ "ok": true })));
                    let label = match &this.server_name {
                        Some(n) => format!("{n} · {desc}"),
                        None => desc,
                    };
                    this.conn = Conn::Connected(label.into());
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
        self.reset_composition();
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
                    this.comps = rows.into_iter().map(|c| Composition { uid: c.uid.into(), name: c.name.into(), template: c.template.into(), composer: c.composer.into(), committed: c.committed.into() }).collect();
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

    // ---- server profiles ------------------------------------------------------------------------

    fn reload_profiles(&mut self) {
        self.profiles_ui.profiles = profiles::load_stored();
    }

    fn open_form(&mut self, editing: Option<profiles::StoredProfile>, cx: &mut Context<Self>) {
        use profiles::StoredAuthMethod as S;
        let (id, name, url, prefix, term, user, kind, st, has_secret, admin) = match &editing {
            Some(p) => (
                p.id.clone(),
                p.name.clone(),
                p.base_url.clone(),
                p.api_path_prefix.clone().unwrap_or_default(),
                p.terminology_url.clone().unwrap_or_default(),
                if let S::Basic { username } = &p.auth_method { username.clone() } else { String::new() },
                match p.auth_method {
                    S::None => AuthKind::None,
                    S::Basic { .. } => AuthKind::Basic,
                    S::Bearer => AuthKind::Bearer,
                },
                p.server_type,
                profiles::has_stored_secret(p),
                p.admin_auth_method.clone(),
            ),
            None => (uuid::Uuid::new_v4().to_string(), String::new(), "http://localhost:8080/ehrbase".to_string(), String::new(), String::new(), String::new(), AuthKind::Basic, profiles::ServerType::Ehrbase, false, None),
        };
        let keep = "leave empty to keep the stored secret";
        let fields = vec![
            Field::new(cx, &name, false, "Local EHRBase"),
            Field::new(cx, &url, false, "http://localhost:8080/ehrbase"),
            Field::new(cx, &prefix, false, "/rest/openehr/v1 (default)"),
            Field::new(cx, &term, false, "optional terminology server URL"),
            Field::new(cx, &user, false, "username"),
            Field::new(cx, "", true, if has_secret { keep } else { "password" }),
            Field::new(cx, "", true, if has_secret { keep } else { "token" }),
        ];
        // Editing keeps an empty api_path_prefix field = default only if it was stored as None; "" is a real value.
        self.profiles_ui.form = Some(ProfileForm { editing: editing.map(|p| p.id), id, fields, server_type: st, auth: kind, has_secret, admin, result: None, testing: false });
        self.profiles_ui.confirm_delete = None;
    }

    /// Validate the form into the same input struct the Tauri `save_server_profile` takes.
    fn form_input(&self) -> Result<profiles::ProfileInput, String> {
        use profiles::AuthMethod as A;
        let f = self.profiles_ui.form.as_ref().ok_or("No form open")?;
        let t = |i: usize| f.fields[i].text.trim().to_string();
        let name = t(F_NAME);
        let url = t(F_URL);
        if name.is_empty() {
            return Err("Name is required".into());
        }
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err("Base URL must start with http:// or https://".into());
        }
        let auth = match f.auth {
            AuthKind::None => A::None,
            AuthKind::Basic => {
                if t(F_USER).is_empty() {
                    return Err("Username is required for Basic auth".into());
                }
                A::Basic { username: t(F_USER), password: f.fields[F_PASS].text.clone() }
            }
            AuthKind::Bearer => A::Bearer { token: f.fields[F_TOKEN].text.clone() },
        };
        // Admin auth isn't editable in this PoC: pass the stored shape with empty secrets so stored ones are kept.
        let admin = f.admin.as_ref().map(|a| match a {
            profiles::StoredAuthMethod::None => A::None,
            profiles::StoredAuthMethod::Basic { username } => A::Basic { username: username.clone(), password: String::new() },
            profiles::StoredAuthMethod::Bearer => A::Bearer { token: String::new() },
        });
        let prefix = t(F_PREFIX);
        Ok(profiles::ProfileInput {
            id: f.id.clone(),
            name,
            base_url: url,
            server_type: f.server_type,
            auth_method: auth,
            admin_auth_method: admin,
            terminology_url: Some(t(F_TERM)).filter(|s| !s.is_empty()),
            // Blank = use the default (None); the Tauri form distinguishes it the same way.
            api_path_prefix: if prefix.is_empty() { None } else { Some(prefix) },
        })
    }

    fn save_form(&mut self, cx: &mut Context<Self>) {
        let input = match self.form_input() {
            Ok(i) => i,
            Err(e) => {
                if let Some(f) = self.profiles_ui.form.as_mut() {
                    f.result = Some(Err(e));
                }
                return;
            }
        };
        let id = input.id.clone();
        match profiles::save(input) {
            Ok(list) => {
                self.profiles_ui.profiles = list;
                self.profiles_ui.form = None;
                // Like the Tauri store: the first saved profile becomes active if none is.
                if self.active_profile.is_none() {
                    self.use_profile(&id, cx);
                } else if self.active_profile.as_deref() == Some(id.as_str()) {
                    // Edited the active profile: reconnect with the new settings.
                    self.use_profile(&id, cx);
                }
            }
            Err(e) => {
                if let Some(f) = self.profiles_ui.form.as_mut() {
                    f.result = Some(Err(e));
                }
            }
        }
    }

    fn test_form(&mut self, cx: &mut Context<Self>) {
        let input = match self.form_input() {
            Ok(i) => i,
            Err(e) => {
                if let Some(f) = self.profiles_ui.form.as_mut() {
                    f.result = Some(Err(e));
                }
                return;
            }
        };
        // "Leave empty to keep": fall back to the stored secret, like `test_unsaved_connection`.
        let mut sp = profiles::ServerProfile {
            id: input.id.clone(),
            name: input.name.clone(),
            base_url: input.base_url.clone(),
            server_type: input.server_type,
            auth_method: input.auth_method.clone(),
            api_path_prefix: profiles::normalize_api_path_prefix(input.api_path_prefix.clone()),
        };
        if let Some(stored) = self.profiles_ui.profiles.iter().find(|p| p.id == input.id) {
            if let Ok(resolved) = profiles::resolve(stored) {
                match (&mut sp.auth_method, resolved.auth_method) {
                    (profiles::AuthMethod::Basic { password, .. }, profiles::AuthMethod::Basic { password: old, .. }) if password.is_empty() => *password = old,
                    (profiles::AuthMethod::Bearer { token }, profiles::AuthMethod::Bearer { token: old }) if token.is_empty() => *token = old,
                    _ => {}
                }
            }
        }
        if let profiles::AuthMethod::Bearer { token } = &mut sp.auth_method {
            match profiles::normalize_bearer_token(token) {
                Ok(t) => *token = t,
                Err(e) => {
                    if let Some(f) = self.profiles_ui.form.as_mut() {
                        f.result = Some(Err(e));
                    }
                    return;
                }
            }
        }
        if let Some(f) = self.profiles_ui.form.as_mut() {
            f.testing = true;
            f.result = None;
        }
        let stype = sp.server_type.as_str();
        let net_profile = net::Profile::from_server(&sp);
        let id = input.id;
        self.spawn_net(cx, async move { net_profile.test_http().await }, move |this, res, _cx| {
            if res.is_ok() {
                // Coarse event only (CDR platform type), same as the Tauri app.
                this.analytics.track("server_connected", Some(json!({ "server_type": stype })));
            }
            if let Some(f) = this.profiles_ui.form.as_mut() {
                if f.id == id {
                    f.testing = false;
                    f.result = Some(res);
                }
            }
        });
    }

    fn test_profile(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(stored) = self.profiles_ui.profiles.iter().find(|p| p.id == id).cloned() else { return };
        let sp = match profiles::resolve(&stored) {
            Ok(p) => p,
            Err(e) => {
                self.profiles_ui.test.insert(id.to_string(), Err(e));
                return;
            }
        };
        let stype = sp.server_type.as_str();
        let np = net::Profile::from_server(&sp);
        let id = id.to_string();
        self.profiles_ui.testing.insert(id.clone());
        self.spawn_net(cx, async move { np.test_http().await }, move |this, res, _cx| {
            if res.is_ok() {
                this.analytics.track("server_connected", Some(json!({ "server_type": stype })));
            }
            this.profiles_ui.testing.remove(&id);
            this.profiles_ui.test.insert(id, res);
        });
    }

    /// Make a stored profile the active one and (re)load all data from it.
    fn use_profile(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(stored) = self.profiles_ui.profiles.iter().find(|p| p.id == id).cloned() else { return };
        match profiles::resolve(&stored) {
            Ok(p) => {
                self.server = Some(net::Profile::from_server(&p));
                self.server_name = Some(p.name.into());
                self.active_profile = Some(stored.id);
                self.ehrs.clear();
                self.selected_ehr = None;
                self.comps.clear();
                self.templates.clear();
                self.tree.clear();
                self.selected_template = 0;
                self.table = None;
                self.counts = None;
                self.counts_error = None;
                self.server_version = None;
                self.reset_composition();
                self.ehrs_has_more = false;
                self.connect(cx);
            }
            Err(e) => self.error = Some(e.into()),
        }
    }

    fn delete_profile(&mut self, id: &str) {
        match profiles::delete(id) {
            Ok(list) => {
                self.profiles_ui.profiles = list;
                self.profiles_ui.test.remove(id);
                if self.active_profile.as_deref() == Some(id) {
                    // Like the Tauri store: the app keeps running on whatever was loaded, but no profile is "active".
                    self.active_profile = None;
                }
            }
            Err(e) => self.error = Some(e.into()),
        }
        self.profiles_ui.confirm_delete = None;
    }

    fn toggle_default(&mut self, id: &str) {
        match profiles::set_default(id) {
            Ok(list) => self.profiles_ui.profiles = list,
            Err(e) => self.error = Some(e.into()),
        }
    }

    fn form_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        let m = &ks.modifiers;
        match ks.key.as_str() {
            "tab" => {
                if m.shift {
                    window.focus_prev();
                } else {
                    window.focus_next();
                }
                cx.notify();
                return;
            }
            "enter" => {
                self.save_form(cx);
                cx.notify();
                return;
            }
            "escape" => {
                self.profiles_ui.form = None;
                cx.notify();
                return;
            }
            _ => {}
        }
        let paste = if (m.control || m.platform) && ks.key == "v" { cx.read_from_clipboard().and_then(|c| c.text()) } else { None };
        let Some(form) = self.profiles_ui.form.as_mut() else { return };
        let Some(f) = form.fields.iter_mut().find(|f| f.focus.is_focused(window)) else { return };
        f.handle_key(ks, paste);
        cx.notify();
        cx.stop_propagation();
    }

    fn field_view(&self, form: &ProfileForm, idx: usize, label: &'static str, window: &Window) -> Div {
        let f = &form.fields[idx];
        let focused = f.focus.is_focused(window);
        let shown = |t: &str| -> String {
            if f.masked {
                "•".repeat(t.chars().count())
            } else {
                t.replace(' ', "\u{00a0}")
            }
        };
        let (before, after): (String, String) = (f.text.chars().take(f.cursor).collect(), f.text.chars().skip(f.cursor).collect());
        let handle = f.focus.clone();
        let content = if f.text.is_empty() && !focused {
            div().text_color(rgb(MUTED)).child(f.placeholder)
        } else {
            div()
                .flex()
                .items_center()
                .text_color(rgb(TEXT))
                .child(shown(&before))
                .children(focused.then(|| div().w(px(1.5)).h(px(17.)).bg(rgb(PRIMARY))))
                .child(shown(&after))
        };
        div().flex().flex_col().gap_1().child(div().text_size(px(12.)).text_color(rgb(MUTED)).child(label)).child(
            div()
                .id(("field", idx))
                .track_focus(&f.focus)
                .h(px(34.))
                .px_3()
                .flex()
                .items_center()
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(if focused { PRIMARY } else { BORDER }))
                .bg(rgb(0x11182e))
                .text_size(px(13.))
                .overflow_hidden()
                .cursor_text()
                .on_mouse_down(MouseButton::Left, move |_, window, _cx| handle.focus(window))
                .child(content),
        )
    }

    /// A pill toggle (server type / auth kind).
    fn chip<F>(id: &'static str, label: String, on: bool, apply: F, cx: &mut Context<Self>) -> Stateful<Div>
    where
        F: Fn(&mut ProfileForm) + 'static,
    {
        div()
            .id(id)
            .px_3()
            .h(px(28.))
            .flex()
            .items_center()
            .rounded(px(14.))
            .cursor_pointer()
            .text_size(px(12.))
            .border_1()
            .border_color(rgb(if on { PRIMARY } else { BORDER }))
            .text_color(rgb(if on { PRIMARY } else { TEXT2 }))
            .bg(rgb(if on { BG3 } else { SURFACE }))
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(f) = this.profiles_ui.form.as_mut() {
                    apply(f);
                }
                cx.notify();
            }))
            .child(label)
    }

    fn labelled(label: &'static str, body: Div) -> Div {
        div().flex().flex_col().gap_1().child(div().text_size(px(12.)).text_color(rgb(MUTED)).child(label)).child(body)
    }

    fn type_chips(form: &ProfileForm, cx: &mut Context<Self>) -> Div {
        const IDS: [&str; 4] = ["st0", "st1", "st2", "st3"];
        let mut row = div().flex().gap_2();
        for (i, t) in profiles::ServerType::ALL.iter().copied().enumerate() {
            row = row.child(Self::chip(IDS[i], t.as_str().to_string(), form.server_type == t, move |f| f.server_type = t, cx));
        }
        row
    }

    fn auth_chips(form: &ProfileForm, cx: &mut Context<Self>) -> Div {
        let mut row = div().flex().gap_2();
        for (id, label, kind) in [("au0", "None", AuthKind::None), ("au1", "Basic", AuthKind::Basic), ("au2", "Bearer", AuthKind::Bearer)] {
            row = row.child(Self::chip(id, label.to_string(), form.auth == kind, move |f| f.auth = kind, cx));
        }
        row
    }

    fn form_buttons(form: &ProfileForm, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .gap_2()
            .child(btn("form-test", if form.testing { "Testing…" } else { "Test Connection" }, false, cx.listener(|this, _, _, cx| {
                this.test_form(cx);
                cx.notify();
            })))
            .child(btn("form-save", "Save", true, cx.listener(|this, _, _, cx| {
                this.save_form(cx);
                cx.notify();
            })))
            .child(btn("form-cancel", "Cancel", false, cx.listener(|this, _, _, cx| {
                this.profiles_ui.form = None;
                cx.notify();
            })))
    }

    fn profile_form_panel(&self, form: &ProfileForm, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let basic = form.auth == AuthKind::Basic;
        let bearer = form.auth == AuthKind::Bearer;
        let secret_label = |plain: &'static str, stored: &'static str| if form.has_secret { stored } else { plain };
        let result = form.result.as_ref().map(|r| {
            let (color, msg) = match r {
                Ok(m) => (SUCCESS, m.clone()),
                Err(e) => (0xff7b7b, e.clone()),
            };
            div().text_size(px(13.)).text_color(rgb(color)).child(msg)
        });
        div()
            .id("profile-form")
            .flex_1()
            .p_6()
            .flex()
            .flex_col()
            .gap_3()
            .overflow_y_scroll()
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| this.form_key(ev, window, cx)))
            .child(div().text_size(px(18.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(PRIMARY)).child(if form.editing.is_some() { "Edit Server" } else { "Add Server" }))
            .child(self.field_view(form, F_NAME, "Name", window))
            .child(self.field_view(form, F_URL, "Base URL", window))
            .child(Self::labelled("Server type", Self::type_chips(form, cx)))
            .child(Self::labelled("Authentication", Self::auth_chips(form, cx)))
            .children(basic.then(|| self.field_view(form, F_USER, "Username", window)))
            .children(basic.then(|| self.field_view(form, F_PASS, secret_label("Password", "Password (stored)"), window)))
            .children(bearer.then(|| self.field_view(form, F_TOKEN, secret_label("Bearer token", "Bearer token (stored)"), window)))
            .child(self.field_view(form, F_PREFIX, "API path prefix", window))
            .child(self.field_view(form, F_TERM, "Terminology URL", window))
            .children(form.admin.as_ref().map(|_| div().text_size(px(12.)).text_color(rgb(MUTED)).child("Admin credentials are kept as stored (edit them in the Tauri app).")))
            .children(result)
            .child(Self::form_buttons(form, cx))
    }

    fn card_header(p: &profiles::StoredProfile, active: bool) -> Div {
        div()
            .flex()
            .items_center()
            .justify_between()
            .child(div().text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(TEXT)).child(p.name.clone()))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .children(p.is_default.then(|| div().text_size(px(11.)).text_color(rgb(0xffd93d)).child("★ default")))
                    .children(active.then(|| div().text_size(px(11.)).text_color(rgb(PRIMARY)).child("● active")))
                    .child(div().text_size(px(11.)).font_family(MONO).text_color(rgb(MUTED)).child(p.server_type.as_str())),
            )
    }

    fn card_actions(&self, p: &profiles::StoredProfile, active: bool, cx: &mut Context<Self>) -> Div {
        let confirming = self.profiles_ui.confirm_delete.as_deref() == Some(p.id.as_str());
        let (i1, i2, i3, i4, i5) = (p.id.clone(), p.id.clone(), p.id.clone(), p.id.clone(), p.id.clone());
        let ps = p.clone();
        div()
            .flex()
            .gap_2()
            .child(btn(SharedString::from(format!("test-{i1}")), "Test", false, cx.listener(move |this, _, _, cx| {
                this.test_profile(&i1, cx);
                cx.notify();
            })))
            .child(btn(SharedString::from(format!("use-{i2}")), if active { "Reload" } else { "Use" }, active, cx.listener(move |this, _, _, cx| {
                this.use_profile(&i2, cx);
                cx.notify();
            })))
            .child(btn(SharedString::from(format!("def-{i3}")), if p.is_default { "Unset default" } else { "Set default" }, false, cx.listener(move |this, _, _, cx| {
                this.toggle_default(&i3);
                cx.notify();
            })))
            .child(btn(SharedString::from(format!("edit-{i4}")), "Edit", false, cx.listener(move |this, _, _, cx| {
                this.open_form(Some(ps.clone()), cx);
                cx.notify();
            })))
            .child(btn(SharedString::from(format!("del-{i5}")), if confirming { "Confirm delete" } else { "Delete" }, false, cx.listener(move |this, _, _, cx| {
                if this.profiles_ui.confirm_delete.as_deref() == Some(i5.as_str()) {
                    this.delete_profile(&i5);
                } else {
                    this.profiles_ui.confirm_delete = Some(i5.clone());
                }
                cx.notify();
            })))
    }

    /// The last connection-test line for a card: (colour, text).
    fn test_line(&self, id: &str) -> Option<(u32, String)> {
        if self.profiles_ui.testing.contains(id) {
            return Some((MUTED, "Testing…".to_string()));
        }
        self.profiles_ui.test.get(id).map(|r| match r {
            Ok(m) => (SUCCESS, m.clone()),
            Err(e) => (0xff7b7b, e.clone()),
        })
    }

    fn profile_card(&self, p: &profiles::StoredProfile, cx: &mut Context<Self>) -> Div {
        let active = self.active_profile.as_deref() == Some(p.id.as_str());
        let auth = match &p.auth_method {
            profiles::StoredAuthMethod::None => "no auth".to_string(),
            profiles::StoredAuthMethod::Basic { username } => format!("basic · {username}"),
            profiles::StoredAuthMethod::Bearer => "bearer".to_string(),
        };
        div()
            .p_3()
            .rounded(px(6.))
            .bg(rgb(SURFACE))
            .border_1()
            .border_color(rgb(if active { PRIMARY } else { BORDER }))
            .flex()
            .flex_col()
            .gap_2()
            .child(Self::card_header(p, active))
            .child(div().text_size(px(12.)).font_family(MONO).text_color(rgb(TEXT2)).whitespace_nowrap().overflow_hidden().text_ellipsis().child(p.base_url.clone()))
            .child(div().text_size(px(11.)).text_color(rgb(MUTED)).child(auth))
            .children(self.test_line(&p.id).map(|(c, m)| div().text_size(px(12.)).text_color(rgb(c)).child(m)))
            .child(self.card_actions(p, active, cx))
    }

    fn servers_view(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let list: Vec<profiles::StoredProfile> = self.profiles_ui.profiles.clone();
        let backend = if list.is_empty() { "—" } else { profiles::credential_backend() };
        let mut cards = div().flex().flex_col().gap_2().px_4().pb_4();
        for p in list.iter() {
            cards = cards.child(self.profile_card(p, cx));
        }
        if list.is_empty() {
            cards = cards.child(div().p_4().text_size(px(13.)).text_color(rgb(MUTED)).child("No server profiles yet. Profiles are shared with the Tauri app (same profiles.json and credential store)."));
        }
        let header = div()
            .p_4()
            .flex()
            .items_start()
            .justify_between()
            .child(Self::page_title("Servers", format!("{} profiles · credentials: {backend}", list.len())))
            .child(btn("add-server", "Add Server", true, cx.listener(|this, _, _, cx| {
                this.open_form(None, cx);
                cx.notify();
            })));
        let left = div()
            .w(px(520.))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(rgb(BORDER))
            .child(header)
            .child(div().id("profile-list").flex_1().overflow_y_scroll().child(cards));
        let right = match &self.profiles_ui.form {
            None => div().id("profile-form-empty").flex_1().flex().items_center().justify_center().text_color(rgb(MUTED)).child("Select Edit on a profile or Add Server"),
            Some(form) => self.profile_form_panel(form, window, cx),
        };
        div().flex().size_full().child(left).child(right)
    }

    // ---- compositions / editor / labels ---------------------------------------------------------

    /// Which server the data on screen comes from, for tab headers.
    fn server_label(&self) -> String {
        match (&self.server, &self.server_name) {
            (Some(_), Some(n)) => format!("{n} · {}", self.server.as_ref().map(|s| s.base_url.as_str()).unwrap_or("")),
            (Some(s), None) => s.base_url.clone(),
            (None, _) => "demo data — pick a server in Servers".to_string(),
        }
    }

    fn reset_composition(&mut self) {
        self.comp_sel = None;
        self.comp_json.clear();
        self.comp_flat.clear();
        self.comp_json_err = None;
        self.comp_flat_err = None;
        self.comp_json_loading = false;
        self.comp_flat_loading = false;
    }

    fn json_lines(v: &serde_json::Value) -> Vec<SharedString> {
        // Non-breaking spaces so indentation survives text layout.
        serde_json::to_string_pretty(v).unwrap_or_default().lines().map(|l| SharedString::from(l.replace(' ', "\u{00a0}"))).collect()
    }

    /// Metadata-only stand-in so the Compositions tab is usable in demo mode.
    fn demo_composition(c: &Composition) -> (serde_json::Value, serde_json::Value) {
        let canonical = json!({
            "_type": "COMPOSITION",
            "name": { "_type": "DV_TEXT", "value": c.name },
            "archetype_details": { "template_id": { "value": c.template } },
            "composer": { "_type": "PARTY_IDENTIFIED", "name": c.composer },
            "context": { "start_time": { "value": c.committed } },
        });
        let flat = json!({
            "ctx/language": "en", "ctx/territory": "DE", "ctx/composer_name": c.composer,
            "ctx/time": c.committed, "ctx/template_id": c.template,
        });
        (canonical, flat)
    }

    fn open_composition(&mut self, i: usize, cx: &mut Context<Self>) {
        let Some(c) = self.comps.get(i).cloned() else { return };
        self.view = View::Composition;
        self.comp_sel = Some(i);
        self.comp_json_err = None;
        self.comp_flat_err = None;
        let Some(server) = self.server.clone() else {
            let (j, f) = Self::demo_composition(&c);
            self.comp_json = Self::json_lines(&j);
            self.comp_flat = Self::json_lines(&f);
            return;
        };
        let Some(ehr) = self.selected_ehr.and_then(|e| self.ehrs.get(e)).map(|e| e.id.to_string()) else { return };
        self.comp_json.clear();
        self.comp_flat.clear();
        self.comp_json_loading = true;
        self.comp_flat_loading = true;
        let uid = c.uid.to_string();
        self.analytics.track("composition_viewed", None);
        // Two independent requests: JSON shows as soon as it arrives, and FLAT (optional per server/template)
        // reports its own error without blocking or failing the other.
        let (s2, e2, u2) = (server.clone(), ehr.clone(), uid.clone());
        self.spawn_net(cx, async move { server.composition(&ehr, &uid).await }, move |this, res, _cx| {
            if this.comp_sel != Some(i) {
                return;
            }
            this.comp_json_loading = false;
            match res {
                Ok(v) => this.comp_json = Self::json_lines(&v),
                Err(e) => this.comp_json_err = Some(e.into()),
            }
        });
        self.spawn_net(cx, async move { s2.composition_flat(&e2, &u2).await }, move |this, res, _cx| {
            if this.comp_sel != Some(i) {
                return;
            }
            this.comp_flat_loading = false;
            match res {
                Ok(v) => this.comp_flat = Self::json_lines(&v),
                Err(e) => this.comp_flat_err = Some(e.into()),
            }
        });
    }

    fn run_from_editor(&mut self, cx: &mut Context<Self>) {
        if let Some(ed) = &self.aql_editor {
            self.query = ed.text.clone().into();
        }
        self.run_query(cx);
    }

    fn editor_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        let m = &ks.modifiers;
        if ks.key == "enter" && (m.control || m.platform) {
            self.run_from_editor(cx);
            cx.notify();
            cx.stop_propagation();
            return;
        }
        let paste = if (m.control || m.platform) && ks.key == "v" { cx.read_from_clipboard().and_then(|c| c.text()) } else { None };
        let Some(ed) = self.aql_editor.as_mut() else { return };
        if !ed.focus.is_focused(window) {
            return;
        }
        ed.handle_key(ks, paste);
        cx.notify();
        cx.stop_propagation();
    }

    fn editor_view(&self, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let nb = |s: &str| s.replace(' ', "\u{00a0}");
        let base = div()
            .id("aql-editor")
            .p_3()
            .rounded(px(6.))
            .bg(rgb(0x11182e))
            .border_1()
            .font_family(MONO)
            .text_size(px(13.))
            .max_h(px(230.))
            .overflow_y_scroll()
            .cursor_text();
        let Some(ed) = self.aql_editor.as_ref() else { return base.border_color(rgb(BORDER)) };
        let focused = ed.focus.is_focused(window);
        let (cur_line, cur_col) = ed.line_col();
        let handle = ed.focus.clone();
        let rows = ed.text.split('\n').enumerate().map(|(i, line)| {
            let kw = ["SELECT", "FROM", "CONTAINS", "WHERE", "ORDER", "LIMIT", "OFFSET"].iter().any(|k| line.trim_start().to_ascii_uppercase().starts_with(k));
            let row = div().h(px(20.)).flex().items_center().whitespace_nowrap().text_color(rgb(if kw { PRIMARY } else { TEXT }));
            if focused && i == cur_line {
                let (before, after): (String, String) = (line.chars().take(cur_col).collect(), line.chars().skip(cur_col).collect());
                row.child(nb(&before)).child(div().w(px(1.5)).h(px(17.)).bg(rgb(PRIMARY))).child(nb(&after))
            } else {
                row.child(nb(line))
            }
        });
        base.track_focus(&ed.focus)
            .border_color(rgb(if focused { PRIMARY } else { BORDER }))
            .on_mouse_down(MouseButton::Left, move |_, window, _cx| handle.focus(window))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| this.editor_key(ev, window, cx)))
            .children(rows)
    }

    /// The composition screen: opened by clicking a composition in EHR Browser, Back returns there.
    fn compositions_view(&mut self, cx: &mut Context<Self>) -> Div {
        div().flex().size_full().child(self.composition_viewer(cx))
    }

    fn back_to_ehrs(cx: &mut Context<Self>) -> Stateful<Div> {
        btn("back-ehrs", "← EHR Browser", false, cx.listener(|this, _, _, cx| {
            this.view = View::Ehrs;
            cx.notify();
        }))
    }

    fn composition_viewer(&mut self, cx: &mut Context<Self>) -> Div {
        let Some(c) = self.comp_sel.and_then(|i| self.comps.get(i)).cloned() else {
            return div().flex_1().p_6().flex().flex_col().gap_3().child(Self::back_to_ehrs(cx)).child(div().text_color(rgb(MUTED)).child("Open a composition from an EHR in EHR Browser"));
        };
        let tab = self.comp_tab;
        let ehr_short: String = self.selected_ehr.and_then(|i| self.ehrs.get(i)).map(|e| e.id.chars().take(8).collect()).unwrap_or_default();
        let (lines, err, loading) = match tab {
            CompTab::Json => (&self.comp_json, &self.comp_json_err, self.comp_json_loading),
            CompTab::Flat => (&self.comp_flat, &self.comp_flat_err, self.comp_flat_loading),
        };
        let body: Div = if loading {
            div().p_4().text_color(rgb(MUTED)).child("Loading…")
        } else if let Some(e) = err {
            div().p_4().text_size(px(13.)).text_color(rgb(0xff7b7b)).child(format!("{} view unavailable: {e}", if tab == CompTab::Json { "JSON" } else { "FLAT" }))
        } else {
            let n = lines.len();
            div().flex().flex_col().flex_1().child(
                uniform_list(
                    "comp-lines",
                    n,
                    cx.processor(|this, range: Range<usize>, _w, _cx| {
                        let lines = if this.comp_tab == CompTab::Json { &this.comp_json } else { &this.comp_flat };
                        range.map(|i| div().h(px(20.)).px_3().font_family(MONO).text_size(px(12.)).text_color(rgb(TEXT)).whitespace_nowrap().child(lines[i].clone())).collect()
                    }),
                )
                .track_scroll(self.comp_scroll.clone())
                .w_full()
                .flex_1(),
            )
        };
        let tabs = div()
            .flex()
            .gap_2()
            .child(btn("tab-json", "JSON", tab == CompTab::Json, cx.listener(|this, _, _, cx| {
                this.comp_tab = CompTab::Json;
                cx.notify();
            })))
            .child(btn("tab-flat", "FLAT", tab == CompTab::Flat, cx.listener(|this, _, _, cx| {
                this.comp_tab = CompTab::Flat;
                cx.notify();
            })));
        div()
            .flex_1()
            .p_6()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_size(px(18.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(PRIMARY)).child(c.name))
                            .child(div().text_size(px(12.)).font_family(MONO).text_color(rgb(MUTED)).child(if c.uid.is_empty() { c.template.clone() } else { c.uid.clone() }))
                            .child(div().text_size(px(12.)).text_color(rgb(MUTED)).child(format!("EHR {} · {}", ehr_short, self.server_label()))),
                    )
                    .child(Self::back_to_ehrs(cx)),
            )
            .child(tabs)
            .child(div().flex().flex_col().flex_1().rounded(px(6.)).border_1().border_color(rgb(BORDER)).bg(rgb(SURFACE)).overflow_hidden().child(body))
    }

    fn dashboard_numbers(&self) -> (String, String, String) {
        if self.server.is_none() {
            let comps: u32 = self.ehrs.iter().map(|e| e.compositions).sum();
            return (self.ehrs.len().to_string(), comps.to_string(), self.templates.len().to_string());
        }
        match (&self.counts, &self.conn) {
            (Some(c), _) => (c.ehrs.to_string(), c.compositions.to_string(), c.templates.to_string()),
            (None, Conn::Connecting) => ("…".to_string(), "…".to_string(), "…".to_string()),
            // COUNT is unsupported on some CDRs: fall back to what is loaded, and say so with a dash for the rest.
            (None, _) => (format!("{}{}", self.ehrs.len(), if self.ehrs_has_more { "+" } else { "" }), "—".to_string(), self.templates.len().to_string()),
        }
    }

    /// Title row with a Refresh button (server mode only); re-runs the connection and all loads.
    fn titled_header(&self, title: &'static str, subtitle: String, padded: bool, cx: &mut Context<Self>) -> Div {
        let refresh = self.server.is_some().then(|| {
            btn(SharedString::from(format!("refresh-{title}")), if matches!(self.conn, Conn::Connecting) { "Loading…" } else { "Refresh" }, false, cx.listener(|this, _, _, cx| {
                this.connect(cx);
                cx.notify();
            }))
        });
        div().p(px(if padded { 16. } else { 0. })).flex().items_start().justify_between().child(Self::page_title(title, subtitle)).children(refresh)
    }

    fn page_title(title: &'static str, subtitle: String) -> Div {
        div().flex().flex_col().gap_1().pb_4().child(div().text_size(px(22.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(PRIMARY)).child(title)).child(div().text_size(px(13.)).text_color(rgb(MUTED)).child(subtitle))
    }

    fn sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut nav = div().flex().flex_col().gap_1().p_3();
        for (view, label) in NAV {
            // The composition viewer is a screen inside EHR Browser (as in the Tauri app), not a tab of its own.
            let active = self.view == view || (view == View::Ehrs && self.view == View::Composition);
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
                    .child(div().flex().items_center().gap_2().child(div().size(px(8.)).flex_none().rounded_full().bg(rgb(dot))).child(div().text_size(px(12.)).text_color(rgb(TEXT2)).child(text)))
                    .child(div().text_size(px(12.)).text_color(rgb(MUTED)).child(if self.analytics.enabled() { "v0.10.3 · gpui · analytics on" } else { "v0.10.3 · gpui" }))
            })
    }

    /// A stat card that links to the view it counts (as in the Tauri dashboard).
    fn stat_card(label: &'static str, value: String, target: View, cx: &mut Context<Self>) -> Stateful<Div> {
        div()
            .id(SharedString::from(format!("stat-{label}")))
            .flex_1()
            .p_4()
            .rounded(px(6.))
            .bg(rgb(SURFACE))
            .border_1()
            .border_color(rgb(BORDER))
            .flex()
            .flex_col()
            .gap_1()
            .cursor_pointer()
            .hover(|s| s.bg(rgb(SURFACE_HOVER)).border_color(rgb(PRIMARY)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.view = target;
                cx.notify();
            }))
            .child(div().text_size(px(30.)).font_weight(FontWeight::BOLD).text_color(rgb(PRIMARY)).child(value))
            .child(div().text_size(px(12.)).text_color(rgb(MUTED)).child(label))
    }

    /// Shown instead of live data when no profile is active: send the user to Servers.
    fn onboarding_card(&self, cx: &mut Context<Self>) -> Div {
        let (title, body, button) = if self.profiles_ui.profiles.is_empty() {
            ("Welcome to openEHR Explorer", "Connect your first openEHR CDR (EHRBase, Better Platform, FerroEHR or any generic openEHR REST server) to see live EHR, composition and template counts here.", "+ Add Your First Server")
        } else {
            ("No server selected", "Select a server profile to see its live EHR, composition and template counts.", "Go to Servers")
        };
        div()
            .p_4()
            .rounded(px(6.))
            .bg(rgb(SURFACE))
            .border_1()
            .border_color(rgb(BORDER))
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_size(px(16.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(TEXT)).child(title))
            .child(div().text_size(px(13.)).text_color(rgb(TEXT2)).child(body))
            .child(div().flex().child(btn("onboard-servers", button, true, cx.listener(|this, _, _, cx| {
                this.view = View::Servers;
                cx.notify();
            }))))
    }

    /// "Connected Server" card (links to Servers): name, URL, type, status, version.
    fn server_card(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let row = |k: &'static str, v: Div| div().flex().items_center().gap_3().child(div().w(px(70.)).flex_none().text_size(px(12.)).text_color(rgb(MUTED)).child(k)).child(v);
        let text = |t: String| div().text_size(px(13.)).text_color(rgb(TEXT)).child(t);
        let (status, color) = match &self.conn {
            Conn::Connected(_) => ("connected", SUCCESS),
            Conn::Connecting => ("connecting", 0xffd93d),
            Conn::Failed(_) => ("error", 0xff7b7b),
            Conn::Demo => ("unknown", MUTED),
        };
        let status_row = div().flex().items_center().gap_2().child(div().size(px(8.)).flex_none().rounded_full().bg(rgb(color))).child(text(status.to_string()));
        let url = self.server.as_ref().map(|s| s.base_url.clone()).unwrap_or_default();
        let stype = self.server.as_ref().map(|s| s.type_name()).unwrap_or("generic");
        div()
            .id("server-card")
            .p_4()
            .rounded(px(6.))
            .bg(rgb(SURFACE))
            .border_1()
            .border_color(rgb(BORDER))
            .flex()
            .flex_col()
            .gap_2()
            .cursor_pointer()
            .hover(|s| s.bg(rgb(SURFACE_HOVER)).border_color(rgb(PRIMARY)))
            .on_click(cx.listener(|this, _, _, cx| {
                this.view = View::Servers;
                cx.notify();
            }))
            .child(div().text_size(px(15.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(PRIMARY)).child("Connected Server"))
            .child(row("Name", text(self.server_name.as_ref().map(|n| n.to_string()).unwrap_or_else(|| "(from --server)".to_string()))))
            .child(row("URL", div().text_size(px(13.)).font_family(MONO).text_color(rgb(TEXT)).child(url)))
            .child(row("Type", div().text_size(px(11.)).font_family(MONO).text_color(rgb(MUTED)).child(stype)))
            .child(row("Status", status_row))
            .children(self.server_version.clone().map(|v| row("Version", text(v.to_string()))))
    }

    fn dashboard(&self, cx: &mut Context<Self>) -> Div {
        let demo = self.server.is_none();
        let (ehr_count, comps, tpl_count) = self.dashboard_numbers();
        let subtitle = if demo { format!("Local EHRBase · hospital.example.org ({})", self.server_label()) } else { self.server_label() };
        div()
            .flex()
            .flex_col()
            .p_6()
            .gap_4()
            .child(self.titled_header("Overview", subtitle, false, cx))
            .children(demo.then(|| self.onboarding_card(cx)))
            .children(self.counts_error.clone().map(|e| div().text_size(px(13.)).text_color(rgb(0xff7b7b)).child(format!("Failed to load counts: {e}"))))
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(Self::stat_card("EHRs", ehr_count, View::Ehrs, cx))
                    .child(Self::stat_card("Compositions", comps, View::Ehrs, cx))
                    .child(Self::stat_card("Templates", tpl_count, View::Templates, cx)),
            )
            .children((!demo).then(|| self.server_card(cx)))
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
            .child(self.titled_header("EHR Browser", format!("{count}{} EHRs · {}", if self.ehrs_has_more { "+" } else { "" }, self.server_label()), true, cx))
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
                for (ci, c) in self.comps.iter().enumerate() {
                    comps = comps.child(
                        div()
                            .id(("comp-card", ci))
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(SURFACE_HOVER)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_composition(ci, cx);
                                cx.notify();
                            }))
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
                    .child(div().p_4().child(Self::page_title("Templates", format!("{} templates · {}", self.templates.len(), self.server_label()))))
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

    fn aql_runner(&mut self, window: &Window, cx: &mut Context<Self>) -> Div {
        let editor = self.editor_view(window, cx);
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
                this.run_from_editor(cx);
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
            .child(Self::page_title("AQL Runner", if self.server.is_some() { self.server_label() } else { "demo data — the query is not executed; pick a server in Servers".to_string() }))
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
            View::Dashboard => self.dashboard(cx),
            View::Ehrs => self.ehr_browser(cx),
            View::Composition => self.compositions_view(cx),
            View::Templates => self.template_browser(cx),
            View::Aql => self.aql_runner(window, cx),
            View::Servers => self.servers_view(window, cx),
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
        Some("servers") => View::Servers,
        _ => View::Dashboard,
    };
    let ehrs: usize = arg("--ehrs").and_then(|s| s.parse().ok()).unwrap_or(3);
    let rows: usize = arg("--rows").and_then(|s| s.parse().ok()).unwrap_or(3);
    let (select, run, bench_scroll) = (flag("--select"), flag("--run"), flag("--bench-scroll"));
    // Explicit --server wins; otherwise use the stored default/first profile (like the Tauri app); --demo opts out.
    let (mut server, mut server_name, mut active_profile, mut startup_error) = (server_from_env(), None, None, None);
    if server.is_none() && !flag("--demo") {
        let stored = profiles::load_stored();
        if let Some(sp) = profiles::initial_active(&stored) {
            match profiles::resolve(sp) {
                Ok(p) => {
                    server = Some(net::Profile::from_server(&p));
                    server_name = Some(p.name);
                    active_profile = Some(p.id);
                }
                Err(e) => startup_error = Some(e),
            }
        }
    }
    let query = arg("--query");
    // Opt-in only; never during benchmarks.
    let analytics = Analytics::init(flag("--analytics") && !bench_scroll, arg("--analytics-host"));
    let max_seconds: Option<u64> = arg("--max-seconds").and_then(|s| s.parse().ok()).or(bench_scroll.then_some(120));
    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds { origin: point(px(0.), px(0.)), size: size(px(1440.), px(960.)) };
        let opts = Opts { boot, view, ehr_count: ehrs, rows, select, run, bench_scroll, server, server_name, active_profile, startup_error, query, analytics: analytics.clone() };
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
