//! `wp_text_diff()`: the PEAR `Text_Diff` (native engine), its inline renderer and WordPress' table renderer, as
//! WordPress ships them (`wp-includes/Text`, `class-wp-text-diff-renderer-table.php`). The line-by-line algorithm,
//! down to the quirks of PHP arrays, is kept so that the revisions screen shows the same differences.

use std::collections::{BTreeMap, HashMap, HashSet};

/// One edit of a diff (`Text_Diff_Op_*`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Copy(Vec<String>),
    Delete(Vec<String>),
    Add(Vec<String>),
    Change(Vec<String>, Vec<String>),
}

impl Op {
    fn orig(&self) -> &[String] {
        match self {
            Op::Copy(lines) | Op::Delete(lines) | Op::Change(lines, _) => lines,
            Op::Add(_) => &[],
        }
    }

    fn fin(&self) -> &[String] {
        match self {
            Op::Copy(lines) | Op::Add(lines) | Op::Change(_, lines) => lines,
            Op::Delete(_) => &[],
        }
    }
}

/// PHP's `==` of two strings: numeric strings compare as numbers.
fn loose_eq(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    match (numeric(a), numeric(b)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

fn numeric(s: &str) -> Option<f64> {
    let t = s.trim_matches(|c: char| matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{b}' | '\u{c}'));
    if t.is_empty() || !t.chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-')) || !t.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    t.parse::<f64>().ok()
}

/// `Text_Diff_Engine_native`.
#[derive(Default)]
struct Native {
    xchanged: Vec<bool>,
    ychanged: Vec<bool>,
    xv: Vec<String>,
    yv: Vec<String>,
    xind: Vec<usize>,
    yind: Vec<usize>,
    seq: Vec<i64>,
    in_seq: HashMap<i64, bool>,
    lcs: usize,
}

impl Native {
    fn diff(from: &[String], to: &[String]) -> Vec<Op> {
        let trim = |s: &String| s.replace(['\n', '\r'], "");
        let from_lines: Vec<String> = from.iter().map(trim).collect();
        let to_lines: Vec<String> = to.iter().map(trim).collect();
        let n_from = from_lines.len();
        let n_to = to_lines.len();
        let mut e = Native { xchanged: vec![false; n_from], ychanged: vec![false; n_to], ..Default::default() };

        let mut skip = 0;
        while skip < n_from && skip < n_to {
            if from_lines[skip] != to_lines[skip] {
                break;
            }
            skip += 1;
        }
        let mut endskip = 0;
        let (mut xi, mut yi) = (n_from as i64, n_to as i64);
        loop {
            xi -= 1;
            yi -= 1;
            if !(xi > skip as i64 && yi > skip as i64) {
                break;
            }
            if from_lines[xi as usize] != to_lines[yi as usize] {
                break;
            }
            endskip += 1;
        }

        let xhash: HashSet<&str> = (skip..n_from - endskip).map(|i| from_lines[i].as_str()).collect();
        let mut yhash: HashSet<&str> = HashSet::new();
        for yi in skip..n_to - endskip {
            let line = to_lines[yi].as_str();
            let changed = !xhash.contains(line);
            e.ychanged[yi] = changed;
            if changed {
                continue;
            }
            yhash.insert(line);
            e.yv.push(line.to_owned());
            e.yind.push(yi);
        }
        for xi in skip..n_from - endskip {
            let line = from_lines[xi].as_str();
            let changed = !yhash.contains(line);
            e.xchanged[xi] = changed;
            if changed {
                continue;
            }
            e.xv.push(line.to_owned());
            e.xind.push(xi);
        }

        let (xl, yl) = (e.xv.len(), e.yv.len());
        e.compareseq(0, xl, 0, yl);

        let mut xchanged = std::mem::take(&mut e.xchanged);
        let mut ychanged = std::mem::take(&mut e.ychanged);
        shift_boundaries(&from_lines, &mut xchanged, &ychanged);
        shift_boundaries(&to_lines, &mut ychanged, &xchanged);

        let mut edits = Vec::new();
        let (mut xi, mut yi) = (0, 0);
        while xi < n_from || yi < n_to {
            let mut copy = Vec::new();
            while xi < n_from && yi < n_to && !xchanged[xi] && !ychanged[yi] {
                copy.push(from_lines[xi].clone());
                xi += 1;
                yi += 1;
            }
            if !copy.is_empty() {
                edits.push(Op::Copy(copy));
            }
            let mut delete = Vec::new();
            while xi < n_from && xchanged[xi] {
                delete.push(from_lines[xi].clone());
                xi += 1;
            }
            let mut add = Vec::new();
            while yi < n_to && ychanged[yi] {
                add.push(to_lines[yi].clone());
                yi += 1;
            }
            match (delete.is_empty(), add.is_empty()) {
                (false, false) => edits.push(Op::Change(delete, add)),
                (false, true) => edits.push(Op::Delete(delete)),
                (true, false) => edits.push(Op::Add(add)),
                (true, true) => {}
            }
        }
        edits
    }

    fn lcs_pos(&mut self, ypos: i64) -> usize {
        let mut end = self.lcs;
        if end == 0 || ypos > self.seq[end] {
            self.lcs += 1;
            if self.seq.len() <= self.lcs {
                self.seq.push(ypos);
            } else {
                self.seq[self.lcs] = ypos;
            }
            self.in_seq.insert(ypos, true);
            return self.lcs;
        }
        let mut beg = 1;
        while beg < end {
            let mid = (beg + end) / 2;
            if ypos > self.seq[mid] {
                beg = mid + 1;
            } else {
                end = mid;
            }
        }
        self.in_seq.insert(self.seq[end], false);
        self.seq[end] = ypos;
        self.in_seq.insert(ypos, true);
        end
    }

    fn in_seq(&self, y: i64) -> bool {
        self.in_seq.get(&y).copied().unwrap_or(false)
    }

    /// `_diag()`: `(lcs, seps)`.
    fn diag(&mut self, mut xoff: usize, mut xlim: usize, mut yoff: usize, mut ylim: usize, nchunks: usize) -> (usize, Vec<(usize, usize)>) {
        let mut flip = false;
        if xlim - xoff > ylim - yoff {
            // Things seems faster (I'm not sure I understand why) when the shortest sequence is in X.
            flip = true;
            std::mem::swap(&mut xoff, &mut yoff);
            std::mem::swap(&mut xlim, &mut ylim);
        }
        let mut ymatches: HashMap<String, Vec<i64>> = HashMap::new();
        for i in (yoff..ylim).rev() {
            let key = if flip { self.xv[i].clone() } else { self.yv[i].clone() };
            ymatches.entry(key).or_default().push(i as i64);
        }
        self.lcs = 0;
        self.seq = vec![yoff as i64 - 1];
        self.in_seq = HashMap::new();
        let mut ymids: Vec<BTreeMap<usize, i64>> = vec![BTreeMap::new()];

        let numer = xlim - xoff + nchunks - 1;
        let mut x = xoff;
        let mut k = 0usize;
        for chunk in 0..nchunks {
            if chunk > 0 {
                for i in 0..=self.lcs {
                    let value = self.seq[i];
                    if ymids.len() <= i {
                        ymids.resize(i + 1, BTreeMap::new());
                    }
                    ymids[i].insert(chunk - 1, value);
                }
            }
            let x1 = xoff + (numer + (xlim - xoff) * chunk) / nchunks;
            while x < x1 {
                let line = if flip { self.yv[x].clone() } else { self.xv[x].clone() };
                x += 1;
                let Some(matches) = ymatches.get(&line).cloned() else { continue };
                if matches.is_empty() {
                    continue;
                }
                let mut p = 0;
                while p < matches.len() && matches[p] != 0 {
                    let y = matches[p];
                    if !self.in_seq(y) {
                        k = self.lcs_pos(y);
                        let prev = ymids.get(k - 1).cloned().unwrap_or_default();
                        if ymids.len() <= k {
                            ymids.resize(k + 1, BTreeMap::new());
                        }
                        ymids[k] = prev;
                        break;
                    }
                    p += 1;
                }
                while p < matches.len() && matches[p] != 0 {
                    let y = matches[p];
                    if y > self.seq[k - 1] {
                        // Optimization: this is a common case: next match is just replacing previous match.
                        let old = self.seq[k];
                        self.in_seq.insert(old, false);
                        self.seq[k] = y;
                        self.in_seq.insert(y, true);
                    } else if !self.in_seq(y) {
                        k = self.lcs_pos(y);
                        let prev = ymids.get(k - 1).cloned().unwrap_or_default();
                        if ymids.len() <= k {
                            ymids.resize(k + 1, BTreeMap::new());
                        }
                        ymids[k] = prev;
                    }
                    p += 1;
                }
            }
        }

        let mut seps = vec![if flip { (yoff, xoff) } else { (xoff, yoff) }];
        let ymid = ymids.get(self.lcs).cloned().unwrap_or_default();
        for n in 0..nchunks - 1 {
            let x1 = xoff + (numer + (xlim - xoff) * n) / nchunks;
            let y1 = (ymid.get(&n).copied().unwrap_or(0) + 1) as usize;
            seps.push(if flip { (y1, x1) } else { (x1, y1) });
        }
        seps.push(if flip { (ylim, xlim) } else { (xlim, ylim) });
        (self.lcs, seps)
    }

    fn compareseq(&mut self, mut xoff: usize, mut xlim: usize, mut yoff: usize, mut ylim: usize) {
        // Slide down the bottom initial diagonal.
        while xoff < xlim && yoff < ylim && loose_eq(&self.xv[xoff], &self.yv[yoff]) {
            xoff += 1;
            yoff += 1;
        }
        // Slide up the top initial diagonal.
        while xlim > xoff && ylim > yoff && loose_eq(&self.xv[xlim - 1], &self.yv[ylim - 1]) {
            xlim -= 1;
            ylim -= 1;
        }
        let mut lcs = 0;
        let mut seps = Vec::new();
        if xoff != xlim && yoff != ylim {
            // This is ad hoc but seems to work well.
            let nchunks = (xlim - xoff).min(ylim - yoff).min(7) + 1;
            let (l, s) = self.diag(xoff, xlim, yoff, ylim, nchunks);
            lcs = l;
            seps = s;
        }
        if lcs == 0 {
            // X and Y sequences have no common subsequence: mark all changed.
            while yoff < ylim {
                let at = self.yind[yoff];
                self.ychanged[at] = true;
                yoff += 1;
            }
            while xoff < xlim {
                let at = self.xind[xoff];
                self.xchanged[at] = true;
                xoff += 1;
            }
        } else {
            // Use the partitions to split this problem into subproblems.
            let mut pt1 = seps[0];
            for pt2 in seps.iter().skip(1).copied() {
                self.compareseq(pt1.0, pt2.0, pt1.1, pt2.1);
                pt1 = pt2;
            }
        }
    }
}

/// `_shiftBoundaries()`.
fn shift_boundaries(lines: &[String], changed: &mut [bool], other_changed: &[bool]) {
    let len = lines.len() as i64;
    let other_len = other_changed.len() as i64;
    let oc = |j: i64| j >= 0 && j < other_len && other_changed[j as usize];
    let mut i: i64 = 0;
    let mut j: i64 = 0;
    let at = |i: i64| i >= 0 && i < len;
    loop {
        // Scan forward to find the beginning of another run of changes. Also keep track of the corresponding point in the other file.
        while j < other_len && oc(j) {
            j += 1;
        }
        while i < len && !changed[i as usize] {
            i += 1;
            j += 1;
            while j < other_len && oc(j) {
                j += 1;
            }
        }
        if i == len {
            break;
        }
        let mut start = i;
        // Find the end of this run of changes.
        loop {
            i += 1;
            if !(i < len && changed[i as usize]) {
                break;
            }
        }
        let mut corresponding;
        loop {
            // Record the length of this run of changes, so that we can later determine whether the run has grown.
            let runlength = i - start;
            // Move the changed region back, so long as the previous unchanged line matches the last changed one.
            while start > 0 && loose_eq(&lines[(start - 1) as usize], &lines[(i - 1) as usize]) {
                start -= 1;
                changed[start as usize] = true;
                i -= 1;
                changed[i as usize] = false;
                while start > 0 && changed[(start - 1) as usize] {
                    start -= 1;
                }
                loop {
                    j -= 1;
                    if !oc(j) {
                        break;
                    }
                }
            }
            // Set CORRESPONDING to the end of the changed run, at the last point where it corresponds to a changed run in the other file.
            corresponding = if j < other_len { i } else { len };
            // Move the changed region forward, so long as the first changed line matches the following unchanged one.
            while i < len && loose_eq(&lines[start as usize], &lines[i as usize]) {
                changed[start as usize] = false;
                start += 1;
                changed[i as usize] = true;
                i += 1;
                while i < len && changed[i as usize] {
                    i += 1;
                }
                j += 1;
                if j < other_len && oc(j) {
                    corresponding = i;
                    while j < other_len && oc(j) {
                        j += 1;
                    }
                }
            }
            if runlength == i - start {
                break;
            }
        }
        // If possible, move the fully-merged run of changes back to a corresponding run in the other file.
        while corresponding < i {
            start -= 1;
            changed[start as usize] = true;
            i -= 1;
            changed[i as usize] = false;
            loop {
                j -= 1;
                if !oc(j) {
                    break;
                }
            }
        }
        let _ = at;
    }
}

/// `Text_Diff::__construct( 'auto', ... )`.
pub fn text_diff(from: &[String], to: &[String]) -> Vec<Op> {
    Native::diff(from, to)
}

/// `htmlspecialchars()`: `& < > " '`.
pub fn htmlspecialchars(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#039;"),
            other => out.push(other),
        }
    }
    out
}

/// The blocks `Text_Diff_Renderer::render()` builds: the changes with their context lines.
fn blocks(diffs: &[Op], nlead: usize, ntrail: usize) -> Vec<Vec<Op>> {
    let mut out: Vec<Vec<Op>> = Vec::new();
    let mut block: Option<Vec<Op>> = None;
    let mut context: Vec<String> = Vec::new();
    for (i, edit) in diffs.iter().enumerate() {
        if let Op::Copy(orig) = edit {
            // Do we have any diff blocks yet?
            if let Some(b) = block.as_mut() {
                // How many lines to keep as context from the copy block.
                let keep = if i == diffs.len() - 1 { ntrail } else { nlead + ntrail };
                if orig.len() <= keep {
                    // We have less lines in the block than we want for context => keep the whole copy block.
                    b.push(edit.clone());
                } else {
                    if ntrail > 0 {
                        // Create a new block with as many lines as we need for the trailing context.
                        b.push(Op::Copy(orig[..ntrail].to_vec()));
                    }
                    out.push(block.take().unwrap_or_default());
                }
            }
            // Keep the copy block as the context for the next block.
            context = orig.clone();
        } else {
            // Don't we have any diff blocks yet?
            if block.is_none() {
                // Extract context lines from the preceding copy block.
                let from = context.len().saturating_sub(nlead);
                let kept = context[from..].to_vec();
                let mut b = Vec::new();
                if !kept.is_empty() {
                    b.push(Op::Copy(kept));
                }
                block = Some(b);
            }
            if let Some(b) = block.as_mut() {
                b.push(edit.clone());
            }
        }
    }
    if let Some(b) = block {
        out.push(b);
    }
    out
}

/// The split level of the inline renderer.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Split {
    Lines,
    Words,
}

