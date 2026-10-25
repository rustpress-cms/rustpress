//! Template tags of `general-template.php` and `category-template.php` that blocks, widgets and
//! classic themes share: `get_terms()`, `wp_list_categories()`, `wp_dropdown_categories()`,
//! `wp_get_archives()`, `wp_tag_cloud()` and `get_calendar()`.

use std::cmp::Ordering;
use std::collections::HashMap;

use rustpress_core::formatting::{esc_attr, esc_html, esc_url, strip_tags};
use rustpress_core::i18n::sprintf;
use rustpress_core::Queried;
use rustpress_db::{ArchiveKind, Term};

use crate::Result;
use crate::content::the_title_for;
use crate::env::Env;

// ------------------------------------------------------------------ PHP helpers

/// A float as PHP prints it (`precision` = 14).
pub fn php_float(f: f64) -> String {
    if f == 0.0 {
        return "0".to_owned();
    }
    let formatted = format!("{f:.13e}");
    let (mantissa, exp) = formatted.split_once('e').unwrap_or((&formatted, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let negative = mantissa.starts_with('-');
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let digits = digits.trim_end_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    let sign = if negative { "-" } else { "" };
    if !(-5..15).contains(&exp) {
        let (first, rest) = digits.split_at(1);
        let rest = if rest.is_empty() { "0" } else { rest };
        return format!("{sign}{first}.{rest}E{}{}", if exp < 0 { "-" } else { "+" }, exp.abs());
    }
    if exp < 0 {
        return format!("{sign}0.{}{digits}", "0".repeat((-exp - 1) as usize));
    }
    let int_len = exp as usize + 1;
    if digits.len() <= int_len {
        format!("{sign}{digits}{}", "0".repeat(int_len - digits.len()))
    } else {
        format!("{sign}{}.{}", &digits[..int_len], &digits[int_len..])
    }
}

/// `strnatcasecmp()`.
pub fn strnatcasecmp(a: &str, b: &str) -> Ordering {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let (mut i, mut j) = (0, 0);
    loop {
        while i < a.len() && a[i].is_ascii_whitespace() {
            i += 1;
        }
        while j < b.len() && b[j].is_ascii_whitespace() {
            j += 1;
        }
        match (a.get(i), b.get(j)) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let end = |s: &[u8], mut k: usize| {
                    while k < s.len() && s[k].is_ascii_digit() {
                        k += 1;
                    }
                    k
                };
                let (ei, ej) = (end(a, i), end(b, j));
                let (na, nb) = (std::str::from_utf8(&a[i..ei]).unwrap_or("").trim_start_matches('0'), std::str::from_utf8(&b[j..ej]).unwrap_or("").trim_start_matches('0'));
                let ord = na.len().cmp(&nb.len()).then_with(|| na.cmp(nb));
                if ord != Ordering::Equal {
                    return ord;
                }
                (i, j) = (ei, ej);
            }
            (Some(x), Some(y)) => {
                let ord = x.to_ascii_lowercase().cmp(&y.to_ascii_lowercase());
                if ord != Ordering::Equal {
                    return ord;
                }
                i += 1;
                j += 1;
            }
        }
    }
}

/// `floatval()`: the leading number.
pub fn floatval(s: &str) -> f64 {
    let s = s.trim_start();
    let end = s.char_indices().find(|(i, c)| !(c.is_ascii_digit() || *c == '.' || (*i == 0 && (*c == '-' || *c == '+')))).map_or(s.len(), |(i, _)| i);
    s[..end].parse().unwrap_or(0.0)
}

fn now_timestamp() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

/// `wp_json_encode( array( $dropdown_id, home_url() ), JSON_HEX_TAG | JSON_UNESCAPED_SLASHES )`.
fn dropdown_exports(id: &str, home: &str) -> String {
    let encode = |s: &str| serde_json::to_string(s).unwrap_or_default().replace('<', "\\u003C").replace('>', "\\u003E");
    format!("[{},{}]", encode(id), encode(home))
}

/// `wp_get_inline_script_tag()` with the dropdown script of a block.
pub fn dropdown_script(template: &str, id: &str, home: &str, source: &str) -> String {
    let body = template.replace("__EXPORTS__", &dropdown_exports(id, home));
    format!("<script>\n{}\n//# sourceURL={source}\n</script>\n", body.trim_matches(['\n', '\r', ' ']))
}

pub const ARCHIVES_DROPDOWN_SCRIPT: &str = include_str!("scripts/archives-dropdown.js");
pub const CATEGORIES_DROPDOWN_SCRIPT: &str = include_str!("scripts/categories-dropdown.js");

// -------------------------------------------------------------------- get_terms

