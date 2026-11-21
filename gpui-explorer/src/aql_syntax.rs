//! AQL tokenizer for syntax highlighting, and a small formatter. Pure and unit-tested.
//! Tokens are per line (char ranges); strings and `--` comments never span lines.

use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tok {
    Keyword,
    Function,
    /// RM class after FROM/CONTAINS (EHR, COMPOSITION, OBSERVATION, …).
    Type,
    /// `[openEHR-EHR-OBSERVATION.blood_pressure.v2]`, `[at0001]`, `[name/value=...]`.
    Archetype,
    /// `c/uid/value`, `o/data[at0001]/...` (the path part after an alias).
    Path,
    Alias,
    String,
    Number,
    Param,
    Comment,
    Operator,
    Punct,
    Plain,
}

const KEYWORDS: &[&str] = &[
    "SELECT", "FROM", "CONTAINS", "WHERE", "ORDER", "BY", "GROUP", "HAVING", "LIMIT", "OFFSET", "AS", "DISTINCT", "TOP", "AND", "OR", "NOT", "LIKE", "MATCHES", "EXISTS", "IN", "ASC", "DESC", "ASCENDING", "DESCENDING", "TRUE", "FALSE", "NULL", "UNION", "BETWEEN", "IS", "VERSION", "LATEST_VERSION", "ALL_VERSIONS",
];
const FUNCTIONS: &[&str] = &["COUNT", "SUM", "MIN", "MAX", "AVG", "LENGTH", "CONTAINS", "POSITION", "SUBSTRING", "CONCAT", "CONCAT_WS", "ABS", "MOD", "CEIL", "FLOOR", "ROUND", "LOWER", "UPPER", "TRIM", "NOW", "CURRENT_DATE", "CURRENT_TIME", "CURRENT_DATE_TIME", "CURRENT_DATETIME", "TO_DATE", "TO_TIMESTAMP", "TO_STRING", "COALESCE", "REPLACE", "SQRT", "EXP", "LOG"];
const TYPES: &[&str] = &[
    "EHR", "EHR_STATUS", "EHR_ACCESS", "COMPOSITION", "FOLDER", "CONTRIBUTION", "VERSIONED_OBJECT", "VERSIONED_COMPOSITION", "VERSIONED_EHR_STATUS", "VERSIONED_FOLDER", "VERSIONED_EHR_ACCESS", "SECTION", "ADMIN_ENTRY", "OBSERVATION", "EVALUATION", "INSTRUCTION", "ACTION", "ACTIVITY", "GENERIC_ENTRY", "CLUSTER", "ELEMENT", "HISTORY", "EVENT", "POINT_EVENT", "INTERVAL_EVENT", "ITEM_TREE", "ITEM_LIST", "ITEM_SINGLE", "ITEM_TABLE", "PARTY_IDENTIFIED", "PARTICIPATION", "DV_TEXT", "DV_CODED_TEXT", "DV_QUANTITY", "DV_COUNT", "DV_DATE_TIME", "DV_DATE", "DV_TIME", "DV_BOOLEAN", "DV_IDENTIFIER", "DV_ORDINAL", "DV_DURATION", "DV_PROPORTION",
];

pub const COMPLETIONS: &[&str] = &["SELECT", "DISTINCT", "TOP", "FROM", "EHR", "CONTAINS", "COMPOSITION", "OBSERVATION", "EVALUATION", "INSTRUCTION", "ACTION", "CLUSTER", "ELEMENT", "WHERE", "AND", "OR", "NOT", "LIKE", "MATCHES", "EXISTS", "ORDER BY", "ASC", "DESC", "LIMIT", "OFFSET", "AS", "COUNT", "MIN", "MAX", "AVG", "SUM"];

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}
fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn is_member(list: &[&str], s: &str) -> bool {
    list.iter().any(|k| k.eq_ignore_ascii_case(s))
}

fn scan_while(c: &[char], mut i: usize, f: impl Fn(char) -> bool) -> usize {
    while i < c.len() && f(c[i]) {
        i += 1;
    }
    i
}