/// `WP_Text_Diff_Renderer_inline::_splitOnWords()`: split at every character that is not a word character.
fn split_on_words(text: &str, newline_escape: &str) -> Vec<String> {
    let text = text.replace('\0', "");
    let mut words = Vec::new();
    let mut current = String::new();
    for c in text.chars() {
        if c.is_alphanumeric() || c == '_' {
            current.push(c);
        } else {
            words.push(std::mem::take(&mut current));
            words.push(c.to_string());
        }
    }
    words.push(current);
    words.into_iter().map(|w| w.replace('\n', newline_escape)).collect()
}

fn inline_lines(lines: &[String], level: Split) -> String {
    if level == Split::Lines { format!("{}\n", lines.join("\n")) } else { lines.concat() }
}

fn inline_added(lines: &[String], level: Split) -> String {
    let mut lines: Vec<String> = lines.iter().map(|l| htmlspecialchars(l)).collect();
    if let Some(first) = lines.first_mut() {
        *first = format!("<ins>{first}");
    }
    if let Some(last) = lines.last_mut() {
        last.push_str("</ins>");
    }
    inline_lines(&lines, level)
}

fn inline_deleted(lines: &[String], level: Split) -> String {
    let mut lines: Vec<String> = lines.iter().map(|l| htmlspecialchars(l)).collect();
    if let Some(first) = lines.first_mut() {
        *first = format!("<del>{first}");
    }
    if let Some(last) = lines.last_mut() {
        last.push_str("</del>");
    }
    inline_lines(&lines, level)
}