/// The arguments of `get_terms()` that matter for lists and clouds.
#[derive(Debug, Clone)]
pub struct TermsArgs {
    pub taxonomy: String,
    pub hide_empty: bool,
    pub hierarchical: bool,
    /// `name`, `id`, `slug` or `count`.
    pub orderby: String,
    pub descending: bool,
    pub parent: Option<u64>,
    pub exclude: Vec<u64>,
    pub exclude_tree: Vec<u64>,
    pub include: Vec<u64>,
    pub child_of: u64,
    /// `0`: all.
    pub number: u64,
    pub pad_counts: bool,
}

impl TermsArgs {
    pub fn new(taxonomy: &str) -> Self {
        Self {
            taxonomy: taxonomy.to_owned(),
            hide_empty: true,
            hierarchical: true,
            orderby: "name".into(),
            descending: false,
            parent: None,
            exclude: Vec::new(),
            exclude_tree: Vec::new(),
            include: Vec::new(),
            child_of: 0,
            number: 0,
            pad_counts: false,
        }
    }
}

fn descendants(all: &[Term], root: u64) -> Vec<u64> {
    let mut out = Vec::new();
    let mut queue = vec![root];
    while let Some(id) = queue.pop() {
        for term in all.iter().filter(|t| t.parent == id) {
            if !out.contains(&term.term_id) && term.term_id != root {
                out.push(term.term_id);
                queue.push(term.term_id);
            }
        }
    }
    out
}

/// `get_terms()` with `fields => all`.
pub async fn get_terms(env: &Env<'_>, args: &TermsArgs) -> Result<Vec<Term>> {
    let site = env.site;
    let all = site.db.terms(&args.taxonomy).await?;
    let mut terms: Vec<Term> = all.clone();
    if !args.include.is_empty() {
        terms.retain(|t| args.include.contains(&t.term_id));
    }
    terms.retain(|t| !args.exclude.contains(&t.term_id));
    if !args.exclude_tree.is_empty() {
        let mut hidden = args.exclude_tree.clone();
        for id in &args.exclude_tree {
            hidden.extend(descendants(&all, *id));
        }
        terms.retain(|t| !hidden.contains(&t.term_id));
    }
    if let Some(parent) = args.parent {
        terms.retain(|t| t.parent == parent);
    }
    if args.child_of != 0 {
        let below = descendants(&all, args.child_of);
        terms.retain(|t| below.contains(&t.term_id));
    }
    if args.hide_empty && !args.hierarchical {
        terms.retain(|t| t.count > 0);
    }
    match args.orderby.as_str() {
        "id" | "term_id" => terms.sort_by_key(|t| t.term_id),
        "slug" => terms.sort_by(|a, b| a.slug.cmp(&b.slug).then(a.term_id.cmp(&b.term_id))),
        "count" => terms.sort_by(|a, b| a.count.cmp(&b.count).then(a.term_id.cmp(&b.term_id))),
        "include" => terms.sort_by_key(|t| args.include.iter().position(|id| *id == t.term_id)),
        // The database already returns them by name.
        _ => {}
    }
    if args.descending {
        terms.reverse();
    }
    if args.pad_counts {
        pad_term_counts(env, &mut terms).await?;
    }
    if args.hierarchical && args.hide_empty {
        // Empty terms stay if a descendant has posts (with the counts before padding).
        let original: HashMap<u64, i64> = all.iter().map(|t| (t.term_id, t.count)).collect();
        terms.retain(|t| original.get(&t.term_id).copied().unwrap_or(0) > 0 || descendants(&all, t.term_id).iter().any(|id| original.get(id).copied().unwrap_or(0) > 0));
    }
    if args.number > 0 {
        terms.truncate(args.number as usize);
    }
    Ok(terms)
}

/// `_pad_term_counts()`: a term counts the posts of its descendants as well.
async fn pad_term_counts(env: &Env<'_>, terms: &mut [Term]) -> Result<()> {
    if terms.iter().all(|t| t.parent == 0) {
        return Ok(());
    }
    let ids: Vec<u64> = terms.iter().map(|t| t.term_taxonomy_id).collect();
    let rows = env.site.db.term_objects(&ids, &["post"]).await?;
    let by_tt: HashMap<u64, u64> = terms.iter().map(|t| (t.term_taxonomy_id, t.term_id)).collect();
    let parent_of: HashMap<u64, u64> = terms.iter().map(|t| (t.term_id, t.parent)).collect();
    let mut items: HashMap<u64, std::collections::BTreeSet<u64>> = HashMap::new();
    for (object, tt) in rows {
        if let Some(term_id) = by_tt.get(&tt) {
            items.entry(*term_id).or_default().insert(object);
        }
    }
    let direct: Vec<(u64, Vec<u64>)> = items.iter().map(|(k, v)| (*k, v.iter().copied().collect())).collect();
    for (term_id, objects) in direct {
        let mut child = term_id;
        let mut seen = vec![];
        while let Some(parent) = parent_of.get(&child).copied().filter(|p| *p != 0) {
            seen.push(child);
            items.entry(parent).or_default().extend(objects.iter().copied());
            child = parent;
            if seen.contains(&parent) {
                break;
            }
        }
    }
    for term in terms.iter_mut() {
        if let Some(set) = items.get(&term.term_id) {
            term.count = set.len() as i64;
        }
    }
    Ok(())
}

