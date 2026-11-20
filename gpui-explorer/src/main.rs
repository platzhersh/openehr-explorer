//! A for-fun GPUI port of openEHR Explorer's main screens.
//!
//! Data is in-process and mirrors `scripts/demo-assets/mock.js` so that screenshots
//! and benchmarks can be compared 1:1 with the Vue frontend.
//!
//! Flags:  --view dashboard|ehrs|templates|aql   --ehrs N   --rows N
//!         --select              (open first EHR / first template detail)
//!         --run                 (execute the AQL query immediately)
//!         --bench-scroll        (auto-scroll the AQL table, print frame stats, exit)

use gpui::{
    div, point, px, rgb, size, uniform_list, App, Application, Bounds, AppContext, Context, Div, FontWeight, InteractiveElement,
    IntoElement, ParentElement, Render, ScrollStrategy, SharedString, StatefulInteractiveElement, Styled,
    TitlebarOptions, UniformListScrollHandle, Window, WindowBounds, WindowOptions,
};
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

struct Composition {
    name: &'static str,
    template: &'static str,
    composer: &'static str,
    committed: &'static str,
}

struct Template {
    id: &'static str,
    concept: &'static str,
    archetype: &'static str,
}

const TEMPLATES: [Template; 4] = [
    Template { id: "vital_signs.v1", concept: "Vital Signs", archetype: "openEHR-EHR-COMPOSITION.encounter.v1" },
    Template { id: "IDCR - Problem List.v1", concept: "Problem List", archetype: "openEHR-EHR-COMPOSITION.problem_list.v1" },
    Template { id: "IDCR - Medication List.v1", concept: "Medication List", archetype: "openEHR-EHR-COMPOSITION.medication_list.v1" },
    Template { id: "IDCR - Discharge Summary.v1", concept: "Discharge Summary", archetype: "openEHR-EHR-COMPOSITION.discharge_summary.v1" },
];

const COMPOSITIONS: [Composition; 3] = [
    Composition { name: "Vital Signs", template: "vital_signs.v1", composer: "Dr. A. Kessler", committed: "2026-08-20T09:15:00Z" },
    Composition { name: "Problem List", template: "IDCR - Problem List.v1", composer: "Dr. A. Kessler", committed: "2026-08-18T13:42:10Z" },
    Composition { name: "Medication List", template: "IDCR - Medication List.v1", composer: "Dr. R. Okafor", committed: "2026-08-12T10:03:44Z" },
];

