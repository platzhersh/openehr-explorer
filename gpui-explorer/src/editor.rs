//! Pure text-editing model for the AQL editor: selection, undo/redo, word navigation, auto-indent,
//! bracket/quote pairing, indent/dedent and comment toggling. No GPUI types, so it is unit-tested.
//! Positions are char indices into `chars`.

#[derive(Clone)]
struct Snap {
    chars: Vec<char>,
    cursor: usize,
    anchor: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Typing,
    Deleting,
    Other,
}

pub struct Editor {
    pub chars: Vec<char>,
    pub cursor: usize,
    /// The selection runs between `anchor` and `cursor` (equal = no selection).
    pub anchor: usize,
    undo: Vec<Snap>,
    redo: Vec<Snap>,
    last: Kind,
    /// Column to return to when moving vertically through shorter lines.
    goal_col: Option<usize>,
    /// Bumped by every edit, so the UI can tell when to re-tokenize.
    pub version: u64,
}

/// One keystroke, decoupled from GPUI's type.
pub struct Key<'a> {
    pub name: &'a str,
    pub ch: Option<&'a str>,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

#[derive(Default, Debug, PartialEq)]
pub struct Out {
    pub handled: bool,
    pub clipboard: Option<String>,
}

const INDENT: &str = "  ";

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn closer(c: char) -> Option<char> {
    match c {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '\'' => Some('\''),
        '"' => Some('"'),
        _ => None,
    }
}