// ----------------------------------------------------------------------- walker

enum Event<'a> {
    Start(&'a Term, usize),
    StartLevel(usize),
    EndLevel(usize),
    End,
}

type Children<'a> = Vec<(u64, Vec<&'a Term>)>;

fn display_element<'a>(element: &'a Term, children: &mut Children<'a>, max_depth: i32, depth: usize, emit: &mut dyn FnMut(Event<'a>)) {
    emit(Event::Start(element, depth));
    let mut new_level = false;
    if (max_depth == 0 || max_depth > depth as i32 + 1)
        && let Some(position) = children.iter().position(|(k, _)| *k == element.term_id)
    {
        let kids = children[position].1.clone();
        for child in kids {
            if !new_level {
                new_level = true;
                emit(Event::StartLevel(depth));
            }
            display_element(child, children, max_depth, depth + 1, emit);
        }
        if let Some(position) = children.iter().position(|(k, _)| *k == element.term_id) {
            children.remove(position);
        }
    }
    if new_level {
        emit(Event::EndLevel(depth));
    }
    emit(Event::End);
}

/// `Walker::walk()` for terms.
fn walk<'a>(elements: &'a [Term], max_depth: i32, emit: &mut dyn FnMut(Event<'a>)) {
    if elements.is_empty() || max_depth < -1 {
        return;
    }
    if max_depth == -1 {
        for e in elements {
            display_element(e, &mut Vec::new(), 1, 0, emit);
        }
        return;
    }
    let mut top: Vec<&Term> = Vec::new();
    let mut children: Children<'a> = Vec::new();
    let push_child = |children: &mut Children<'a>, e: &'a Term| match children.iter_mut().find(|(k, _)| *k == e.parent) {
        Some((_, list)) => list.push(e),
        None => children.push((e.parent, vec![e])),
    };
    for e in elements {
        if e.parent == 0 {
            top.push(e);
        } else {
            push_child(&mut children, e);
        }
    }
    if top.is_empty() {
        // None is top level: the first element's siblings are.
        let root = elements[0].parent;
        children.clear();
        for e in elements {
            if e.parent == root {
                top.push(e);
            } else {
                push_child(&mut children, e);
            }
        }
    }
    for e in top {
        display_element(e, &mut children, max_depth, 0, emit);
    }
    // Orphans are shown regardless.
    if max_depth == 0 && !children.is_empty() {
        for (_, orphans) in children {
            for orphan in orphans {
                display_element(orphan, &mut Vec::new(), 1, 0, emit);
            }
        }
    }
}

// ------------------------------------------------------------ wp_list_categories

/// The arguments of `wp_list_categories()`.
#[derive(Debug, Clone)]
pub struct ListCategories {
    pub terms: TermsArgs,
    pub show_count: bool,
    pub depth: i32,
    pub title_li: String,
    pub show_option_none: String,
    pub show_option_all: String,
    /// `list` or anything else for plain links.
    pub style: String,
    pub separator: String,
    pub current_category: u64,
    pub class: Option<String>,
    pub hide_title_if_empty: bool,
    pub use_desc_for_title: bool,
}

impl ListCategories {
    pub fn new(env: &Env<'_>, taxonomy: &str) -> Self {
        Self {
            terms: TermsArgs::new(taxonomy),
            show_count: false,
            depth: 0,
            title_li: env.site.tr("Categories"),
            show_option_none: env.site.tr("No categories"),
            show_option_all: String::new(),
            style: "list".into(),
            separator: "<br />".into(),
            current_category: 0,
            class: None,
            hide_title_if_empty: false,
            use_desc_for_title: false,
        }
    }
}

/// The term the main query shows, if it is in `taxonomy`.
fn queried_term_in(env: &Env<'_>, taxonomy: &str) -> Option<Term> {
    let q = env.query;
    match &q.queried {
        Queried::Term(term) if (q.flags.is_category || q.flags.is_tax || q.flags.is_tag) && term.taxonomy == taxonomy => Some(term.clone()),
        _ => None,
    }
}