// (depth, label, rm_type, aql_path)
const WEB_TEMPLATE: [(u8, &str, &str, &str); 11] = [
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

struct AqlTable {
    columns: Vec<&'static str>,
    rows: Vec<[SharedString; 4]>,
    exec_ms: f64,
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
    table: Option<AqlTable>,
    row_target: usize,
    ehr_scroll: UniformListScrollHandle,
    aql_scroll: UniformListScrollHandle,
    tree_scroll: UniformListScrollHandle,
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

impl Explorer {
    fn new(boot: Instant, view: View, ehr_count: usize, rows: usize, select: bool, run: bool, bench_scroll: bool) -> Self {
        let mut seed = 42u64;
        let mut ehrs = vec![
            Ehr { id: "9c2c5c5e-1c1b-4b0a-9d1e-9a4b2f3c8a11".into(), created: "2026-06-02T08:14:21Z".into(), subject: "P-10234".into(), compositions: 3 },
            Ehr { id: "4a9e2d7b-6f31-4c9e-8a5a-1d2e3f4a5b6c".into(), created: "2026-08-19T10:05:17Z".into(), subject: "—".into(), compositions: 0 },
            Ehr { id: "b28f6a9c-2e4d-4a1b-9c3f-7e8d9c0b1a2f".into(), created: "2026-08-11T16:38:44Z".into(), subject: "—".into(), compositions: 0 },
        ];
        while ehrs.len() < ehr_count {
            let n = lcg(&mut seed);
            ehrs.push(Ehr {
                id: fake_uuid(&mut seed).into(),
                created: format!("2026-{:02}-{:02}T{:02}:{:02}:00Z", 1 + n % 11, 1 + (n >> 4) % 28, (n >> 9) % 24, (n >> 14) % 60).into(),
                subject: format!("P-{}", 10000 + n % 90000).into(),
                compositions: (n % 7) as u32,
            });
        }
        let mut this = Self {
            view,
            ehrs,
            selected_ehr: select.then_some(0),
            selected_template: 0,
            table: None,
            row_target: rows,
            ehr_scroll: UniformListScrollHandle::new(),
            aql_scroll: UniformListScrollHandle::new(),
            tree_scroll: UniformListScrollHandle::new(),
            boot,
            first_render_reported: false,
            run_clicked_at: None,
            bench: None,
            bench_scroll,
        };
        if run {
            this.run_query();
        }
        this
    }

    /// Called by the bench driver task: advance the scroll position from the number of frames
    /// rendered so far and ask for another frame.
    fn bench_tick(&mut self, cx: &mut Context<Self>) {
        let Some(b) = self.bench.as_ref() else { return };
        let n = self.table.as_ref().map(|t| t.rows.len()).unwrap_or(0);
        if n > 0 {
            let ix = (b.frame * (n / b.total_frames).max(1)).min(n - 1);
            self.aql_scroll.scroll_to_item(ix, ScrollStrategy::Top);
        }
        cx.notify();
    }

    fn run_query(&mut self) {
        let t = Instant::now();
        let mut seed = 7u64;
        let n = self.row_target;
        let mut rows = Vec::with_capacity(n);
        for i in 0..n {
            let id: SharedString = if i < 3 {
                ["9c2c5c5e-1c1b-4b0a-9d1e-9a4b2f3c8a11", "4a9e2d7b-6f31-4c9e-8a5a-1d2e3f4a5b6c", "b28f6a9c-2e4d-4a1b-9c3f-7e8d9c0b1a2f"][i].into()
            } else {
                fake_uuid(&mut seed).into()
            };
            let r = lcg(&mut seed);
            rows.push([
                id,
                format!("{}", 100 + r % 60).into(),
                format!("{}", 60 + (r >> 8) % 40).into(),
                format!("2026-{:02}-{:02}T{:02}:{:02}:00Z", 1 + r % 11, 1 + (r >> 4) % 28, (r >> 9) % 24, (r >> 14) % 60).into(),
            ]);
        }
        self.table = Some(AqlTable {
            columns: vec!["ehr_id", "systolic", "diastolic", "time_committed"],
            rows,
            exec_ms: t.elapsed().as_secs_f64() * 1000.0,
        });
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
            .child(div().p_4().text_size(px(12.)).text_color(rgb(MUTED)).child("v0.10.3 · gpui"))
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
        let comps: u32 = self.ehrs.iter().map(|e| e.compositions).sum();
        div()
            .flex()
            .flex_col()
            .p_6()
            .gap_4()
            .child(Self::page_title("Dashboard", "Local EHRBase · hospital.example.org".into()))
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(card("EHRs", self.ehrs.len().to_string()))
                    .child(card("Compositions", comps.to_string()))
                    .child(card("Templates", TEMPLATES.len().to_string())),
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
                    .child(div().size(px(8.)).rounded_full().bg(rgb(SUCCESS)))
                    .child(div().text_size(px(13.)).text_color(rgb(TEXT)).child("Connected — EHRBase 2.7.0, openEHR SDK 2.19.0")),
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
                                this.selected_ehr = Some(i);
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
            .child(div().p_4().child(Self::page_title("EHR Browser", format!("{count} EHRs"))))
            .child(list);

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
                for c in COMPOSITIONS.iter().take(e.compositions.min(3) as usize) {
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
                                    .child(div().text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(TEXT)).child(c.name))
                                    .child(div().text_size(px(12.)).text_color(rgb(MUTED)).child(c.template)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_end()
                                    .gap_1()
                                    .child(div().text_size(px(12.)).text_color(rgb(TEXT2)).child(c.composer))
                                    .child(div().text_size(px(12.)).text_color(rgb(MUTED)).child(c.committed)),
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
                            .child(kv("System ID", "hospital.example.org".into()))
                            .child(kv("Created", e.created.clone()))
                            .child(kv("Subject", e.subject.clone()))
                            .child(kv("Modifiable", "true".into()))
                            .child(kv("Queryable", "true".into())),
                    )
                    .child(div().text_size(px(15.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(TEXT)).child(format!("Compositions ({})", e.compositions)))
                    .child(comps)
            }
        };
        div().flex().size_full().child(list_pane).child(detail)
    }

    fn template_browser(&mut self, cx: &mut Context<Self>) -> Div {
        let mut list = div().flex().flex_col().gap_2().px_4();
        for (i, t) in TEMPLATES.iter().enumerate() {
            let sel = self.selected_template == i;
            list = list.child(
                div()
                    .id(("tpl", i))
                    .p_3()
                    .rounded(px(6.))
                    .cursor_pointer()
                    .border_1()
                    .border_color(rgb(if sel { PRIMARY } else { BORDER }))
                    .bg(rgb(if sel { BG3 } else { SURFACE }))
                    .hover(|s| s.bg(rgb(SURFACE_HOVER)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected_template = i;
                        cx.notify();
                    }))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(TEXT)).child(t.concept))
                    .child(div().text_size(px(12.)).font_family(MONO).text_color(rgb(MUTED)).child(t.id))
                    .child(div().text_size(px(11.)).font_family(MONO).text_color(rgb(MUTED)).child(t.archetype)),
            );
        }
        let t = &TEMPLATES[self.selected_template];
        let tree = uniform_list(
            "tpl-tree",
            WEB_TEMPLATE.len(),
            cx.processor(|_this, range: Range<usize>, _w, _cx| {
                range
                    .map(|i| {
                        let (depth, label, rm, path) = WEB_TEMPLATE[i];
                        div()
                            .w_full()
                            .h(px(32.))
                            .flex()
                            .items_center()
                            .gap_3()
                            .pl(px(12. + depth as f32 * 22.))
                            .border_b_1()
                            .border_color(rgb(BORDER))
                            .child(div().text_size(px(13.)).text_color(rgb(TEXT)).w(px(170.)).child(label))
                            .child(div().text_size(px(11.)).font_family(MONO).text_color(rgb(PRIMARY)).w(px(110.)).child(rm))
                            .child(div().text_size(px(11.)).font_family(MONO).text_color(rgb(MUTED)).child(path))
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
                    .child(div().p_4().child(Self::page_title("Templates", format!("{} templates", TEMPLATES.len()))))
                    .child(list),
            )
            .child(
                div()
                    .flex_1()
                    .p_6()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(div().text_size(px(18.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(PRIMARY)).child(format!("{} — Web Template", t.concept)))
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
            .children(QUERY.lines().map(|l| {
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
                this.run_query();
                cx.notify();
            }))
            .child("Run (Ctrl+Enter)");
        let status = match &self.table {
            Some(t) => format!("{} rows · built in {:.1} ms", t.rows.len(), t.exec_ms),
            None => "No results yet".to_string(),
        };
        let col_w = [px(330.), px(110.), px(110.), px(220.)];
        let results = match &self.table {
            None => div().flex_1().flex().items_center().justify_center().text_color(rgb(MUTED)).child("Run a query to see results"),
            Some(t) => {
                let header = div().flex().h(px(34.)).items_center().bg(rgb(BG3)).border_b_1().border_color(rgb(BORDER)).children(
                    t.columns.iter().enumerate().map(|(i, c)| div().w(col_w[i]).px_3().text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(PRIMARY)).child(*c)),
                );
                let n = t.rows.len();
                let body = uniform_list(
                    "aql-rows",
                    n,
                    cx.processor(move |this, range: Range<usize>, _w, _cx| {
                        let t = this.table.as_ref().unwrap();
                        range
                            .map(|i| {
                                let r = &t.rows[i];
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
                                    .children(r.iter().enumerate().map(|(c, v)| div().w(col_w[c]).px_3().child(v.clone())))
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
            .child(div().flex_1().h_full().min_w_0().child(content));

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

fn main() {
    let boot = Instant::now();
    let view = match arg("--view").as_deref() {
        Some("ehrs") => View::Ehrs,
        Some("templates") => View::Templates,
        Some("aql") => View::Aql,
        _ => View::Dashboard,
    };
    let ehrs: usize = arg("--ehrs").and_then(|s| s.parse().ok()).unwrap_or(3);
    let rows: usize = arg("--rows").and_then(|s| s.parse().ok()).unwrap_or(3);
    let (select, run, bench_scroll) = (flag("--select"), flag("--run"), flag("--bench-scroll"));
    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds { origin: point(px(0.), px(0.)), size: size(px(1440.), px(960.)) };
        let handle = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions { title: Some("openEHR Explorer".into()), ..Default::default() }),
                ..Default::default()
            },
            |_, cx| cx.new(|_| Explorer::new(boot, view, ehrs, rows, select, run, bench_scroll)),
        )
        .unwrap();
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
        // Safety net so scripted runs can never hang.
        cx.spawn(async move |cx| {
            cx.background_executor().timer(Duration::from_secs(120)).await;
            cx.update(|cx| cx.quit()).ok();
        })
        .detach();
    });
}