impl Editor {
    pub fn new(text: &str) -> Self {
        let chars: Vec<char> = text.replace("\r\n", "\n").chars().collect();
        let n = chars.len();
        Self { chars, cursor: n, anchor: n, undo: vec![], redo: vec![], last: Kind::Other, goal_col: None, version: 0 }
    }

    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }

    pub fn set_text(&mut self, text: &str) {
        self.checkpoint(Kind::Other);
        self.chars = text.replace("\r\n", "\n").chars().collect();
        self.cursor = self.chars.len();
        self.anchor = self.cursor;
        self.version += 1;
    }

    // ---- geometry ------------------------------------------------------------------------------

    pub fn line_count(&self) -> usize {
        self.chars.iter().filter(|c| **c == '\n').count() + 1
    }

    pub fn line_start(&self, pos: usize) -> usize {
        self.chars[..pos].iter().rposition(|c| *c == '\n').map(|i| i + 1).unwrap_or(0)
    }

    pub fn line_end(&self, pos: usize) -> usize {
        self.chars[pos..].iter().position(|c| *c == '\n').map(|i| pos + i).unwrap_or(self.chars.len())
    }

    pub fn line_col(&self, pos: usize) -> (usize, usize) {
        let start = self.line_start(pos);
        (self.chars[..start].iter().filter(|c| **c == '\n').count(), pos - start)
    }

    pub fn pos_at(&self, line: usize, col: usize) -> usize {
        let mut start = 0;
        for _ in 0..line {
            match self.chars[start..].iter().position(|c| *c == '\n') {
                Some(i) => start += i + 1,
                None => return self.chars.len(),
            }
        }
        start + col.min(self.line_end(start) - start)
    }

    pub fn selection(&self) -> (usize, usize) {
        (self.cursor.min(self.anchor), self.cursor.max(self.anchor))
    }

    pub fn has_selection(&self) -> bool {
        self.cursor != self.anchor
    }

    pub fn selected_text(&self) -> String {
        let (a, b) = self.selection();
        self.chars[a..b].iter().collect()
    }

    /// Lines (inclusive) covered by the selection, or the cursor line.
    fn line_range(&self) -> (usize, usize) {
        let (a, b) = self.selection();
        let (l0, _) = self.line_col(a);
        let (mut l1, c1) = self.line_col(b);
        // A selection that ends at column 0 of a later line does not include that line.
        if b > a && c1 == 0 && l1 > l0 {
            l1 -= 1;
        }
        (l0, l1)
    }

    // ---- history -------------------------------------------------------------------------------

    fn snap(&self) -> Snap {
        Snap { chars: self.chars.clone(), cursor: self.cursor, anchor: self.anchor }
    }

    /// Record an undo point unless this edit continues the previous run of typing/deleting.
    fn checkpoint(&mut self, kind: Kind) {
        if kind == Kind::Other || kind != self.last {
            self.undo.push(self.snap());
            if self.undo.len() > 200 {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
        self.last = kind;
        self.version += 1;
    }

    pub fn undo(&mut self) -> bool {
        let Some(s) = self.undo.pop() else { return false };
        self.redo.push(self.snap());
        (self.chars, self.cursor, self.anchor) = (s.chars, s.cursor, s.anchor);
        self.last = Kind::Other;
        self.version += 1;
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(s) = self.redo.pop() else { return false };
        self.undo.push(self.snap());
        (self.chars, self.cursor, self.anchor) = (s.chars, s.cursor, s.anchor);
        self.last = Kind::Other;
        self.version += 1;
        true
    }

    // ---- primitive edits -----------------------------------------------------------------------

    fn replace(&mut self, a: usize, b: usize, with: &[char]) {
        self.chars.splice(a..b, with.iter().copied());
        self.cursor = a + with.len();
        self.anchor = self.cursor;
        self.goal_col = None;
    }

    fn delete_selection(&mut self) -> bool {
        if !self.has_selection() {
            return false;
        }
        let (a, b) = self.selection();
        self.replace(a, b, &[]);
        true
    }

    /// Insert text (paste or typed), replacing the selection.
    pub fn insert_str(&mut self, s: &str) {
        let clean: Vec<char> = s.replace("\r\n", "\n").replace('\r', "\n").chars().filter(|c| !c.is_control() || *c == '\n').collect();
        if clean.is_empty() && !self.has_selection() {
            return;
        }
        self.checkpoint(Kind::Other);
        let (a, b) = self.selection();
        self.replace(a, b, &clean);
    }

    fn peek(&self, pos: usize) -> Option<char> {
        self.chars.get(pos).copied()
    }

    /// Type one character, with bracket/quote pairing.
    pub fn type_char(&mut self, c: char) {
        let kind = if is_word(c) { Kind::Typing } else { Kind::Other };
        if self.has_selection() {
            if let Some(close) = closer(c) {
                // Wrap the selection.
                self.checkpoint(Kind::Other);
                let (a, b) = self.selection();
                let mut wrapped = vec![c];
                wrapped.extend_from_slice(&self.chars[a..b].to_vec());
                wrapped.push(close);
                self.replace(a, b, &wrapped);
                self.anchor = a + 1;
                self.cursor = b + 1;
                return;
            }
        }
        // Typing a closer that is already next: just step over it.
        if matches!(c, ')' | ']' | '}' | '\'' | '"') && !self.has_selection() && self.peek(self.cursor) == Some(c) {
            self.cursor += 1;
            self.anchor = self.cursor;
            self.goal_col = None;
            return;
        }
        self.checkpoint(kind);
        self.delete_selection();
        let prev = if self.cursor > 0 { self.peek(self.cursor - 1) } else { None };
        let next = self.peek(self.cursor);
        if let Some(close) = closer(c) {
            let quote = c == '\'' || c == '"';
            let next_ok = next.is_none_or(|n| !is_word(n) && n != c);
            let prev_ok = !quote || prev.is_none_or(|p| !is_word(p));
            if next_ok && prev_ok {
                let at = self.cursor;
                self.replace(at, at, &[c, close]);
                self.cursor = at + 1;
                self.anchor = self.cursor;
                self.last = Kind::Other;
                return;
            }
        }
        let at = self.cursor;
        self.replace(at, at, &[c]);
    }

    fn indent_of(&self, pos: usize) -> String {
        let s = self.line_start(pos);
        self.chars[s..].iter().take_while(|c| **c == ' ').collect()
    }

    pub fn newline(&mut self) {
        self.checkpoint(Kind::Other);
        self.delete_selection();
        let indent = self.indent_of(self.cursor);
        let before = if self.cursor > 0 { self.peek(self.cursor - 1) } else { None };
        let after = self.peek(self.cursor);
        let mut ins = format!("\n{indent}");
        let mut back = 0;
        if matches!(before, Some('(' | '[' | '{')) {
            ins.push_str(INDENT);
            if before.and_then(closer) == after {
                // Cursor sits between a fresh pair: open a block and keep the closer on its own line.
                let mut tail = format!("\n{indent}");
                back = tail.chars().count();
                ins.push_str(&std::mem::take(&mut tail));
            }
        }
        let v: Vec<char> = ins.chars().collect();
        let at = self.cursor;
        self.replace(at, at, &v);
        self.cursor -= back;
        self.anchor = self.cursor;
    }

    pub fn backspace(&mut self, by_word: bool) {
        if self.has_selection() {
            self.checkpoint(Kind::Other);
            self.delete_selection();
            return;
        }
        if self.cursor == 0 {
            return;
        }
        self.checkpoint(Kind::Deleting);
        let c = self.cursor;
        // Deleting an empty pair removes both halves.
        if let (Some(p), Some(n)) = (self.peek(c - 1), self.peek(c)) {
            if closer(p) == Some(n) {
                self.replace(c - 1, c + 1, &[]);
                return;
            }
        }
        // Inside leading whitespace, delete back to the previous indent stop.
        let start = self.line_start(c);
        if !by_word && c - start > 0 && self.chars[start..c].iter().all(|x| *x == ' ') {
            let col = c - start;
            let drop = if col % INDENT.len() == 0 { INDENT.len() } else { col % INDENT.len() };
            self.replace(c - drop, c, &[]);
            return;
        }
        let from = if by_word { self.word_left(c) } else { c - 1 };
        self.replace(from, c, &[]);
    }

    pub fn delete_forward(&mut self, by_word: bool) {
        if self.has_selection() {
            self.checkpoint(Kind::Other);
            self.delete_selection();
            return;
        }
        if self.cursor >= self.chars.len() {
            return;
        }
        self.checkpoint(Kind::Deleting);
        let c = self.cursor;
        let to = if by_word { self.word_right(c) } else { c + 1 };
        self.replace(c, to, &[]);
    }

    // ---- navigation ----------------------------------------------------------------------------

    pub fn word_left(&self, mut p: usize) -> usize {
        while p > 0 && !is_word(self.chars[p - 1]) && self.chars[p - 1] != '\n' {
            p -= 1;
        }
        if p > 0 && self.chars[p - 1] == '\n' {
            return p - 1;
        }
        while p > 0 && is_word(self.chars[p - 1]) {
            p -= 1;
        }
        p
    }

    pub fn word_right(&self, mut p: usize) -> usize {
        let n = self.chars.len();
        while p < n && !is_word(self.chars[p]) && self.chars[p] != '\n' {
            p += 1;
        }
        if p < n && self.chars[p] == '\n' {
            return p + 1;
        }
        while p < n && is_word(self.chars[p]) {
            p += 1;
        }
        p
    }

    fn move_to(&mut self, pos: usize, extend: bool) {
        self.cursor = pos.min(self.chars.len());
        if !extend {
            self.anchor = self.cursor;
        }
        self.last = Kind::Other;
    }

    pub fn move_left(&mut self, word: bool, extend: bool) {
        self.goal_col = None;
        if self.has_selection() && !extend {
            let (a, _) = self.selection();
            return self.move_to(a, false);
        }
        let to = if word { self.word_left(self.cursor) } else { self.cursor.saturating_sub(1) };
        self.move_to(to, extend);
    }

    pub fn move_right(&mut self, word: bool, extend: bool) {
        self.goal_col = None;
        if self.has_selection() && !extend {
            let (_, b) = self.selection();
            return self.move_to(b, false);
        }
        let to = if word { self.word_right(self.cursor) } else { self.cursor + 1 };
        self.move_to(to, extend);
    }

    pub fn move_vertical(&mut self, delta: isize, extend: bool) {
        let (line, col) = self.line_col(self.cursor);
        let goal = *self.goal_col.get_or_insert(col);
        let target = line as isize + delta;
        if target < 0 {
            return self.move_to(0, extend);
        }
        if target as usize >= self.line_count() {
            return self.move_to(self.chars.len(), extend);
        }
        let keep = self.goal_col;
        let pos = self.pos_at(target as usize, goal);
        self.move_to(pos, extend);
        self.goal_col = keep;
    }

    /// Smart Home: first press goes to the first non-blank, second to column 0.
    pub fn home(&mut self, doc: bool, extend: bool) {
        self.goal_col = None;
        if doc {
            return self.move_to(0, extend);
        }
        let s = self.line_start(self.cursor);
        let first = s + self.chars[s..].iter().take_while(|c| **c == ' ').count();
        let first = first.min(self.line_end(s));
        self.move_to(if self.cursor == first { s } else { first }, extend);
    }

    pub fn end(&mut self, doc: bool, extend: bool) {
        self.goal_col = None;
        let to = if doc { self.chars.len() } else { self.line_end(self.cursor) };
        self.move_to(to, extend);
    }

    /// Replace the `prefix_len` chars before the caret with `text` (autocomplete).
    pub fn accept_completion(&mut self, prefix_len: usize, text: &str) {
        self.checkpoint(Kind::Other);
        let to = self.cursor;
        let from = to.saturating_sub(prefix_len);
        let v: Vec<char> = text.chars().collect();
        self.replace(from, to, &v);
    }

    pub fn select_all(&mut self) {
        self.anchor = 0;
        self.cursor = self.chars.len();
        self.last = Kind::Other;
    }

    /// Mouse: place the caret (or extend the selection when `extend`).
    pub fn click(&mut self, pos: usize, extend: bool) {
        self.goal_col = None;
        self.move_to(pos, extend);
    }

    pub fn select_word_at(&mut self, pos: usize) {
        let pos = pos.min(self.chars.len());
        let (mut a, mut b) = (pos, pos);
        while a > 0 && is_word(self.chars[a - 1]) {
            a -= 1;
        }
        while b < self.chars.len() && is_word(self.chars[b]) {
            b += 1;
        }
        self.anchor = a;
        self.cursor = b;
    }

    pub fn select_line_at(&mut self, pos: usize) {
        let pos = pos.min(self.chars.len());
        self.anchor = self.line_start(pos);
        self.cursor = (self.line_end(pos) + 1).min(self.chars.len());
    }

    // ---- line operations -----------------------------------------------------------------------

    /// Rewrite lines `l0..=l1` with `f`, keeping the selection endpoints at their text.
    fn map_lines(&mut self, l0: usize, l1: usize, f: impl Fn(&str) -> String) {
        self.checkpoint(Kind::Other);
        let text = self.text();
        let lines: Vec<&str> = text.split('\n').collect();
        let (cl, cc) = self.line_col(self.cursor);
        let (al, ac) = self.line_col(self.anchor);
        let mut out: Vec<String> = Vec::with_capacity(lines.len());
        let mut delta = vec![0isize; lines.len()];
        for (i, l) in lines.iter().enumerate() {
            if i >= l0 && i <= l1 {
                let n = f(l);
                delta[i] = n.chars().count() as isize - l.chars().count() as isize;
                out.push(n);
            } else {
                out.push((*l).to_string());
            }
        }
        self.chars = out.join("\n").chars().collect();
        let fix = |chars: &Self, line: usize, col: usize, d: isize| chars.pos_at(line, (col as isize + d).max(0) as usize);
        let (cd, ad) = (delta[cl], delta[al]);
        let (c, a) = (fix(self, cl, cc, cd), fix(self, al, ac, ad));
        self.cursor = c;
        self.anchor = a;
    }

    pub fn indent(&mut self) {
        let (l0, l1) = self.line_range();
        if l0 == l1 && !self.has_selection() {
            // Plain Tab inside a line: pad to the next indent stop.
            let (_, col) = self.line_col(self.cursor);
            let pad = INDENT.len() - col % INDENT.len();
            return self.insert_str(&" ".repeat(pad));
        }
        self.map_lines(l0, l1, |l| if l.is_empty() { String::new() } else { format!("{INDENT}{l}") });
    }

    pub fn dedent(&mut self) {
        let (l0, l1) = self.line_range();
        self.map_lines(l0, l1, |l| {
            let n = l.chars().take(INDENT.len()).take_while(|c| *c == ' ').count();
            l.chars().skip(n).collect()
        });
    }

    pub fn toggle_comment(&mut self) {
        let (l0, l1) = self.line_range();
        let text = self.text();
        let lines: Vec<&str> = text.split('\n').collect();
        let body: Vec<&&str> = lines[l0..=l1].iter().filter(|l| !l.trim().is_empty()).collect();
        let all = !body.is_empty() && body.iter().all(|l| l.trim_start().starts_with("--"));
        let min_indent = body.iter().map(|l| l.chars().take_while(|c| *c == ' ').count()).min().unwrap_or(0);
        self.map_lines(l0, l1, move |l| {
            if l.trim().is_empty() {
                return l.to_string();
            }
            if all {
                let ind = l.chars().take_while(|c| *c == ' ').count();
                let rest: String = l.chars().skip(ind).skip(2).collect();
                let rest = rest.strip_prefix(' ').unwrap_or(&rest);
                format!("{}{rest}", " ".repeat(ind))
            } else {
                let head: String = l.chars().take(min_indent).collect();
                let tail: String = l.chars().skip(min_indent).collect();
                format!("{head}-- {tail}")
            }
        });
    }

    pub fn duplicate_line(&mut self) {
        let (l0, l1) = self.line_range();
        let text = self.text();
        let lines: Vec<&str> = text.split('\n').collect();
        let dup = lines[l0..=l1].join("\n");
        let end = self.pos_at(l1, usize::MAX);
        self.checkpoint(Kind::Other);
        let ins: Vec<char> = format!("\n{dup}").chars().collect();
        let (cur, anc) = (self.cursor, self.anchor);
        self.chars.splice(end..end, ins.iter().copied());
        let shift = ins.len();
        self.cursor = if cur >= end { cur + shift } else { cur };
        self.anchor = if anc >= end { anc + shift } else { anc };
    }

    // ---- key dispatch --------------------------------------------------------------------------

    /// Apply a key. `paste` is the clipboard text when the key is Ctrl/Cmd+V.
    pub fn key(&mut self, k: &Key, paste: Option<String>) -> Out {
        let ext = k.shift;
        let mut out = Out { handled: true, clipboard: None };
        if k.ctrl && !k.alt {
            match k.name {
                "a" => self.select_all(),
                "c" => {
                    if self.has_selection() {
                        out.clipboard = Some(self.selected_text());
                    }
                }
                "x" => {
                    if self.has_selection() {
                        out.clipboard = Some(self.selected_text());
                        self.checkpoint(Kind::Other);
                        self.delete_selection();
                    }
                }
                "v" => {
                    if let Some(t) = paste {
                        self.insert_str(&t);
                    }
                }
                "z" if k.shift => {
                    self.redo();
                }
                "z" => {
                    self.undo();
                }
                "y" => {
                    self.redo();
                }
                "/" => self.toggle_comment(),
                "d" => self.duplicate_line(),
                "left" => self.move_left(true, ext),
                "right" => self.move_right(true, ext),
                "home" => self.home(true, ext),
                "end" => self.end(true, ext),
                "backspace" => self.backspace(true),
                "delete" => self.delete_forward(true),
                "[" => self.dedent(),
                "]" => self.indent(),
                _ => out.handled = false,
            }
            return out;
        }
        match k.name {
            "left" => self.move_left(false, ext),
            "right" => self.move_right(false, ext),
            "up" => self.move_vertical(-1, ext),
            "down" => self.move_vertical(1, ext),
            "home" => self.home(false, ext),
            "end" => self.end(false, ext),
            "backspace" => self.backspace(false),
            "delete" => self.delete_forward(false),
            "enter" => self.newline(),
            "tab" if k.shift => self.dedent(),
            "tab" => self.indent(),
            "escape" => {
                self.anchor = self.cursor;
            }
            _ => match k.ch {
                Some(ch) if !k.alt && !ch.is_empty() => {
                    let mut it = ch.chars();
                    match (it.next(), it.next()) {
                        (Some(c), None) => self.type_char(c),
                        _ => self.insert_str(ch),
                    }
                }
                _ => out.handled = false,
            },
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ed(s: &str) -> Editor {
        Editor::new(s)
    }

    fn press(e: &mut Editor, name: &str, ctrl: bool, shift: bool) {
        e.key(&Key { name, ch: None, ctrl, shift, alt: false }, None);
    }

    fn typ(e: &mut Editor, s: &str) {
        for c in s.chars() {
            let t = c.to_string();
            e.key(&Key { name: "x", ch: Some(&t), ctrl: false, shift: false, alt: false }, None);
        }
    }

    #[test]
    fn typing_and_backspace() {
        let mut e = ed("");
        typ(&mut e, "SELECT");
        assert_eq!(e.text(), "SELECT");
        press(&mut e, "backspace", false, false);
        assert_eq!(e.text(), "SELEC");
    }

    #[test]
    fn auto_close_and_skip_over() {
        let mut e = ed("");
        typ(&mut e, "o[");
        assert_eq!(e.text(), "o[]");
        assert_eq!(e.cursor, 2);
        typ(&mut e, "at0001]");
        assert_eq!(e.text(), "o[at0001]");
        assert_eq!(e.cursor, 9);
    }

    #[test]
    fn quote_pairing_skips_after_word() {
        let mut e = ed("");
        typ(&mut e, "don't");
        assert_eq!(e.text(), "don't");
        let mut e = ed("");
        typ(&mut e, "'abc'");
        assert_eq!(e.text(), "'abc'");
    }

    #[test]
    fn backspace_removes_empty_pair() {
        let mut e = ed("");
        typ(&mut e, "(");
        assert_eq!(e.text(), "()");
        press(&mut e, "backspace", false, false);
        assert_eq!(e.text(), "");
    }

    #[test]
    fn wrap_selection_in_brackets() {
        let mut e = ed("abc");
        e.select_all();
        typ(&mut e, "(");
        assert_eq!(e.text(), "(abc)");
        assert_eq!(e.selected_text(), "abc");
    }

    #[test]
    fn enter_keeps_indent_and_opens_blocks() {
        let mut e = ed("  FROM EHR e");
        press(&mut e, "enter", false, false);
        assert_eq!(e.text(), "  FROM EHR e\n  ");
        let mut e = ed("");
        typ(&mut e, "(");
        press(&mut e, "enter", false, false);
        assert_eq!(e.text(), "(\n  \n)");
        assert_eq!(e.line_col(e.cursor), (1, 2));
    }

    #[test]
    fn undo_coalesces_typing_runs() {
        let mut e = ed("");
        typ(&mut e, "abc");
        typ(&mut e, " ");
        typ(&mut e, "def");
        assert_eq!(e.text(), "abc def");
        e.undo();
        assert_eq!(e.text(), "abc ");
        e.undo();
        assert_eq!(e.text(), "abc");
        e.undo();
        assert_eq!(e.text(), "");
        e.redo();
        assert_eq!(e.text(), "abc");
    }

    #[test]
    fn selection_and_shift_arrows() {
        let mut e = ed("hello world");
        e.click(0, false);
        for _ in 0..5 {
            press(&mut e, "right", false, true);
        }
        assert_eq!(e.selected_text(), "hello");
        typ(&mut e, "X");
        assert_eq!(e.text(), "X world");
    }

    #[test]
    fn word_navigation() {
        let mut e = ed("SELECT c/uid/value FROM EHR");
        e.click(0, false);
        e.move_right(true, false);
        assert_eq!(e.cursor, 6);
        e.move_right(true, false);
        assert_eq!(e.cursor, 8);
        e.click(27, false);
        e.move_left(true, false);
        assert_eq!(e.cursor, 24);
    }

    #[test]
    fn vertical_keeps_goal_column() {
        let mut e = ed("abcdef\nab\nabcdef");
        e.click(5, false);
        e.move_vertical(1, false);
        assert_eq!(e.line_col(e.cursor), (1, 2));
        e.move_vertical(1, false);
        assert_eq!(e.line_col(e.cursor), (2, 5));
    }

    #[test]
    fn smart_home_toggles() {
        let mut e = ed("    x");
        e.end(false, false);
        e.home(false, false);
        assert_eq!(e.cursor, 4);
        e.home(false, false);
        assert_eq!(e.cursor, 0);
    }

    #[test]
    fn indent_dedent_block() {
        let mut e = ed("a\nb\nc");
        e.select_all();
        e.indent();
        assert_eq!(e.text(), "  a\n  b\n  c");
        e.dedent();
        assert_eq!(e.text(), "a\nb\nc");
        assert_eq!(e.selection(), (0, 5));
    }

    #[test]
    fn tab_pads_to_stop() {
        let mut e = ed("a");
        e.indent();
        assert_eq!(e.text(), "a ");
    }

    #[test]
    fn comment_toggle_roundtrip() {
        let mut e = ed("SELECT x\n  FROM EHR e");
        e.select_all();
        e.toggle_comment();
        assert_eq!(e.text(), "-- SELECT x\n--   FROM EHR e");
        e.toggle_comment();
        assert_eq!(e.text(), "SELECT x\n  FROM EHR e");
    }

    #[test]
    fn cut_copy_paste() {
        let mut e = ed("hello world");
        e.click(0, false);
        e.move_right(true, true);
        let out = e.key(&Key { name: "x", ch: None, ctrl: true, shift: false, alt: false }, None);
        assert_eq!(out.clipboard.as_deref(), Some("hello"));
        assert_eq!(e.text(), " world");
        let out = e.key(&Key { name: "v", ch: None, ctrl: true, shift: false, alt: false }, Some("a\r\nb".into()));
        assert!(out.handled);
        assert_eq!(e.text(), "a\nb world");
    }

    #[test]
    fn select_word_and_line() {
        let mut e = ed("foo bar\nbaz");
        e.select_word_at(5);
        assert_eq!(e.selected_text(), "bar");
        e.select_line_at(1);
        assert_eq!(e.selected_text(), "foo bar\n");
    }

    #[test]
    fn duplicate_line_copies_below() {
        let mut e = ed("a\nb");
        e.click(0, false);
        e.duplicate_line();
        assert_eq!(e.text(), "a\na\nb");
    }
}