/// `wp_list_categories()`.
pub async fn wp_list_categories(env: &Env<'_>, args: &ListCategories) -> Result<String> {
    let site = env.site;
    let mut args = args.clone();
    if args.terms.hierarchical && args.show_count {
        args.terms.pad_counts = true;
    }
    if args.terms.hierarchical {
        let mut tree = args.terms.exclude_tree.clone();
        tree.extend(args.terms.exclude.iter().copied());
        args.terms.exclude_tree = tree;
        args.terms.exclude.clear();
    }
    let class = args.class.clone().unwrap_or_else(|| if args.terms.taxonomy == "category" { "categories".to_owned() } else { args.terms.taxonomy.clone() });
    let categories = get_terms(env, &args.terms).await?;
    let list = args.style == "list";
    let mut out = String::new();
    let wrap = !args.title_li.is_empty() && list && (!categories.is_empty() || !args.hide_title_if_empty);
    if wrap {
        out.push_str(&format!("<li class=\"{}\">{}<ul>", esc_attr(&class), args.title_li));
    }
    if categories.is_empty() {
        if !args.show_option_none.is_empty() {
            if list {
                out.push_str(&format!("<li class=\"cat-item-none\">{}</li>", args.show_option_none));
            } else {
                out.push_str(&args.show_option_none);
            }
        }
    } else {
        if !args.show_option_all.is_empty() {
            let posts_page = if site.options.string("show_on_front", "posts") == "page" && site.settings.page_for_posts != 0 {
                match site.db.post(site.settings.page_for_posts).await? {
                    Some(page) => site.permalink(&page).await?,
                    None => site.home_url("/"),
                }
            } else {
                site.home_url("/")
            };
            let posts_page = esc_url(&posts_page);
            if list {
                out.push_str(&format!("<li class='cat-item-all'><a href='{posts_page}'>{}</a></li>", args.show_option_all));
            } else {
                out.push_str(&format!("<a href='{posts_page}'>{}</a>", args.show_option_all));
            }
        }
        if args.current_category == 0
            && let Some(term) = queried_term_in(env, &args.terms.taxonomy)
        {
            args.current_category = term.term_id;
        }
        let current = if args.current_category != 0 { site.db.term_by_id(Some(&args.terms.taxonomy), args.current_category).await? } else { None };
        let mut ancestors = Vec::new();
        if let Some(term) = &current {
            let mut parent = term.parent;
            while parent != 0 && !ancestors.contains(&parent) {
                ancestors.push(parent);
                parent = site.db.term_by_id(Some(&args.terms.taxonomy), parent).await?.map_or(0, |t| t.parent);
            }
        }
        let mut links = HashMap::new();
        for term in &categories {
            links.insert(term.term_id, site.term_link(term).await?);
        }
        let depth = if args.terms.hierarchical { args.depth } else { -1 };
        let mut body = String::new();
        walk(&categories, depth, &mut |event| match event {
            Event::StartLevel(depth) => {
                if list {
                    body.push_str(&format!("{}<ul class='children'>\n", "\t".repeat(depth)));
                }
            }
            Event::EndLevel(depth) => {
                if list {
                    body.push_str(&format!("{}</ul>\n", "\t".repeat(depth)));
                }
            }
            Event::End => {
                if list {
                    body.push_str("</li>\n");
                }
            }
            Event::Start(term, _) => {
                let name = esc_attr(&site.term_name(term));
                if name.is_empty() {
                    return;
                }
                let mut attributes = format!(" href=\"{}\"", esc_url(&links[&term.term_id]));
                if args.use_desc_for_title && !term.description.is_empty() {
                    attributes.push_str(&format!(" title=\"{}\"", esc_attr(&strip_tags(&term.description))));
                }
                let mut link = format!("<a{attributes}>{name}</a>");
                if args.show_count {
                    link.push_str(&format!(" ({})", site.number_i18n(term.count.max(0) as u64)));
                }
                if list {
                    let mut classes = vec!["cat-item".to_owned(), format!("cat-item-{}", term.term_id)];
                    if let Some(current) = &current {
                        if term.term_id == current.term_id {
                            classes.push("current-cat".into());
                            link = link.replace("<a", "<a aria-current=\"page\"");
                        } else if term.term_id == current.parent {
                            classes.push("current-cat-parent".into());
                        }
                        if ancestors.contains(&term.term_id) {
                            classes.push("current-cat-ancestor".into());
                        }
                    }
                    body.push_str(&format!("\t<li class=\"{}\">{link}\n", esc_attr(&classes.join(" "))));
                } else {
                    body.push_str(&format!("\t{link}{}\n", args.separator));
                }
            }
        });
        out.push_str(&body);
    }
    if wrap {
        out.push_str("</ul></li>");
    }
    Ok(out)
}

// ------------------------------------------------------------ wp_dropdown_categories

/// The arguments of `wp_dropdown_categories()` the blocks and widgets pass.
#[derive(Debug, Clone)]
pub struct DropdownCategories {
    pub terms: TermsArgs,
    pub show_count: bool,
    pub depth: i32,
    pub show_option_none: String,
    pub show_option_all: String,
    pub option_none_value: String,
    pub name: String,
    pub id: String,
    pub class: String,
    pub value_field_slug: bool,
    pub selected: String,
    pub hide_if_empty: bool,
    pub required: bool,
}

