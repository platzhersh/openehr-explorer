//! Request Inspector panel (a bottom drawer, like the Tauri app's): log list with filters on the left,
//! request/response detail on the right. Model and helpers are in `inspector.rs`.

use crate::highlight::{self, Kind, Lang, Runs};
use crate::inspector::{self, Filter, Log, LogEntry, State};
use crate::{btn, Explorer, Field, BG, BG2, BG3, BORDER, MONO, MUTED, PRIMARY, SUCCESS, SURFACE, TEXT, TEXT2};
use gpui::{
    div, px, rgb, uniform_list, App, ClipboardItem, Context, Div, ElementId, FontStyle, FontWeight, HighlightStyle, InteractiveElement, KeyDownEvent, ParentElement, SharedString, Stateful, StatefulInteractiveElement, Styled, StyledText,
    UniformListScrollHandle, Window,
};
use std::ops::Range;

const HEADER_H: f32 = 32.0;
const LOG_W: f32 = 400.0;
const META_W: f32 = 380.0;
const RED: u32 = 0xff7b7b;
const AMBER: u32 = 0xffcb6b;
const BLUE: u32 = 0x82aaff;

#[derive(Clone, Copy, PartialEq)]
pub enum Drawer {
    Collapsed,
    Half,
    Expanded,
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Request,
    Response,
}

struct BodyCache {
    key: (u64, Tab, bool, bool),
    lang: Lang,
    lines: Vec<SharedString>,
    runs: Vec<Runs>,
}

fn kind_style(k: Kind) -> HighlightStyle {
    let (color, italic) = match k {
        Kind::Key => (0x8fd3ff, false),
        Kind::String | Kind::AttrValue => (0xc3e88d, false),
        Kind::Number | Kind::Entity => (0xf78c6c, false),
        Kind::Keyword => (0xc792ea, false),
        Kind::Punct => (TEXT2, false),
        Kind::Tag => (0xff9cac, false),
        Kind::Attr => (AMBER, false),
        Kind::Comment | Kind::Doctype => (MUTED, true),
        Kind::Plain => (TEXT, false),
    };
    HighlightStyle { color: Some(rgb(color).into()), font_style: italic.then_some(FontStyle::Italic), ..Default::default() }
}

/// One line of text coloured by its highlight runs (byte ranges into `text`).
pub fn highlighted(text: &str, runs: &[(std::ops::Range<usize>, Kind)]) -> StyledText {
    let valid = runs.iter().filter(|(r, _)| r.end <= text.len() && text.is_char_boundary(r.start) && text.is_char_boundary(r.end)).map(|(r, k)| (r.clone(), kind_style(*k)));
    StyledText::new(text.to_string()).with_highlights(valid.collect::<Vec<_>>())
}

pub struct InspectorUi {
    log: Log,
    drawer: Drawer,
    tab: Tab,
    pretty: bool,
    filter: Filter,
    filter_field: Field,
    confirm_clear: bool,
    /// Indices into `log.entries` that pass the filter (rebuilt every frame).
    visible: Vec<usize>,
    list_scroll: UniformListScrollHandle,
    body_scroll: UniformListScrollHandle,
    body: Option<BodyCache>,
}

impl InspectorUi {
    pub fn new(cx: &mut App) -> Self {
        Self {
            log: Log::default(),
            drawer: Drawer::Collapsed,
            tab: Tab::Response,
            pretty: true,
            filter: Filter::default(),
            filter_field: Field::new(cx, "", false, "Filter by path…"),
            confirm_clear: false,
            visible: vec![],
            list_scroll: UniformListScrollHandle::new(),
            body_scroll: UniformListScrollHandle::new(),
            body: None,
        }
    }

    /// Selected entry's body for the current tab: display lines + highlight runs (cached until selection/tab/mode changes).
    fn body(&mut self) -> (Option<&BodyCache>, Option<(usize, bool)>) {
        let Some(e) = self.log.selected_entry() else { return (None, None) };
        let (raw, headers) = match self.tab {
            Tab::Request => (e.request_body.as_deref(), &e.request_headers),
            Tab::Response => (e.response_body.as_deref(), &e.response_headers),
        };
        let info = raw.map(|b| (b.len(), self.tab == Tab::Response && e.body_truncated));
        let key = (e.id, self.tab, self.pretty, e.state == State::Complete);
        if self.body.as_ref().map(|c| c.key) != Some(key) {
            let body = raw.unwrap_or("");
            let lang = highlight::detect(inspector::header_value(headers, "content-type"), body);
            let lines: Vec<String> = match lang {
                Lang::Xml | Lang::Html if self.pretty => highlight::format_markup(body),
                _ => inspector::body_lines(body, self.pretty && lang == Lang::Json),
            };
            let runs = highlight::highlight_lines(lang, &lines);
            self.body = Some(BodyCache { key, lang, lines: lines.into_iter().map(SharedString::from).collect(), runs });
        }
        (self.body.as_ref(), info)
    }
}

