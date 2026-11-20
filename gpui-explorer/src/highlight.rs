//! Syntax highlighting for response bodies: JSON, XML and HTML. Pure (no GPUI) and unit-tested.
//! Runs are byte ranges into each line, so the UI can hand them straight to `StyledText`.

use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Key,
    String,
    Number,
    Keyword,
    Punct,
    Tag,
    Attr,
    AttrValue,
    Comment,
    Doctype,
    Entity,
    Plain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Json,
    Xml,
    Html,
    Plain,
}

impl Lang {
    pub fn label(self) -> &'static str {
        match self {
            Lang::Json => "JSON",
            Lang::Xml => "XML",
            Lang::Html => "HTML",
            Lang::Plain => "Text",
        }
    }
}

pub type Runs = Vec<(Range<usize>, Kind)>;

/// Language from the Content-Type when it says so, else sniffed from the body.
pub fn detect(content_type: Option<&str>, body: &str) -> Lang {
    let ct = content_type.unwrap_or("").to_ascii_lowercase();
    if ct.contains("json") {
        return Lang::Json;
    }
    if ct.contains("html") {
        return Lang::Html;
    }
    if ct.contains("xml") {
        return Lang::Xml;
    }
    let head = body.trim_start();
    let lower: String = head.chars().take(64).collect::<String>().to_ascii_lowercase();
    if lower.starts_with("<!doctype html") || lower.starts_with("<html") {
        Lang::Html
    } else if lower.starts_with("<?xml") || lower.starts_with('<') {
        Lang::Xml
    } else if head.starts_with('{') || head.starts_with('[') {
        Lang::Json
    } else {
        Lang::Plain
    }
}

// ---- JSON ------------------------------------------------------------------------------------------

/// Tokens of one JSON line (stateless: pretty-printed JSON never spans a string over lines).
pub fn json_line(line: &str) -> Runs {
    let b: Vec<(usize, char)> = line.char_indices().collect();
    let end_of = |i: usize| b.get(i).map(|(p, _)| *p).unwrap_or(line.len());
    let mut out: Runs = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i].1;
        let start = i;
        let kind = match c {
            '"' => {
                i += 1;
                while i < b.len() && b[i].1 != '"' {
                    i += if b[i].1 == '\\' { 2 } else { 1 };
                }
                i = (i + 1).min(b.len());
                let colon = b[i..].iter().find(|(_, x)| !x.is_whitespace()).is_some_and(|(_, x)| *x == ':');
                if colon {
                    Kind::Key
                } else {
                    Kind::String
                }
            }
            '{' | '}' | '[' | ']' | ',' | ':' => {
                i += 1;
                Kind::Punct
            }
            c if c == '-' || c.is_ascii_digit() => {
                while i < b.len() && (b[i].1.is_ascii_digit() || matches!(b[i].1, '-' | '+' | '.' | 'e' | 'E')) {
                    i += 1;
                }
                Kind::Number
            }
            c if c.is_ascii_alphabetic() => {
                while i < b.len() && b[i].1.is_ascii_alphabetic() {
                    i += 1;
                }
                if matches!(&line[end_of(start)..end_of(i)], "true" | "false" | "null") {
                    Kind::Keyword
                } else {
                    Kind::Plain
                }
            }
            _ => {
                i += 1;
                Kind::Plain
            }
        };
        push(&mut out, end_of(start)..end_of(i), kind);
    }
    out
}

fn push(out: &mut Runs, r: Range<usize>, kind: Kind) {
    if r.is_empty() {
        return;
    }
    match out.last_mut() {
        Some((prev, k)) if *k == kind && matches!(kind, Kind::Plain | Kind::AttrValue) && prev.end == r.start => prev.end = r.end,
        _ => out.push((r, kind)),
    }
}

// ---- XML / HTML ------------------------------------------------------------------------------------

/// Carried between lines: a comment or a tag can span lines.
#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub struct MarkupState {
    in_comment: bool,
    in_tag: bool,
    quote: Option<char>,
}

fn find_from(s: &str, from: usize, pat: &str) -> Option<usize> {
    s[from..].find(pat).map(|i| from + i)
}