/// `wp_dropdown_categories()`.
pub async fn wp_dropdown_categories(env: &Env<'_>, args: &DropdownCategories) -> Result<String> {
    let site = env.site;
    let mut args = args.clone();
    if args.show_count && args.terms.hierarchical {
        args.terms.pad_counts = true;
    }
    let categories = get_terms(env, &args.terms).await?;
    let name = esc_attr(&args.name);
    let id = if args.id.is_empty() { name.clone() } else { esc_attr(&args.id) };
    let class = esc_attr(&args.class);
    let required = if args.required { "required" } else { "" };
    let mut out = String::new();
    if !args.hide_if_empty || !categories.is_empty() {
        out.push_str(&format!("<select {required} name='{name}' id='{id}' class='{class}'>\n"));
    }
    if categories.is_empty() && !args.hide_if_empty && !args.show_option_none.is_empty() {
        out.push_str(&format!("\t<option value='{}' selected='selected'>{}</option>\n", esc_attr(&args.option_none_value), args.show_option_none));
    }
    if !categories.is_empty() {
        if !args.show_option_all.is_empty() {
            let selected = if args.selected == "0" { " selected='selected'" } else { "" };
            out.push_str(&format!("\t<option value='0'{selected}>{}</option>\n", args.show_option_all));
        }
        if !args.show_option_none.is_empty() {
            let selected = if args.option_none_value == args.selected { " selected='selected'" } else { "" };
            out.push_str(&format!("\t<option value='{}'{selected}>{}</option>\n", esc_attr(&args.option_none_value), args.show_option_none));
        }
        let depth = if args.terms.hierarchical { args.depth } else { -1 };
        walk(&categories, depth, &mut |event| {
            if let Event::Start(term, depth) = event {
                let value = if args.value_field_slug { term.slug.clone() } else { term.term_id.to_string() };
                let mut option = format!("\t<option class=\"level-{depth}\" value=\"{}\"", esc_attr(&value));
                if value == args.selected {
                    option.push_str(" selected=\"selected\"");
                }
                option.push('>');
                option.push_str(&"&nbsp;".repeat(depth * 3));
                option.push_str(&site.term_name(term));
                if args.show_count {
                    option.push_str(&format!("&nbsp;&nbsp;({})", site.number_i18n(term.count.max(0) as u64)));
                }
                option.push_str("</option>\n");
                out.push_str(&option);
            }
        });
    }
    if !args.hide_if_empty || !categories.is_empty() {
        out.push_str("</select>\n");
    }
    Ok(out)
}

// ------------------------------------------------------------------ wp_get_archives

/// The arguments of `wp_get_archives()`.
#[derive(Debug, Clone)]
pub struct Archives {
    pub kind: String,
    pub limit: Option<u64>,
    /// `html`, `option`, `link` or custom.
    pub format: String,
    pub before: String,
    pub after: String,
    pub show_post_count: bool,
    pub descending: bool,
}

impl Archives {
    pub fn new() -> Self {
        Self { kind: "monthly".into(), limit: None, format: "html".into(), before: String::new(), after: String::new(), show_post_count: false, descending: true }
    }
}

impl Default for Archives {
    fn default() -> Self {
        Self::new()
    }
}

/// `get_archives_link()`.
pub fn get_archives_link(env: &Env<'_>, url: &str, text: &str, format: &str, before: &str, after: &str, selected: bool) -> String {
    let text = env.site.texturize(text);
    let url = esc_url(url);
    let aria = if selected { " aria-current=\"page\"" } else { "" };
    match format {
        "link" => format!("\t<link rel='archives' title='{}' href='{url}' />\n", esc_attr(&text)),
        "option" => format!("\t<option value='{url}'{}>{before} {text} {after}</option>\n", if selected { " selected='selected'" } else { "" }),
        "html" => format!("\t<li>{before}<a href='{url}'{aria}>{text}</a>{after}</li>\n"),
        _ => format!("\t{before}<a href='{url}'{aria}>{text}</a>{after}\n"),
    }
}

/// `absint( get_query_var( $name ) )` as the string WordPress compares with.
fn query_number(env: &Env<'_>, name: &str) -> String {
    env.query.vars.u64(name).unwrap_or(0).to_string()
}