/// Quoted string starting at `i` (a backslash escapes the next char); an unterminated one still colours to end of line,
/// so typing a quote is visible at once.
fn scan_string(c: &[char], i: usize) -> usize {
    let q = c[i];
    let mut j = i + 1;
    while j < c.len() && c[j] != q {
        j += if c[j] == '\\' { 2 } else { 1 };
    }
    (j + 1).min(c.len())
}

/// Bracketed predicate starting at `[`, up to the matching `]` (strings inside may contain brackets).
fn scan_bracket(c: &[char], mut i: usize) -> usize {
    let mut depth = 0;
    let mut quote: Option<char> = None;
    while i < c.len() {
        let x = c[i];
        i += 1;
        if let Some(q) = quote {
            if x == '\\' {
                i += 1; // skip the escaped char
            } else if x == q {
                quote = None;
            }
            continue;
        }
        match x {
            '\'' | '"' => quote = Some(x),
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
    }
    i
}

/// Path starting at `/`: runs to whitespace or a delimiter, predicates included.
fn scan_path(c: &[char], mut i: usize) -> usize {
    while i < c.len() && !c[i].is_whitespace() && !matches!(c[i], ',' | ')' | '(' | '=' | '<' | '>' | '!') && !(c[i] == '-' && c.get(i + 1) == Some(&'-')) {
        i = if c[i] == '[' { scan_bracket(c, i) } else { i + 1 };
    }
    i
}

fn classify_word(word: &str, next_is_paren: bool, prev_kw: &mut Option<String>) -> Tok {
    let after_container = matches!(prev_kw.as_deref(), Some("FROM" | "CONTAINS"));
    if after_container && is_member(TYPES, word) {
        Tok::Type
    } else if next_is_paren && is_member(FUNCTIONS, word) {
        Tok::Function
    } else if is_member(KEYWORDS, word) {
        *prev_kw = Some(word.to_ascii_uppercase());
        Tok::Keyword
    } else {
        Tok::Alias
    }
}

/// Scan one token starting at `i`: (end, kind).
fn next_token(c: &[char], i: usize, prev_kw: &mut Option<String>) -> (usize, Tok) {
    let ch = c[i];
    let next = c.get(i + 1).copied();
    match ch {
        '-' if next == Some('-') => (c.len(), Tok::Comment),
        '\'' | '"' => (scan_string(c, i), Tok::String),
        '[' => (scan_bracket(c, i), Tok::Archetype),
        '$' if next.is_some_and(is_ident_start) => (scan_while(c, i + 1, is_ident), Tok::Param),
        '/' => (scan_path(c, i), Tok::Path),
        '=' | '<' | '>' | '!' | '+' | '*' | '%' => (scan_while(c, i + 1, |x| matches!(x, '=' | '>')), Tok::Operator),
        '(' | ')' | ',' | ';' | '{' | '}' => (i + 1, Tok::Punct),
        _ if ch.is_ascii_digit() || (ch == '.' && next.is_some_and(|x| x.is_ascii_digit())) => (scan_while(c, i, |x| x.is_ascii_digit() || x == '.'), Tok::Number),
        _ if is_ident_start(ch) => {
            let end = scan_while(c, i, is_ident);
            let word: String = c[i..end].iter().collect();
            let paren = c[end..].iter().find(|x| **x != ' ') == Some(&'(');
            (end, classify_word(&word, paren, prev_kw))
        }
        _ => (i + 1, Tok::Plain),
    }
}

/// Tokenize one line. `prev_kw` is the last keyword seen before this line (for type detection).
/// Returns tokens covering every char (gaps are `Plain`) in order.
pub fn tokenize_line(line: &str, prev_kw: &mut Option<String>) -> Vec<(Range<usize>, Tok)> {
    let c: Vec<char> = line.chars().collect();
    let mut out: Vec<(Range<usize>, Tok)> = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let (end, kind) = next_token(&c, i, prev_kw);
        match out.last_mut() {
            Some((r, Tok::Plain)) if kind == Tok::Plain => r.end = end,
            _ => out.push((i..end, kind)),
        }
        i = end;
    }
    out
}