/// Text outside tags: plain, with `&entity;` references picked out.
fn text_runs(line: &str, from: usize, to: usize, out: &mut Runs) {
    let mut pos = from;
    while pos < to {
        match line[pos..to].find('&') {
            None => {
                push(out, pos..to, Kind::Plain);
                return;
            }
            Some(i) => {
                let amp = pos + i;
                push(out, pos..amp, Kind::Plain);
                match line[amp..to].find(';').filter(|j| *j <= 10 && !line[amp + 1..amp + *j].contains(char::is_whitespace)) {
                    Some(j) => {
                        push(out, amp..amp + j + 1, Kind::Entity);
                        pos = amp + j + 1;
                    }
                    None => {
                        push(out, amp..amp + 1, Kind::Plain);
                        pos = amp + 1;
                    }
                }
            }
        }
    }
}

/// Scan attributes/closers inside a tag from `pos`; returns the new position (== line.len() if the tag continues on the next line).
fn tag_body(line: &str, mut pos: usize, st: &mut MarkupState, out: &mut Runs) -> usize {
    while pos < line.len() {
        if let Some(q) = st.quote {
            let end = line[pos..].find(q).map(|i| pos + i + 1);
            push(out, pos..end.unwrap_or(line.len()), Kind::AttrValue);
            match end {
                Some(e) => {
                    st.quote = None;
                    pos = e;
                    continue;
                }
                None => return line.len(),
            }
        }
        let c = line[pos..].chars().next().unwrap_or(' ');
        let clen = c.len_utf8();
        if c == '>' || (c == '/' && line[pos..].starts_with("/>")) || (c == '?' && line[pos..].starts_with("?>")) {
            let n = if c == '>' { 1 } else { 2 };
            push(out, pos..pos + n, Kind::Tag);
            st.in_tag = false;
            return pos + n;
        }
        if c == '"' || c == '\'' {
            st.quote = Some(c);
            push(out, pos..pos + 1, Kind::AttrValue);
            pos += 1;
            continue;
        }
        if c == '=' {
            push(out, pos..pos + 1, Kind::Punct);
            pos += 1;
            continue;
        }
        if c.is_whitespace() {
            push(out, pos..pos + clen, Kind::Plain);
            pos += clen;
            continue;
        }
        let end = line[pos..].find(|x: char| x.is_whitespace() || matches!(x, '=' | '>' | '/' | '"' | '\'')).map(|i| pos + i).unwrap_or(line.len()).max(pos + clen);
        push(out, pos..end, Kind::Attr);
        pos = end;
    }
    pos
}

/// Tokens of one markup line, updating the cross-line state.
pub fn markup_line(line: &str, st: &mut MarkupState) -> Runs {
    let mut out: Runs = Vec::new();
    let mut pos = 0;
    while pos < line.len() {
        if st.in_comment {
            match find_from(line, pos, "-->") {
                Some(e) => {
                    push(&mut out, pos..e + 3, Kind::Comment);
                    st.in_comment = false;
                    pos = e + 3;
                }
                None => {
                    push(&mut out, pos..line.len(), Kind::Comment);
                    return out;
                }
            }
            continue;
        }
        if st.in_tag {
            pos = tag_body(line, pos, st, &mut out);
            continue;
        }
        let Some(lt) = find_from(line, pos, "<") else {
            text_runs(line, pos, line.len(), &mut out);
            return out;
        };
        text_runs(line, pos, lt, &mut out);
        let rest = &line[lt..];
        if rest.starts_with("<!--") {
            st.in_comment = true;
            push(&mut out, lt..lt + 4, Kind::Comment);
            pos = lt + 4;
        } else if rest.starts_with("<![CDATA[") || rest.starts_with("<!") || rest.starts_with("<?") {
            let (close, kind) = if rest.starts_with("<?") { ("?>", Kind::Doctype) } else if rest.starts_with("<![CDATA[") { ("]]>", Kind::String) } else { (">", Kind::Doctype) };
            let end = find_from(line, lt + 2, close).map(|e| e + close.len()).unwrap_or(line.len());
            push(&mut out, lt..end, kind);
            pos = end;
        } else {
            let name_start = lt + if rest.starts_with("</") { 2 } else { 1 };
            let name_end = line[name_start..].find(|c: char| c.is_whitespace() || matches!(c, '>' | '/')).map(|i| name_start + i).unwrap_or(line.len());
            if name_end == name_start {
                // A lone '<' in text.
                push(&mut out, lt..lt + 1, Kind::Plain);
                pos = lt + 1;
            } else {
                push(&mut out, lt..name_end, Kind::Tag);
                st.in_tag = true;
                pos = name_end;
            }
        }
    }
    out
}