/// `wp_get_archives()` for posts.
pub async fn wp_get_archives(env: &Env<'_>, args: &Archives) -> Result<String> {
    let site = env.site;
    let kind = if args.kind.is_empty() { "monthly" } else { args.kind.as_str() };
    let is_archive = env.query.flags.is_archive;
    let (year, monthnum, day, week) = (query_number(env, "year"), query_number(env, "monthnum"), query_number(env, "day"), query_number(env, "w"));
    let locale = site.locale_data();
    let mut out = String::new();
    let after_with_count = |posts: u64| if args.show_post_count { format!("&nbsp;({posts}){}", args.after) } else { args.after.clone() };
    match kind {
        "monthly" | "yearly" | "daily" | "weekly" => {
            let start_of_week = site.options.u64("start_of_week", 0) as u32;
            let archive_kind = match kind {
                "monthly" => ArchiveKind::Monthly,
                "yearly" => ArchiveKind::Yearly,
                "daily" => ArchiveKind::Daily,
                _ => ArchiveKind::Weekly(start_of_week),
            };
            let rows = site.db.archive_rows(archive_kind, "post", args.descending, args.limit.filter(|l| *l > 0)).await?;
            let mut last_week = None;
            for row in rows {
                let after = after_with_count(row.posts);
                match kind {
                    "monthly" => {
                        let url = site.date_link(row.year as u32, row.month as u32, 0);
                        let text = sprintf(&site.tr("%1$s %2$d"), &[&locale.month[row.month as usize - 1], &row.year.to_string()]);
                        let selected = is_archive && year == row.year.to_string() && monthnum == row.month.to_string();
                        out.push_str(&get_archives_link(env, &url, &text, &args.format, &args.before, &after, selected));
                    }
                    "yearly" => {
                        let url = site.date_link(row.year as u32, 0, 0);
                        let selected = is_archive && year == row.year.to_string();
                        out.push_str(&get_archives_link(env, &url, &row.year.to_string(), &args.format, &args.before, &after, selected));
                    }
                    "daily" => {
                        let url = site.date_link(row.year as u32, row.month as u32, row.day as u32);
                        let date = format!("{}-{:02}-{:02} 00:00:00", row.year, row.month, row.day);
                        let ts = site.timezone.local_timestamp(&date).unwrap_or(0);
                        let text = site.date(&site.options.string("date_format", "F j, Y"), ts);
                        let selected = is_archive && year == row.year.to_string() && monthnum == row.month.to_string() && day == row.day.to_string();
                        out.push_str(&get_archives_link(env, &url, &text, &args.format, &args.before, &after, selected));
                    }
                    _ => {
                        if last_week == Some(row.week) {
                            continue;
                        }
                        last_week = Some(row.week);
                        let (start, end) = week_start_end(&row.yyyymmdd, start_of_week);
                        let format = site.options.string("date_format", "F j, Y");
                        let text = format!("{}&#8211;{}", site.date_utc(&format, start), site.date_utc(&format, end));
                        let url = site.home_url(&format!("/?m={}&w={}", row.year, row.week));
                        let selected = is_archive && year == row.year.to_string() && week == row.week.to_string();
                        out.push_str(&get_archives_link(env, &url, &text, &args.format, &args.before, &after, selected));
                    }
                }
            }
        }
        "postbypost" | "alpha" => {
            let posts = site.db.archive_posts("post", kind == "alpha", args.limit.filter(|l| *l > 0)).await?;
            for post in posts {
                if post.post_date.starts_with("0000-00-00") {
                    continue;
                }
                let url = site.permalink(&post).await?;
                let text = if post.post_title.is_empty() { post.id.to_string() } else { strip_tags(&the_title_for(site, &post)) };
                let selected = env.query.queried_object_id() == post.id && env.query.flags.is_singular();
                out.push_str(&get_archives_link(env, &url, &text, &args.format, &args.before, &args.after, selected));
            }
        }
        _ => {}
    }
    Ok(out)
}

/// `get_weekstartend()`: the timestamps (as UTC) of the first and the last second of the week.
fn week_start_end(yyyymmdd: &str, start_of_week: u32) -> (i64, i64) {
    let part = |range: std::ops::Range<usize>| yyyymmdd.get(range).and_then(|s| s.parse::<i64>().ok()).unwrap_or(1970);
    let date = chrono::NaiveDate::from_ymd_opt(part(0..4) as i32, part(5..7) as u32, part(8..10) as u32).unwrap_or_default();
    let day = date.and_hms_opt(0, 0, 0).map_or(0, |d| d.and_utc().timestamp());
    let mut weekday = i64::from(chrono::Datelike::weekday(&date).num_days_from_sunday());
    if weekday < i64::from(start_of_week) {
        weekday += 7;
    }
    let start = day - 86_400 * (weekday - i64::from(start_of_week));
    (start, start + 7 * 86_400 - 1)
}

// ------------------------------------------------------------------- wp_tag_cloud

/// The arguments of `wp_tag_cloud()` (`format => flat`, or a `list`).
#[derive(Debug, Clone)]
pub struct TagCloud {
    pub taxonomy: String,
    pub smallest: f64,
    pub largest: f64,
    pub unit: String,
    pub number: u64,
    pub list: bool,
    pub separator: String,
    pub show_count: bool,
}

impl TagCloud {
    pub fn new() -> Self {
        Self { taxonomy: "post_tag".into(), smallest: 8.0, largest: 22.0, unit: "pt".into(), number: 45, list: false, separator: "\n".into(), show_count: false }
    }
}

impl Default for TagCloud {
    fn default() -> Self {
        Self::new()
    }
}