fn inline_changed(orig: &[String], fin: &[String], level: Split) -> String {
    if level == Split::Words {
        let (mut orig, mut fin) = (orig.to_vec(), fin.to_vec());
        let mut prefix = String::new();
        while orig[0].starts_with(' ') && fin[0].starts_with(' ') {
            prefix.push(' ');
            orig[0] = orig[0][1..].to_owned();
            fin[0] = fin[0][1..].to_owned();
        }
        return format!("{prefix}{}{}", inline_deleted(&orig, level), inline_added(&fin, level));
    }
    let text1 = orig.join("\n");
    let text2 = fin.join("\n");
    let nl = "\0";
    let diff = Native::diff(&split_on_words(&text1, nl), &split_on_words(&text2, nl));
    format!("{}\n", inline_render(&diff, Split::Words).replace(nl, "\n"))
}

/// `WP_Text_Diff_Renderer_inline::render()`.
fn inline_render(diff: &[Op], level: Split) -> String {
    let mut out = String::new();
    for block in blocks(diff, 10000, 10000) {
        for edit in &block {
            match edit {
                Op::Copy(lines) => {
                    let encoded: Vec<String> = lines.iter().map(|l| htmlspecialchars(l)).collect();
                    out.push_str(&inline_lines(&encoded, level));
                }
                Op::Add(lines) => out.push_str(&inline_added(lines, level)),
                Op::Delete(lines) => out.push_str(&inline_deleted(lines, level)),
                Op::Change(orig, fin) => out.push_str(&inline_changed(orig, fin, level)),
            }
        }
    }
    out
}