/// Re-indent markup that arrives minified (few lines): one element per line, text-only elements kept inline.
pub fn format_markup(body: &str) -> Vec<String> {
    if body.lines().count() > 3 {
        return body.lines().map(str::to_string).collect();
    }
    // Split into tags and text.
    let mut parts: Vec<(bool, String)> = Vec::new();
    let mut cur = String::new();
    let mut in_tag = false;
    for c in body.trim().chars() {
        if c == '<' && !in_tag {
            if !cur.trim().is_empty() {
                parts.push((false, cur.trim().to_string()));
            }
            cur.clear();
            in_tag = true;
        }
        cur.push(c);
        if c == '>' && in_tag {
            // A '>' inside a quoted attribute value is rare; comments/CDATA end with "-->"/"]]>" which also close here.
            parts.push((true, std::mem::take(&mut cur)));
            in_tag = false;
        }
    }
    if !cur.trim().is_empty() {
        parts.push((in_tag, cur.trim().to_string()));
    }
    let is_open = |t: &str| !t.starts_with("</") && !t.starts_with("<?") && !t.starts_with("<!") && !t.ends_with("/>");
    let mut lines: Vec<String> = Vec::new();
    let mut depth = 0usize;
    let mut i = 0;
    while i < parts.len() {
        let (tag, text) = &parts[i];
        let pad = "  ".repeat(depth);
        if *tag && is_open(text) {
            // <a>text</a> or <a></a> stays on one line.
            let inline_close = match (parts.get(i + 1), parts.get(i + 2)) {
                (Some((false, t)), Some((true, c))) if c.starts_with("</") => Some(format!("{text}{t}{c}")),
                (Some((true, c)), _) if c.starts_with("</") => Some(format!("{text}{c}")),
                _ => None,
            };
            if let Some(l) = inline_close {
                lines.push(format!("{pad}{l}"));
                i += if matches!(parts.get(i + 1), Some((false, _))) { 3 } else { 2 };
                continue;
            }
            lines.push(format!("{pad}{text}"));
            depth += 1;
        } else if *tag && text.starts_with("</") {
            depth = depth.saturating_sub(1);
            lines.push(format!("{}{text}", "  ".repeat(depth)));
        } else {
            lines.push(format!("{pad}{text}"));
        }
        i += 1;
    }
    lines
}