/// Tokenize the whole text, one Vec per line.
pub fn tokenize(text: &str) -> Vec<Vec<(Range<usize>, Tok)>> {
    let mut kw = None;
    text.split('\n').map(|l| tokenize_line(l, &mut kw)).collect()
}

/// Rewrite a query in a consistent layout: upper-case keywords, each major clause on its own line,
/// CONTAINS indented. Strings, comments, paths and archetype ids are left exactly as typed.
pub fn format(text: &str) -> String {
    let mut out = String::new();
    let mut kw = None;
    for (li, line) in text.split('\n').enumerate() {
        let toks = tokenize_line(line, &mut kw);
        let chars: Vec<char> = line.chars().collect();
        let mut s = String::new();
        for (r, k) in toks {
            let piece: String = chars[r].iter().collect();
            if matches!(k, Tok::Keyword | Tok::Type) {
                s.push_str(&piece.to_ascii_uppercase());
            } else {
                s.push_str(&piece);
            }
        }
        if li > 0 {
            out.push('\n');
        }
        out.push_str(s.trim_end());
    }
    layout_clauses(&out)
}

/// Put FROM / WHERE / ORDER BY / LIMIT / OFFSET on their own lines and indent CONTAINS, outside strings/comments.
fn layout_clauses(text: &str) -> String {
    let mut kw = None;
    let mut res: Vec<String> = Vec::new();
    for line in text.split('\n') {
        let toks = tokenize_line(line, &mut kw);
        let chars: Vec<char> = line.chars().collect();
        let mut cur = String::new();
        for (r, k) in toks {
            let piece: String = chars[r].iter().collect();
            let break_before = k == Tok::Keyword && matches!(piece.as_str(), "FROM" | "WHERE" | "LIMIT" | "OFFSET" | "GROUP" | "HAVING") || (k == Tok::Keyword && piece == "ORDER");
            let contains = k == Tok::Keyword && piece == "CONTAINS";
            if (break_before || contains) && !cur.trim().is_empty() {
                res.push(cur.trim_end().to_string());
                cur = if contains { "  ".to_string() } else { String::new() };
            }
            if cur.is_empty() && k == Tok::Plain && piece.trim().is_empty() {
                continue;
            }
            if cur.is_empty() && contains {
                cur.push_str("  ");
            }
            cur.push_str(&piece);
        }
        res.push(cur.trim_end().to_string());
    }
    res.join("\n")
}

/// Word being typed immediately before `col` on `line` (for autocomplete).
pub fn word_before(line: &str, col: usize) -> String {
    let chars: Vec<char> = line.chars().take(col).collect();
    let start = chars.iter().rposition(|c| !is_ident(*c)).map(|i| i + 1).unwrap_or(0);
    chars[start..].iter().collect()
}