/// The texts the table renderer prints for screen readers.
#[derive(Debug, Clone)]
pub struct TableLabels {
    pub added: String,
    pub deleted: String,
    pub unchanged: String,
}

impl Default for TableLabels {
    fn default() -> Self {
        Self { added: "Added:".into(), deleted: "Deleted:".into(), unchanged: "Unchanged:".into() }
    }
}

struct Table<'a> {
    split_view: bool,
    labels: &'a TableLabels,
}

impl Table<'_> {
    fn added_line(&self, line: &str) -> String {
        format!("<td class='diff-addedline'><span aria-hidden='true' class='dashicons dashicons-plus'></span><span class='screen-reader-text'>{} </span>{line}</td>", self.labels.added)
    }

    fn deleted_line(&self, line: &str) -> String {
        format!("<td class='diff-deletedline'><span aria-hidden='true' class='dashicons dashicons-minus'></span><span class='screen-reader-text'>{} </span>{line}</td>", self.labels.deleted)
    }

    fn context_line(&self, line: &str) -> String {
        format!("<td class='diff-context'><span class='screen-reader-text'>{} </span>{line}</td>", self.labels.unchanged)
    }

    fn added(&self, lines: &[String], encode: bool) -> String {
        let mut r = String::new();
        for line in lines {
            let line = if encode { htmlspecialchars(line) } else { line.clone() };
            if self.split_view {
                r.push_str(&format!("<tr><td>&nbsp;</td>{}</tr>\n", self.added_line(&line)));
            } else {
                r.push_str(&format!("<tr>{}</tr>\n", self.added_line(&line)));
            }
        }
        r
    }

    fn deleted(&self, lines: &[String], encode: bool) -> String {
        let mut r = String::new();
        for line in lines {
            let line = if encode { htmlspecialchars(line) } else { line.clone() };
            if self.split_view {
                r.push_str(&format!("<tr>{}<td>&nbsp;</td></tr>\n", self.deleted_line(&line)));
            } else {
                r.push_str(&format!("<tr>{}</tr>\n", self.deleted_line(&line)));
            }
        }
        r
    }

    fn context(&self, lines: &[String]) -> String {
        let mut r = String::new();
        for line in lines {
            let line = htmlspecialchars(line);
            if self.split_view {
                r.push_str(&format!("<tr>{}{}</tr>\n", self.context_line(&line), self.context_line(&line)));
            } else {
                r.push_str(&format!("<tr>{}</tr>\n", self.context_line(&line)));
            }
        }
        r
    }

    fn changed(&self, orig: &[String], fin: &[String]) -> String {
        let (orig_matches, _final_matches, orig_rows, final_rows) = interleave_changed_lines(orig, fin);
        let mut orig_diffs: HashMap<usize, String> = HashMap::new();
        let mut final_diffs: HashMap<usize, String> = HashMap::new();
        for (o, f) in &orig_matches {
            let Match::To(f) = f else { continue };
            let single = text_diff(&[orig[*o].clone()], &[fin[*f].clone()]);
            let diff = inline_render(&single, Split::Lines);
            let pieces = ins_del_pieces(&diff);
            if !pieces.is_empty() {
                let stripped_matches = strip_ins_del(&pieces.join(" ")).len();
                let stripped_diff = strip_ins_del(&diff).len() * 2 - stripped_matches;
                let ratio = stripped_matches as f64 / stripped_diff as f64;
                if ratio > 0.6 {
                    continue; // Too different. Don't save diffs.
                }
            }
            orig_diffs.insert(*o, remove_tag_content(&diff, "ins"));
            final_diffs.insert(*f, remove_tag_content(&diff, "del"));
        }
        let mut r = String::new();
        for row in 0..orig_rows.len() {
            let (orow, frow) = (orig_rows[row], final_rows[row]);
            if orow < 0 && frow < 0 {
                continue;
            }
            let orig_line = if orow >= 0 && orig_diffs.contains_key(&(orow as usize)) {
                orig_diffs[&(orow as usize)].clone()
            } else if orow >= 0 && (orow as usize) < orig.len() {
                htmlspecialchars(&orig[orow as usize])
            } else {
                String::new()
            };
            let final_line = if frow >= 0 && final_diffs.contains_key(&(frow as usize)) {
                final_diffs[&(frow as usize)].clone()
            } else if frow >= 0 && (frow as usize) < fin.len() {
                htmlspecialchars(&fin[frow as usize])
            } else {
                String::new()
            };
            if orow < 0 {
                r.push_str(&self.added(&[final_line], false));
            } else if frow < 0 {
                r.push_str(&self.deleted(&[orig_line], false));
            } else if self.split_view {
                r.push_str(&format!("<tr>{}{}</tr>\n", self.deleted_line(&orig_line), self.added_line(&final_line)));
            } else {
                r.push_str(&format!("<tr>{}</tr><tr>{}</tr>\n", self.deleted_line(&orig_line), self.added_line(&final_line)));
            }
        }
        r
    }
}