/// Highlight a whole set of display lines.
pub fn highlight_lines(lang: Lang, lines: &[String]) -> Vec<Runs> {
    match lang {
        Lang::Json => lines.iter().map(|l| json_line(l)).collect(),
        Lang::Xml | Lang::Html => {
            let mut st = MarkupState::default();
            lines.iter().map(|l| markup_line(l, &mut st)).collect()
        }
        Lang::Plain => lines.iter().map(|l| if l.is_empty() { vec![] } else { vec![(0..l.len(), Kind::Plain)] }).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(line: &str, runs: &Runs) -> Vec<(String, Kind)> {
        runs.iter().filter(|(_, k)| *k != Kind::Plain).map(|(r, k)| (line[r.clone()].to_string(), *k)).collect()
    }

    #[test]
    fn json_keys_strings_numbers_keywords() {
        let l = r#"  "name": "a \"q\" b", "n": -1.5e3, "ok": true, "x": null"#;
        let k = kinds(l, &json_line(l));
        assert!(k.contains(&(r#""name""#.into(), Kind::Key)));
        assert!(k.contains(&(r#""a \"q\" b""#.into(), Kind::String)));
        assert!(k.contains(&("-1.5e3".into(), Kind::Number)));
        assert!(k.contains(&("true".into(), Kind::Keyword)));
        assert!(k.contains(&("null".into(), Kind::Keyword)));
    }

    #[test]
    fn json_runs_are_ordered_and_on_char_boundaries() {
        let l = "  \"é\u{00a0}x\": \"ü\",";
        let runs = json_line(l);
        let mut pos = 0;
        for (r, _) in &runs {
            assert!(l.is_char_boundary(r.start) && l.is_char_boundary(r.end));
            assert_eq!(r.start, pos);
            pos = r.end;
        }
        assert_eq!(pos, l.len());
    }

    #[test]
    fn json_string_followed_by_colon_across_spaces_is_key() {
        let l = r#""a"  : "b""#;
        let k = kinds(l, &json_line(l));
        assert_eq!(k[0], (r#""a""#.into(), Kind::Key));
        assert!(k.contains(&(r#""b""#.into(), Kind::String)));
    }

    #[test]
    fn xml_tags_attrs_text_entities() {
        let l = r#"<a href="x y" id='1'>fish &amp; chips</a>"#;
        let mut st = MarkupState::default();
        let k = kinds(l, &markup_line(l, &mut st));
        assert!(k.contains(&("<a".into(), Kind::Tag)));
        assert!(k.contains(&("href".into(), Kind::Attr)));
        assert!(k.contains(&("\"x y\"".into(), Kind::AttrValue)));
        assert!(k.contains(&("'1'".into(), Kind::AttrValue)));
        assert!(k.contains(&(">".into(), Kind::Tag)));
        assert!(k.contains(&("&amp;".into(), Kind::Entity)));
        assert!(k.contains(&("</a".into(), Kind::Tag)));
        assert_eq!(st, MarkupState::default());
    }

    #[test]
    fn markup_comment_and_tag_span_lines() {
        let mut st = MarkupState::default();
        let l1 = "<!-- start";
        let l2 = "end --><p";
        let l3 = "  class=\"a";
        let l4 = "b\">hi</p>";
        assert_eq!(kinds(l1, &markup_line(l1, &mut st)), vec![("<!--".into(), Kind::Comment), (" start".into(), Kind::Comment)]);
        let k2 = kinds(l2, &markup_line(l2, &mut st));
        assert_eq!(k2[0], ("end -->".into(), Kind::Comment));
        assert!(st.in_tag);
        markup_line(l3, &mut st);
        assert!(st.quote.is_some(), "unterminated attribute value carries over");
        let k4 = kinds(l4, &markup_line(l4, &mut st));
        assert_eq!(k4[0], ("b\"".into(), Kind::AttrValue));
        assert!(!st.in_tag);
    }

    #[test]
    fn doctype_pi_and_self_closing() {
        let mut st = MarkupState::default();
        let l = "<?xml version=\"1.0\"?><!DOCTYPE html><br/>";
        let k = kinds(l, &markup_line(l, &mut st));
        assert!(k.contains(&("<?xml version=\"1.0\"?>".into(), Kind::Doctype)));
        assert!(k.contains(&("<!DOCTYPE html>".into(), Kind::Doctype)));
        assert!(k.contains(&("<br".into(), Kind::Tag)));
        assert!(k.contains(&("/>".into(), Kind::Tag)));
    }

    #[test]
    fn lone_lt_and_runs_cover_line() {
        let mut st = MarkupState::default();
        let l = "a < b & c <b>x</b> é";
        let runs = markup_line(l, &mut st);
        let mut pos = 0;
        for (r, _) in &runs {
            assert!(l.is_char_boundary(r.start) && l.is_char_boundary(r.end));
            assert_eq!(r.start, pos);
            pos = r.end;
        }
        assert_eq!(pos, l.len());
    }

    #[test]
    fn detection() {
        assert_eq!(detect(Some("application/json; charset=utf-8"), ""), Lang::Json);
        assert_eq!(detect(Some("application/openehr.wt+json"), "x"), Lang::Json);
        assert_eq!(detect(Some("text/html"), ""), Lang::Html);
        assert_eq!(detect(Some("application/xml"), ""), Lang::Xml);
        assert_eq!(detect(None, "  <!DOCTYPE HTML><html>"), Lang::Html);
        assert_eq!(detect(None, "<?xml version='1.0'?><a/>"), Lang::Xml);
        assert_eq!(detect(None, "[1,2]"), Lang::Json);
        assert_eq!(detect(None, "hello"), Lang::Plain);
    }

    #[test]
    fn minified_xml_is_reindented_with_inline_text() {
        let f = format_markup("<?xml version=\"1.0\"?><status><version>2.7</version><list><item/><item>x</item></list><empty></empty></status>");
        assert_eq!(f, vec!["<?xml version=\"1.0\"?>", "<status>", "  <version>2.7</version>", "  <list>", "    <item/>", "    <item>x</item>", "  </list>", "  <empty></empty>", "</status>"]);
    }

    #[test]
    fn already_formatted_markup_is_untouched() {
        let body = "<a>\n  <b/>\n  <c/>\n  <d/>\n</a>";
        assert_eq!(format_markup(body), body.lines().map(str::to_string).collect::<Vec<_>>());
    }
}