/// Keyword/type suggestions for a prefix (case-insensitive, at least 2 chars, not an exact match).
pub fn suggest(prefix: &str) -> Vec<&'static str> {
    if prefix.chars().count() < 2 {
        return vec![];
    }
    let p = prefix.to_ascii_uppercase();
    COMPLETIONS.iter().copied().filter(|k| k.starts_with(&p) && *k != p).take(6).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(line: &str) -> Vec<(String, Tok)> {
        let mut kw = None;
        let ch: Vec<char> = line.chars().collect();
        tokenize_line(line, &mut kw).into_iter().filter(|(_, k)| *k != Tok::Plain).map(|(r, k)| (ch[r].iter().collect(), k)).collect()
    }

    #[test]
    fn keywords_case_insensitive() {
        let k = kinds("select c from EHR e");
        assert_eq!(k[0], ("select".into(), Tok::Keyword));
        assert!(k.contains(&("from".into(), Tok::Keyword)));
        assert!(k.contains(&("EHR".into(), Tok::Type)));
    }

    #[test]
    fn archetype_and_path() {
        let k = kinds("o/data[at0001]/events[at0006]/data/items[at0004]/value/magnitude AS sys");
        assert!(k.iter().any(|(s, t)| s.starts_with("/data[at0001]") && *t == Tok::Path));
        let k = kinds("CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.blood_pressure.v2]");
        assert!(k.contains(&("OBSERVATION".into(), Tok::Type)));
        assert!(k.contains(&("[openEHR-EHR-OBSERVATION.blood_pressure.v2]".into(), Tok::Archetype)));
    }

    #[test]
    fn strings_numbers_params_comments() {
        let k = kinds("WHERE x = 'a -- b' AND y > 3.5 AND z = $ehr_id -- trailing");
        assert!(k.contains(&("'a -- b'".into(), Tok::String)));
        assert!(k.contains(&("3.5".into(), Tok::Number)));
        assert!(k.contains(&("$ehr_id".into(), Tok::Param)));
        assert!(k.contains(&("-- trailing".into(), Tok::Comment)));
    }

    #[test]
    fn functions_only_with_parens() {
        let k = kinds("SELECT COUNT(*) FROM EHR e");
        assert!(k.contains(&("COUNT".into(), Tok::Function)));
        let k = kinds("SELECT count FROM EHR e");
        assert!(!k.iter().any(|(_, t)| *t == Tok::Function));
    }

    #[test]
    fn type_only_after_from_or_contains() {
        let k = kinds("SELECT element FROM EHR e");
        assert!(!k.contains(&("element".into(), Tok::Type)));
    }

    #[test]
    fn tokens_cover_whole_line_in_order() {
        let line = "SELECT e/ehr_id/value, 'x' FROM EHR e [at0001] -- c";
        let mut kw = None;
        let t = tokenize_line(line, &mut kw);
        let mut pos = 0;
        for (r, _) in &t {
            assert_eq!(r.start, pos);
            pos = r.end;
        }
        assert_eq!(pos, line.chars().count());
    }

    #[test]
    fn escaped_quotes_do_not_end_strings_or_predicates() {
        let k = kinds(r#"WHERE a = 'it\'s' AND b = "x\"y""#);
        assert!(k.contains(&(r"'it\'s'".into(), Tok::String)));
        assert!(k.contains(&(r#""x\"y""#.into(), Tok::String)));
        let k = kinds(r#"o[name/value='a\']b'] x"#);
        assert!(k.contains(&(r"[name/value='a\']b']".into(), Tok::Archetype)));
    }

    #[test]
    fn comment_right_after_a_path_is_a_comment() {
        let k = kinds("SELECT c/uid/value--note");
        assert!(k.contains(&("/uid/value".into(), Tok::Path)));
        assert!(k.contains(&("--note".into(), Tok::Comment)));
    }

    #[test]
    fn unterminated_string_runs_to_eol() {
        let k = kinds("WHERE a = 'abc");
        assert!(k.contains(&("'abc".into(), Tok::String)));
    }

    #[test]
    fn format_uppercases_and_breaks_clauses() {
        let f = format("select e/ehr_id/value as id from ehr e contains composition c where c/name/value = 'select' limit 10");
        assert_eq!(f, "SELECT e/ehr_id/value AS id\nFROM EHR e\n  CONTAINS COMPOSITION c\nWHERE c/name/value = 'select'\nLIMIT 10");
    }

    #[test]
    fn format_is_idempotent() {
        let once = format("select a from ehr e contains composition c order by a desc limit 5");
        assert_eq!(format(&once), once);
    }

    #[test]
    fn suggestions() {
        assert_eq!(word_before("SELECT ob", 9), "ob");
        assert!(suggest("ob").contains(&"OBSERVATION"));
        assert!(suggest("o").is_empty());
        assert!(!suggest("SELECT").contains(&"SELECT"));
    }
}