/// `wp_tag_cloud()`: an empty string without tags.
pub async fn wp_tag_cloud(env: &Env<'_>, args: &TagCloud) -> Result<String> {
    let site = env.site;
    // The top `number` tags by count ("Always query top tags"), shown by name.
    let mut query = TermsArgs::new(&args.taxonomy);
    query.orderby = "count".into();
    query.descending = true;
    query.number = args.number;
    let mut tags = get_terms(env, &query).await?;
    if tags.is_empty() {
        return Ok(String::new());
    }
    tags.sort_by(|a, b| strnatcasecmp(&a.name, &b.name));
    let counts: Vec<f64> = tags.iter().map(|t| ((t.count as f64 + 1.0).log10() * 100.0).round()).collect();
    let min = counts.iter().copied().fold(f64::INFINITY, f64::min);
    let max = counts.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let spread = if max - min <= 0.0 { 1.0 } else { max - min };
    let font_spread = if args.largest - args.smallest < 0.0 { 1.0 } else { args.largest - args.smallest };
    let font_step = font_spread / spread;
    let aria_label = args.show_count || font_spread != 0.0;
    let mut links = Vec::new();
    for (i, tag) in tags.iter().enumerate() {
        let formatted = sprintf(&site.trn("%s item", "%s items", tag.count), &[&site.number_i18n(tag.count.max(0) as u64)]);
        let link = site.term_link(tag).await?;
        let role = if link == "#" { " role=\"button\"" } else { "" };
        let aria = if aria_label { format!(" aria-label=\"{} ({})\"", esc_attr(&tag.name), esc_attr(&formatted)) } else { String::new() };
        let count = if args.show_count { format!("<span class=\"tag-link-count\"> ({})</span>", tag.count) } else { String::new() };
        let font_size = args.smallest + (counts[i] - min) * font_step;
        links.push(format!(
            "<a href=\"{}\"{role} class=\"{}\" style=\"font-size: {};\"{aria}>{}{count}</a>",
            esc_url(&link),
            esc_attr(&format!("tag-cloud-link tag-link-{} tag-link-position-{}", tag.term_id, i + 1)),
            esc_attr(&format!("{}{}", php_float(font_size).replace(',', "."), args.unit)),
            esc_html(&tag.name),
        ));
    }
    Ok(if args.list { format!("<ul class='wp-tag-cloud' role='list'>\n\t<li>{}</li>\n</ul>\n", links.join("</li>\n\t<li>")) } else { links.join(&args.separator) })
}

// -------------------------------------------------------------------- get_calendar

/// `gmdate()` for the date formats of the calendar (English names, as WordPress prints them there).
fn gmdate_english(format: &str, year: u32, month: u32, day: u32) -> String {
    const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    let mut out = String::new();
    let mut chars = format.chars();
    while let Some(c) = chars.next() {
        match c {
            'F' => out.push_str(MONTHS[month as usize - 1]),
            'M' => out.push_str(&MONTHS[month as usize - 1][..3]),
            'j' => out.push_str(&day.to_string()),
            'd' => out.push_str(&format!("{day:02}")),
            'n' => out.push_str(&month.to_string()),
            'm' => out.push_str(&format!("{month:02}")),
            'Y' => out.push_str(&year.to_string()),
            'y' => out.push_str(&format!("{:02}", year % 100)),
            '\\' => out.extend(chars.next()),
            other => out.push(other),
        }
    }
    out
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400) => 29,
        _ => 28,
    }
}

/// Weekday of a date, 0 = Sunday.
fn weekday(year: u32, month: u32, day: u32) -> u32 {
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    (y + y / 4 - y / 100 + y / 400 + t[month as usize - 1] + day) % 7
}