/// `preg_match_all( '!(<ins>.*?</ins>|<del>.*?</del>)!', ... )`: `.` does not match a newline.
fn ins_del_pieces(diff: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = diff;
    loop {
        let ins = rest.find("<ins>");
        let del = rest.find("<del>");
        let (start, close) = match (ins, del) {
            (Some(i), Some(d)) if i < d => (i, "</ins>"),
            (Some(i), None) => (i, "</ins>"),
            (_, Some(d)) => (d, "</del>"),
            (None, None) => break,
        };
        let body = &rest[start..];
        match body.find(close) {
            Some(end) if !body[..end].contains('\n') => {
                out.push(body[..end + close.len()].to_owned());
                rest = &body[end + close.len()..];
            }
            _ => rest = &body[1..],
        }
    }
    out
}

fn strip_ins_del(text: &str) -> String {
    text.replace("<ins>", "").replace("</ins>", "").replace("<del>", "").replace("</del>", "")
}

/// `preg_replace( '|<ins>.*?</ins>|', '', $diff )`.
fn remove_tag_content(diff: &str, tag: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut out = String::new();
    let mut rest = diff;
    while let Some(start) = rest.find(&open) {
        let after = &rest[start..];
        match after.find(&close) {
            Some(end) if !after[..end].contains('\n') => {
                out.push_str(&rest[..start]);
                rest = &after[end + close.len()..];
            }
            _ => {
                out.push_str(&rest[..start + 1]);
                rest = &rest[start + 1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// A pairing of a changed line: the index of the line on the other side, or the `x` of a line that lost its partner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Match {
    To(usize),
    X,
}

/// `compute_string_distance()`.
fn string_distance(a: &str, b: &str) -> f64 {
    let mut ca = [0i64; 256];
    let mut cb = [0i64; 256];
    for byte in a.bytes() {
        ca[byte as usize] += 1;
    }
    for byte in b.bytes() {
        cb[byte as usize] += 1;
    }
    let difference: i64 = ca.iter().zip(cb.iter()).map(|(x, y)| (x - y).abs()).sum();
    if a.is_empty() || a == "0" { difference as f64 } else { difference as f64 / a.len() as f64 }
}

/// `interleave_changed_lines()`: `(orig_matches, final_matches, orig_rows, final_rows)`.
fn interleave_changed_lines(orig: &[String], fin: &[String]) -> (Vec<(usize, Match)>, Vec<(usize, Match)>, Vec<i64>, Vec<i64>) {
    let mut matches: Vec<((usize, usize), f64)> = Vec::new();
    for o in 0..orig.len() {
        for f in 0..fin.len() {
            matches.push(((o, f), string_distance(&orig[o], &fin[f])));
        }
    }
    // `asort()` is stable.
    matches.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut orig_matches: BTreeMap<usize, Match> = BTreeMap::new();
    let mut final_matches: BTreeMap<usize, Match> = BTreeMap::new();
    for ((o, f), _) in matches {
        let (has_o, has_f) = (orig_matches.contains_key(&o), final_matches.contains_key(&f));
        if has_o && has_f {
            continue;
        }
        if !has_o && !has_f {
            orig_matches.insert(o, Match::To(f));
            final_matches.insert(f, Match::To(o));
            continue;
        }
        if has_o {
            final_matches.insert(f, Match::X);
        } else {
            orig_matches.insert(o, Match::X);
        }
    }
    let mut orig_rows: Vec<i64> = orig_matches.keys().map(|k| *k as i64).collect();
    let orig_rows_copy = orig_rows.clone();
    let mut final_rows: Vec<i64> = final_matches.keys().map(|k| *k as i64).collect();
    let splice = |rows: &mut Vec<i64>, at: usize, items: Vec<i64>| {
        let at = at.min(rows.len());
        rows.splice(at..at, items);
    };
    for orig_row in orig_rows_copy {
        let final_pos = match orig_matches[&(orig_row as usize)] {
            Match::To(f) => final_rows.iter().position(|r| *r == f as i64),
            Match::X => None,
        };
        let orig_pos = orig_rows.iter().position(|r| *r == orig_row).unwrap_or(0);
        match final_pos {
            None => splice(&mut final_rows, orig_pos, vec![-1]),
            Some(fp) if fp < orig_pos => {
                let n = fp as i64 - orig_pos as i64;
                splice(&mut final_rows, orig_pos, (n..=-1).rev().collect());
            }
            Some(fp) if fp > orig_pos => {
                let n = orig_pos as i64 - fp as i64;
                splice(&mut orig_rows, orig_pos, (n..=-1).rev().collect());
            }
            Some(_) => {}
        }
    }
    let mut diff_count = orig_rows.len() as i64 - final_rows.len() as i64;
    if diff_count < 0 {
        while diff_count < 0 {
            orig_rows.push(diff_count);
            diff_count += 1;
        }
    } else if diff_count > 0 {
        diff_count = -diff_count;
        while diff_count < 0 {
            final_rows.push(diff_count);
            diff_count += 1;
        }
    }
    (orig_matches.into_iter().collect(), final_matches.into_iter().collect(), orig_rows, final_rows)
}

/// What `wp_text_diff()` is asked for.
#[derive(Debug, Clone)]
pub struct DiffArgs {
    pub title: String,
    pub title_left: String,
    pub title_right: String,
    pub show_split_view: bool,
    pub labels: TableLabels,
}

impl Default for DiffArgs {
    fn default() -> Self {
        Self { title: String::new(), title_left: String::new(), title_right: String::new(), show_split_view: true, labels: TableLabels::default() }
    }
}

/// `normalize_whitespace()`: trimmed, runs of newlines and of blanks collapsed.
pub fn normalize_whitespace(text: &str) -> String {
    let trimmed = text.trim_matches(|c: char| matches!(c, ' ' | '\t' | '\n' | '\r' | '\0' | '\u{b}')).replace('\r', "\n");
    let mut out = String::with_capacity(trimmed.len());
    let mut last_newline = false;
    let mut last_blank = false;
    for c in trimmed.chars() {
        match c {
            '\n' if last_newline => {}
            '\n' => {
                out.push('\n');
                last_newline = true;
                last_blank = false;
            }
            ' ' | '\t' if last_blank => {}
            ' ' | '\t' => {
                out.push(' ');
                last_blank = true;
                last_newline = false;
            }
            other => {
                out.push(other);
                last_newline = false;
                last_blank = false;
            }
        }
    }
    out
}

/// `wp_text_diff()`: an HTML table of the differences, or an empty string when there are none.
pub fn wp_text_diff(left: &str, right: &str, args: &DiffArgs) -> String {
    let left = normalize_whitespace(left);
    let right = normalize_whitespace(right);
    let left_lines: Vec<String> = left.split('\n').map(str::to_owned).collect();
    let right_lines: Vec<String> = right.split('\n').map(str::to_owned).collect();
    let edits = Native::diff(&left_lines, &right_lines);
    let table = Table { split_view: args.show_split_view, labels: &args.labels };
    let mut diff = String::new();
    for block in blocks(&edits, 10000, 10000) {
        for edit in &block {
            match edit {
                Op::Copy(lines) => diff.push_str(&table.context(lines)),
                Op::Add(lines) => diff.push_str(&table.added(lines, true)),
                Op::Delete(lines) => diff.push_str(&table.deleted(lines, true)),
                Op::Change(orig, fin) => diff.push_str(&table.changed(orig, fin)),
            }
        }
    }
    if diff.is_empty() {
        return String::new();
    }
    let split_class = if args.show_split_view { " is-split-view" } else { "" };
    let mut r = format!("<table class='diff{split_class}'>\n");
    if !args.title.is_empty() {
        r.push_str(&format!("<caption class='diff-title'>{}</caption>\n", args.title));
    }
    let titled = !args.title_left.is_empty() || !args.title_right.is_empty();
    if titled {
        r.push_str("<thead>");
        let left_tag = if args.title_left.is_empty() { "td" } else { "th" };
        let right_tag = if args.title_right.is_empty() { "td" } else { "th" };
        r.push_str("<tr class='diff-sub-title'>\n");
        r.push_str(&format!("\t<{left_tag}>{}</{left_tag}>\n", args.title_left));
        if args.show_split_view {
            r.push_str(&format!("\t<{right_tag}>{}</{right_tag}>\n", args.title_right));
        }
        r.push_str("</tr>\n");
        r.push_str("</thead>\n");
    }
    r.push_str(&format!("<tbody>\n{diff}\n</tbody>\n"));
    r.push_str("</table>");
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<String> {
        text.split('\n').map(str::to_owned).collect()
    }

    #[test]
    fn identical_texts_have_no_diff() {
        assert_eq!(wp_text_diff("a\nb", "a\nb", &DiffArgs::default()), "");
    }

    #[test]
    fn engine_finds_changes() {
        let edits = text_diff(&lines("a\nb\nc"), &lines("a\nx\nc"));
        assert_eq!(edits, vec![Op::Copy(vec!["a".into()]), Op::Change(vec!["b".into()], vec!["x".into()]), Op::Copy(vec!["c".into()])]);
    }

    #[test]
    fn added_and_deleted_lines() {
        let edits = text_diff(&lines("a\nb"), &lines("a\nb\nc"));
        assert_eq!(edits, vec![Op::Copy(vec!["a".into(), "b".into()]), Op::Add(vec!["c".into()])]);
        let edits = text_diff(&lines("a\nb\nc"), &lines("a\nc"));
        assert_eq!(edits, vec![Op::Copy(vec!["a".into()]), Op::Delete(vec!["b".into()]), Op::Copy(vec!["c".into()])]);
    }

    #[test]
    fn words_are_split_at_non_word_characters() {
        assert_eq!(split_on_words("a, b", "\n"), vec!["a", ",", "", " ", "b"]);
    }
}