fn method_color(m: &str) -> u32 {
    match m {
        "GET" => SUCCESS,
        "POST" => BLUE,
        "PUT" => AMBER,
        "DELETE" => RED,
        _ => MUTED,
    }
}

fn status_color(e: &LogEntry) -> u32 {
    match e.state {
        State::Pending => MUTED,
        State::Failed => RED,
        State::Complete => match e.status {
            500.. => RED,
            400..=499 => AMBER,
            300..=399 => BLUE,
            200..=299 => SUCCESS,
            _ => MUTED,
        },
    }
}

fn section(title: &'static str) -> Div {
    div().pt_3().pb_1().text_size(px(11.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(MUTED)).child(title.to_uppercase())
}

fn kv(label: &'static str, value: impl Into<SharedString>, color: u32, mono: bool) -> Div {
    let v = div().flex_1().min_w_0().text_size(px(12.)).text_color(rgb(color)).child(value.into());
    div().flex().gap_2().py(px(1.)).child(div().w(px(92.)).flex_none().text_size(px(12.)).text_color(rgb(MUTED)).child(label)).child(if mono { v.font_family(MONO) } else { v })
}

impl Explorer {
    /// Install the log sink and forward entries into the UI as they arrive.
    pub(crate) fn start_inspector(&mut self, cx: &mut Context<Self>) {
        let Some(mut rx) = inspector::install() else { return };
        cx.spawn(async move |this, cx| {
            while let Some(entry) = rx.recv().await {
                if this
                    .update(cx, |this, cx| {
                        this.inspector.log.upsert(entry);
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    /// Ctrl/Cmd+Shift+I cycles collapsed → half → expanded → collapsed.
    pub(crate) fn inspector_cycle(&mut self) {
        self.inspector.drawer = match self.inspector.drawer {
            Drawer::Collapsed => Drawer::Half,
            Drawer::Half => Drawer::Expanded,
            Drawer::Expanded => Drawer::Collapsed,
        };
    }

    pub(crate) fn inspector_open(&mut self, to: Drawer) {
        self.inspector.drawer = to;
    }

    fn inspector_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let ui = &mut self.inspector;
        if !ui.filter_field.focus.is_focused(window) {
            return;
        }
        let paste = if (ev.keystroke.modifiers.control || ev.keystroke.modifiers.platform) && ev.keystroke.key == "v" { cx.read_from_clipboard().and_then(|c| c.text()) } else { None };
        ui.filter_field.handle_key(&ev.keystroke, paste);
        ui.filter.text = ui.filter_field.text.trim().to_string();
        cx.notify();
        cx.stop_propagation();
    }

    fn inspector_header(&self, cx: &mut Context<Self>) -> Div {
        let ui = &self.inspector;
        let count = ui.log.entries.len();
        let state_btn = |id: &'static str, label: &'static str, to: Drawer, cx: &mut Context<Self>| btn(id, label, ui.drawer == to, cx.listener(move |this, _, _, cx| {
            this.inspector_open(to);
            cx.notify();
        }));
        let clear = (count > 0).then(|| {
            if ui.confirm_clear {
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_size(px(12.)).text_color(rgb(TEXT2)).child("Clear all?"))
                    .child(btn("insp-clear-yes", "Yes", true, cx.listener(|this, _, _, cx| {
                        this.inspector.log.clear();
                        this.inspector.confirm_clear = false;
                        cx.notify();
                    })))
                    .child(btn("insp-clear-no", "No", false, cx.listener(|this, _, _, cx| {
                        this.inspector.confirm_clear = false;
                        cx.notify();
                    })))
            } else {
                div().child(btn("insp-clear", "Clear", false, cx.listener(|this, _, _, cx| {
                    this.inspector.confirm_clear = true;
                    cx.notify();
                })))
            }
        });
        let title = div()
            .id("insp-title")
            .flex()
            .items_center()
            .gap_2()
            .h_full()
            .flex_1()
            .cursor_pointer()
            .on_click(cx.listener(|this, _, _, cx| {
                this.inspector_cycle();
                cx.notify();
            }))
            .child(div().text_size(px(13.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(TEXT)).child("Request Inspector"))
            .children((count > 0).then(|| div().px_2().rounded(px(8.)).bg(rgb(BG3)).text_size(px(11.)).text_color(rgb(PRIMARY)).child(count.to_string())))
            .children((ui.log.has_errors() && ui.drawer == Drawer::Collapsed).then(|| div().size(px(8.)).rounded_full().bg(rgb(RED))))
            .child(div().text_size(px(11.)).text_color(rgb(MUTED)).child("Ctrl+Shift+I"));
        div()
            .h(px(HEADER_H))
            .flex_none()
            .flex()
            .items_center()
            .gap_3()
            .px_3()
            .bg(rgb(BG2))
            .border_t_1()
            .border_color(rgb(BORDER))
            .child(title)
            .children(clear)
            .child(div().flex().gap_1().child(state_btn("insp-collapse", "Collapse", Drawer::Collapsed, cx)).child(state_btn("insp-half", "Half", Drawer::Half, cx)).child(state_btn("insp-full", "Full", Drawer::Expanded, cx)))
    }

    fn filter_chip(id: impl Into<ElementId>, label: &'static str, on: bool, toggle: impl Fn(&mut Filter) + 'static, cx: &mut Context<Self>) -> Stateful<Div> {
        div()
            .id(id)
            .px_2()
            .h(px(22.))
            .flex()
            .items_center()
            .rounded(px(11.))
            .cursor_pointer()
            .text_size(px(11.))
            .border_1()
            .border_color(rgb(if on { PRIMARY } else { BORDER }))
            .text_color(rgb(if on { PRIMARY } else { TEXT2 }))
            .bg(rgb(if on { BG3 } else { SURFACE }))
            .on_click(cx.listener(move |this, _, _, cx| {
                toggle(&mut this.inspector.filter);
                cx.notify();
            }))
            .child(label)
    }

    fn log_pane(&mut self, window: &Window, cx: &mut Context<Self>) -> Div {
        let ui = &self.inspector;
        let toggle_in = |v: &mut Vec<String>, k: &str| match v.iter().position(|x| x == k) {
            Some(i) => {
                v.remove(i);
            }
            None => v.push(k.to_string()),
        };
        let mut chips = div().flex().flex_wrap().gap_1();
        for m in ["GET", "POST", "PUT", "DELETE"] {
            let on = ui.filter.methods.iter().any(|x| x == m);
            chips = chips.child(Self::filter_chip(SharedString::from(format!("insp-m-{m}")), m, on, move |f| toggle_in(&mut f.methods, m), cx));
        }
        for c in ["2xx", "3xx", "4xx", "5xx", "failed"] {
            let on = ui.filter.classes.iter().any(|x| x == c);
            chips = chips.child(Self::filter_chip(SharedString::from(format!("insp-c-{c}")), c, on, move |f| toggle_in(&mut f.classes, c), cx));
        }
        let input = Self::input_box(&ui.filter_field, ElementId::from("insp-filter"), window);
        let n = ui.visible.len();
        let list = if n == 0 {
            div().flex_1().flex().items_center().justify_center().text_size(px(12.)).text_color(rgb(MUTED)).child(if ui.log.entries.is_empty() { "No requests yet" } else { "No requests match the filter" })
        } else {
            div().flex_1().min_h_0().child(
                uniform_list(
                    "insp-log",
                    n,
                    cx.processor(|this, range: Range<usize>, _w, cx| {
                        range
                            .filter_map(|i| {
                                let idx = *this.inspector.visible.get(i)?;
                                let e = this.inspector.log.entries.get(idx)?.clone();
                                let selected = this.inspector.log.selected == Some(e.id);
                                let id = e.id;
                                Some(
                                    div()
                                        .id(ElementId::from(("insp-row", id as usize)))
                                        .w_full()
                                        .h(px(28.))
                                        .px_2()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .cursor_pointer()
                                        .border_b_1()
                                        .border_color(rgb(BORDER))
                                        .bg(rgb(if selected { BG3 } else { BG }))
                                        .hover(|s| s.bg(rgb(crate::SURFACE_HOVER)))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.inspector.log.selected = Some(id);
                                            cx.notify();
                                        }))
                                        .child(div().w(px(52.)).flex_none().text_size(px(11.)).font_weight(FontWeight::BOLD).text_color(rgb(method_color(&e.method))).child(e.method.clone()))
                                        .child(div().flex_1().min_w_0().whitespace_nowrap().overflow_hidden().text_ellipsis().font_family(MONO).text_size(px(11.)).text_color(rgb(TEXT)).child(inspector::extract_path(&e.url)))
                                        .child(div().flex_none().text_size(px(11.)).text_color(rgb(status_color(&e))).child(inspector::status_label(&e)))
                                        .child(div().w(px(52.)).flex_none().text_size(px(11.)).text_color(rgb(MUTED)).child(if e.state == State::Pending { "…".to_string() } else { format!("{} ms", e.duration_ms) })),
                                )
                            })
                            .collect()
                    }),
                )
                .track_scroll(ui.list_scroll.clone())
                .size_full(),
            )
        };
        div()
            .w(px(LOG_W))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(rgb(BORDER))
            .child(div().p_2().flex().flex_col().gap_2().child(input).child(chips))
            .child(list)
    }

    fn headers_block(headers: &[(String, String)]) -> Div {
        let rows = headers.iter().map(|(k, v)| div().flex().gap_2().py(px(1.)).child(div().w(px(120.)).flex_none().overflow_hidden().text_size(px(11.)).font_family(MONO).text_color(rgb(PRIMARY)).child(k.clone())).child(div().flex_1().min_w_0().text_size(px(11.)).font_family(MONO).text_color(rgb(TEXT)).child(v.clone())));
        div().flex().flex_col().children(rows)
    }

    fn meta_pane(&self, e: &LogEntry, cx: &mut Context<Self>) -> Stateful<Div> {
        let tab = self.inspector.tab;
        let mut col = div().id("insp-meta").w(px(META_W)).flex_none().h_full().overflow_y_scroll().px_3().pb_3().flex().flex_col().border_r_1().border_color(rgb(BORDER));
        col = col.child(section("Summary"));
        match tab {
            Tab::Request => {
                col = col.child(kv("Method", e.method.clone(), method_color(&e.method), false)).child(kv("URL", e.url.clone(), TEXT, true)).child(kv("Duration", if e.state == State::Pending { "…".to_string() } else { format!("{} ms", e.duration_ms) }, TEXT, false)).child(kv("Time", inspector::iso8601(e.timestamp_ms), TEXT, true));
                col = col.child(section("Request headers")).child(Self::headers_block(&e.request_headers));
                let curl = inspector::generate_curl(e);
                let shown: SharedString = curl.replace(" \\\n  ", " ").into();
                col = col.child(section("cURL")).child(div().text_size(px(11.)).font_family(MONO).text_color(rgb(TEXT2)).child(shown)).child(div().pt_2().flex().child(btn("insp-copy-curl", "Copy cURL", false, cx.listener(move |_this, _, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(curl.clone()));
                }))));
            }
            Tab::Response => {
                col = col.child(kv("Status", inspector::status_label(e), status_color(e), false));
                if let Some(err) = &e.error {
                    col = col.child(kv("Error", err.clone(), RED, true));
                }
                col = col.child(kv("Content-Type", inspector::header_value(&e.response_headers, "content-type").unwrap_or("—").to_string(), TEXT, true)).child(kv("Size", e.response_body.as_ref().map(|b| inspector::format_size(b.len())).unwrap_or_else(|| "—".into()), TEXT, false));
                col = col.child(section("Response headers")).child(Self::headers_block(&e.response_headers));
            }
        }
        col
    }

    fn body_pane(&mut self, cx: &mut Context<Self>) -> Div {
        let pretty = self.inspector.pretty;
        let (n, info, lang) = {
            let (cache, info) = self.inspector.body();
            (cache.map(|c| c.lines.len()).unwrap_or(0), info, cache.map(|c| c.lang))
        };
        let title = match self.inspector.tab {
            Tab::Request => "Request body",
            Tab::Response => "Response body",
        };
        let meta = match info {
            Some((bytes, truncated)) => format!("{} · {}{}", lang.map(Lang::label).unwrap_or("Text"), inspector::format_size(bytes), if truncated { " · truncated to 2 MB in the log" } else { "" }),
            None => "no body".to_string(),
        };
        let toolbar = div()
            .h(px(34.))
            .flex_none()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .child(div().text_size(px(11.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(MUTED)).child(title.to_uppercase()))
            .child(div().flex_1().text_size(px(11.)).text_color(rgb(MUTED)).child(meta))
            .child(btn("insp-pretty", "Pretty", pretty, cx.listener(|this, _, _, cx| {
                this.inspector.pretty = true;
                cx.notify();
            })))
            .child(btn("insp-raw", "Raw", !pretty, cx.listener(|this, _, _, cx| {
                this.inspector.pretty = false;
                cx.notify();
            })))
            .child(btn("insp-copy-body", "Copy", false, cx.listener(|this, _, _, cx| {
                let text = this.inspector.body().0.map(|c| c.lines.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n")).unwrap_or_default();
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            })));
        let body = if n == 0 {
            div().flex_1().flex().items_center().justify_center().text_size(px(12.)).text_color(rgb(MUTED)).child("No body")
        } else {
            div().flex_1().min_h_0().child(
                uniform_list(
                    "insp-body",
                    n,
                    cx.processor(|this, range: Range<usize>, _w, _cx| {
                        let Some(cache) = this.inspector.body().0 else { return vec![] };
                        range.filter_map(|i| Some((cache.lines.get(i)?, cache.runs.get(i)?))).map(|(l, r)| div().h(px(18.)).px_3().whitespace_nowrap().overflow_hidden().font_family(MONO).text_size(px(12.)).text_color(rgb(TEXT)).child(highlighted(l, r))).collect()
                    }),
                )
                .track_scroll(self.inspector.body_scroll.clone())
                .size_full(),
            )
        };
        div().flex_1().min_w_0().h_full().flex().flex_col().child(toolbar).child(body)
    }

    fn detail_pane(&mut self, cx: &mut Context<Self>) -> Div {
        let Some(e) = self.inspector.log.selected_entry().cloned() else {
            return div().flex_1().h_full().flex().items_center().justify_center().text_size(px(12.)).text_color(rgb(MUTED)).child("Select a request");
        };
        let tab = self.inspector.tab;
        let tabs = div()
            .h(px(34.))
            .flex_none()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .border_b_1()
            .border_color(rgb(BORDER))
            .child(btn("insp-tab-req", "Request", tab == Tab::Request, cx.listener(|this, _, _, cx| {
                this.inspector.tab = Tab::Request;
                cx.notify();
            })))
            .child(btn("insp-tab-resp", "Response", tab == Tab::Response, cx.listener(|this, _, _, cx| {
                this.inspector.tab = Tab::Response;
                cx.notify();
            })))
            .child(div().flex_1().min_w_0().whitespace_nowrap().overflow_hidden().text_ellipsis().font_family(MONO).text_size(px(11.)).text_color(rgb(MUTED)).child(format!("{} {}", e.method, inspector::extract_path(&e.url))));
        let panes = div().flex_1().min_h_0().flex().child(self.meta_pane(&e, cx)).child(self.body_pane(cx));
        div().flex_1().min_w_0().h_full().flex().flex_col().child(tabs).child(panes)
    }

    /// The whole drawer (header always; content when not collapsed).
    pub(crate) fn inspector_drawer(&mut self, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.inspector.visible = self.inspector.log.filtered(&self.inspector.filter);
        let total = f32::from(window.viewport_size().height);
        let height = match self.inspector.drawer {
            Drawer::Collapsed => HEADER_H,
            Drawer::Half => (total * 0.4).max(HEADER_H + 160.0),
            Drawer::Expanded => total - HEADER_H,
        };
        let header = self.inspector_header(cx);
        let mut drawer = div().id("insp-drawer").w_full().flex_none().h(px(height)).flex().flex_col().bg(rgb(BG)).on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| this.inspector_key(ev, window, cx))).child(header);
        if self.inspector.drawer != Drawer::Collapsed {
            let content = div().flex_1().min_h_0().flex().child(self.log_pane(window, cx)).child(self.detail_pane(cx));
            drawer = drawer.child(content);
        }
        drawer
    }
}