/// `get_calendar( $initial )`. `override_month` is `$monthnum`/`$year` of a calendar block.
pub async fn get_calendar(env: &Env<'_>, initial: bool, override_month: Option<(u32, u32)>) -> Result<String> {
    let site = env.site;
    let locale = site.locale_data();
    let now = now_timestamp();
    let today = |code: &str| site.timezone.format(code, now).parse::<u32>().unwrap_or(0);
    let vars = &env.query.vars;
    let (this_year, this_month) = match (override_month, vars.u64("monthnum"), vars.u64("year"), vars.get("m")) {
        (Some((year, month)), ..) => (year, month),
        (None, Some(month), Some(year), _) if month > 0 && year > 0 => (year as u32, month as u32),
        (None, _, _, Some(m)) if !m.is_empty() => (m.get(..4).and_then(|y| y.parse().ok()).unwrap_or(0), if m.len() < 6 { 1 } else { m.get(4..6).and_then(|v| v.parse().ok()).unwrap_or(1) }),
        _ => (today("Y"), today("n")),
    };
    // `mktime( 0, 0, 0, $thismonth, 1, $thisyear )` normalizes an out-of-range month into the year; the caption
    // still prints the month name for the raw value (empty if there is none).
    let raw_month = this_month;
    let months = this_year as i64 * 12 + this_month as i64 - 1;
    let (this_year, this_month) = ((months.div_euclid(12)).max(1) as u32, months.rem_euclid(12) as u32 + 1);
    let week_begins = site.options.u64("start_of_week", 0) as u32;
    let last_day = days_in_month(this_year, this_month);
    // WordPress compares `post_date` with the raw string `$thisyear-$thismonth-01`; MariaDB reads an invalid month
    // as no date: nothing is "before" it and every post is "after" it (observed with `?m=201713`).
    let valid = (1..=12).contains(&raw_month);
    let previous = if valid { site.db.calendar_neighbor(this_year, this_month, last_day, true).await? } else { None };
    let next = if valid { site.db.calendar_neighbor(this_year, this_month, last_day, false).await? } else { site.db.calendar_neighbor(0, 1, 31, false).await? };
    let caption = sprintf(&site.trx("%1$s %2$s", "calendar caption"), &[locale.month.get((raw_month as usize).wrapping_sub(1)).map_or("", String::as_str), &this_year.to_string()]);
    let mut out = format!("<table id=\"wp-calendar\" class=\"wp-calendar-table\">\n\t<caption>{caption}</caption>\n\t<thead>\n\t<tr>");
    for i in 0..7 {
        let wd = ((i + week_begins) % 7) as usize;
        let label = if initial { &locale.weekday_initial[wd] } else { &locale.weekday_abbrev[wd] };
        out.push_str(&format!("\n\t\t<th scope=\"col\" aria-label=\"{}\">{label}</th>", esc_attr(&locale.weekday[wd])));
    }
    out.push_str("\n\t</tr>\n\t</thead>\n\t<tbody>\n\t<tr>");
    let with_posts = site.db.calendar_days(this_year, this_month, last_day).await?;
    let week_mod = |n: i64| n.rem_euclid(7);
    let pad = week_mod(weekday(this_year, this_month, 1) as i64 - week_begins as i64);
    if pad > 0 {
        out.push_str(&format!("\n\t\t<td colspan=\"{pad}\" class=\"pad\">&nbsp;</td>"));
    }
    let mut new_row = false;
    for day in 1..=last_day {
        if new_row {
            out.push_str("\n\t</tr>\n\t<tr>\n\t\t");
        }
        new_row = false;
        out.push_str(if today("j") == day && today("n") == this_month && today("Y") == this_year { "<td id=\"today\">" } else { "<td>" });
        if with_posts.contains(&day) {
            let date = gmdate_english(&site.trx("F j, Y", "daily archives date format"), this_year, this_month, day);
            let label = sprintf(&site.tr("Posts published on %s"), &[&date]);
            out.push_str(&format!("<a href=\"{}\" aria-label=\"{}\">{day}</a>", site.date_link(this_year, this_month, day), esc_attr(&label)));
        } else {
            out.push_str(&day.to_string());
        }
        out.push_str("</td>");
        if week_mod(weekday(this_year, this_month, day) as i64 - week_begins as i64) == 6 {
            new_row = true;
        }
    }
    // `$day` is one past the last day here.
    let pad = 7 - week_mod(weekday(this_year, this_month, last_day) as i64 + 1 - week_begins as i64);
    if (1..7).contains(&pad) {
        out.push_str(&format!("\n\t\t<td class=\"pad\" colspan=\"{pad}\">&nbsp;</td>"));
    }
    out.push_str("\n\t</tr>\n\t</tbody>\n\t</table>");
    out.push_str(&format!("<nav aria-label=\"{}\" class=\"wp-calendar-nav\">", site.tr("Previous and next months")));
    let abbrev = |month: u32| locale.month_abbrev[month as usize - 1].clone();
    match previous {
        Some((y, m)) => out.push_str(&format!("\n\t\t<span class=\"wp-calendar-nav-prev\"><a href=\"{}\">&laquo; {}</a></span>", site.date_link(y, m, 0), abbrev(m))),
        None => out.push_str("\n\t\t<span class=\"wp-calendar-nav-prev\">&nbsp;</span>"),
    }
    out.push_str("\n\t\t<span class=\"pad\">&nbsp;</span>");
    match next {
        Some((y, m)) => out.push_str(&format!("\n\t\t<span class=\"wp-calendar-nav-next\"><a href=\"{}\">{} &raquo;</a></span>", site.date_link(y, m, 0), abbrev(m))),
        None => out.push_str("\n\t\t<span class=\"wp-calendar-nav-next\">&nbsp;</span>"),
    }
    out.push_str("\n\t</nav>");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_print_with_fourteen_digits() {
        assert_eq!(php_float(8.0), "8");
        assert_eq!(php_float(22.0), "22");
        assert_eq!(php_float(10.857142857142857), "10.857142857143");
        assert_eq!(php_float(0.5), "0.5");
        assert_eq!(php_float(1e20), "1.0E+20");
    }

    #[test]
    fn natural_order_compares_numbers() {
        assert_eq!(strnatcasecmp("img2", "img10"), Ordering::Less);
        assert_eq!(strnatcasecmp("Alpha", "alpha"), Ordering::Equal);
        assert_eq!(strnatcasecmp("b", "A"), Ordering::Greater);
    }
}
