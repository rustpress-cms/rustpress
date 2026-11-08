//! `TijsVerkoyen\CssToInlineStyles\CssToInlineStyles::convert()` (2.2, as WPForms bundles it) with the selectors of
//! `Symfony\Component\CssSelector`: the rules of a style sheet become `style` attributes, in the order of their
//! specificity.

use std::sync::OnceLock;

use regex::Regex;

use super::dom::{self, Dom};

/// `Specificity` as `Rule\Processor::calculateSpecificityBasedOnASelector()` counts it.
type Specificity = (usize, usize, usize);

#[derive(Debug, Clone)]
struct Property {
    name: String,
    value: String,
    specificity: Specificity,
}

impl Property {
    fn important(&self) -> bool {
        self.value.to_ascii_lowercase().contains("!important")
    }
}

#[derive(Debug, Clone)]
struct Rule {
    selector: String,
    properties: Vec<Property>,
    specificity: Specificity,
    order: usize,
}

fn re(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("pattern"))
}

/// The common `cleanup()` of the processors: no line breaks, tabs as spaces, `"` as `'`, no comments, single spaces.
fn cleanup(s: &str) -> String {
    static COMMENT: OnceLock<Regex> = OnceLock::new();
    static SPACES: OnceLock<Regex> = OnceLock::new();
    let s = s.replace(['\r', '\n'], "").replace('\t', " ").replace('"', "'");
    let s = re(&COMMENT, r"/\*.*?\*/").replace_all(&s, "");
    re(&SPACES, r"\s\s+").replace_all(&s, " ").trim().to_owned()
}

/// `Css\Processor::doCleanup()`.
fn do_cleanup(css: &str) -> String {
    static CHARSET: OnceLock<Regex> = OnceLock::new();
    static MEDIA: OnceLock<Regex> = OnceLock::new();
    let css = re(&CHARSET, r#"@charset "[^"]+";"#).replace_all(css, "");
    let css = re(&MEDIA, r"@media [^{]*\{(?:[^{}]+|\{[^{}]*\})*\}").replace_all(&css, "");
    cleanup(&css)
}

/// `PHP trim()`.
fn trim(s: &str) -> &str {
    s.trim_matches([' ', '\t', '\n', '\r', '\0', '\x0B'])
}

/// `Property\Processor::splitIntoSeparateProperties()` and `convertArrayToObjects()`.
fn properties(text: &str, specificity: Specificity) -> Vec<Property> {
    let text = cleanup(text);
    let text = text.trim_end_matches(';');
    let mut parts: Vec<String> = text.split(';').map(|p| trim(p).to_owned()).collect();
    let mut i = 0;
    while i + 1 < parts.len() {
        if trim(&parts[i + 1]).starts_with("base64,") {
            let next = parts.remove(i + 1);
            parts[i] = format!("{};{}", parts[i], trim(&next));
        }
        i += 1;
    }
    parts
        .into_iter()
        .filter_map(|p| {
            let (name, value) = p.split_once(':')?;
            let value = trim(value);
            (!value.is_empty()).then(|| Property { name: trim(name).to_owned(), value: value.to_owned(), specificity })
        })
        .collect()
}

/// `calculateSpecificityBasedOnASelector()`.
fn specificity(selector: &str) -> Specificity {
    static B: OnceLock<Regex> = OnceLock::new();
    static C: OnceLock<Regex> = OnceLock::new();
    let a = selector.matches('#').count();
    let b = re(
        &B,
        r"(?i)(\.[A-Za-z0-9_]+)|\[([A-Za-z0-9_]+)|(:(link|visited|active|hover|focus|lang|target|enabled|disabled|checked|indeterminate|root|nth-child|nth-last-child|nth-of-type|nth-last-of-type|first-child|last-child|first-of-type|last-of-type|only-child|only-of-type|empty|contains))",
    )
    .find_iter(selector)
    .count();
    let c = re(&C, r"(?i)((^|[\s+>~]+)[A-Za-z0-9_]+|:{1,2}(after|before|first-letter|first-line|selection))").find_iter(selector).count();
    (a, b, c)
}

/// `Css\Processor::getRules()`.
fn rules(css: &str) -> Vec<Rule> {
    let css = do_cleanup(css);
    let css = cleanup(&css);
    let css = css.trim_end_matches('}');
    let mut out = Vec::new();
    for (i, rule) in css.split('}').enumerate() {
        let rule = cleanup(rule);
        let rule = rule.trim_end_matches('}');
        let chunks: Vec<&str> = rule.split('{').collect();
        let Some(body) = chunks.get(1) else { continue };
        for selector in trim(chunks[0]).split(',') {
            let selector = trim(selector);
            let spec = specificity(selector);
            out.push(Rule { selector: selector.to_owned(), properties: properties(body, spec), specificity: spec, order: i + 1 });
        }
    }
    out
}

/// A compound selector: the element, its conditions, and how it is joined to the one before it.
#[derive(Debug, Clone)]
struct Compound {
    element: Option<String>,
    conditions: Vec<Condition>,
    /// ' ', '>', '+', '~' to the compound before; unused on the first.
    combinator: char,
}

#[derive(Debug, Clone)]
enum Condition {
    Class(String),
    Id(String),
    Attr(String, Option<(String, String)>),
    Never,
    FirstChild,
    LastChild,
    OnlyChild,
    FirstOfType,
    LastOfType,
    OnlyOfType,
    Empty,
    Root,
    Link,
    Checked,
    Not(Box<Compound>),
}

fn ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_' || !c.is_ascii()
}

fn ident(s: &str, i: &mut usize) -> Option<String> {
    let start = *i;
    for (o, c) in s[*i..].char_indices() {
        if !ident_char(c) {
            *i = start + o;
            return (o > 0).then(|| s[start..start + o].to_owned());
        }
    }
    *i = s.len();
    (start < s.len()).then(|| s[start..].to_owned())
}

/// The compound selector at `i`; `None` for a selector that Symfony's converter cannot translate.
fn compound(s: &str, i: &mut usize, negated: bool) -> Option<Compound> {
    let mut c = Compound { element: None, conditions: Vec::new(), combinator: ' ' };
    let b = s.as_bytes();
    let mut any = false;
    if *i < s.len() && b[*i] == b'*' {
        *i += 1;
        any = true;
    } else if *i < s.len() && ident_char(s[*i..].chars().next()?) && b[*i] != b'-' {
        c.element = Some(ident(s, i)?.to_ascii_lowercase());
        any = true;
    }
    while *i < s.len() {
        match b[*i] {
            b'.' => {
                *i += 1;
                c.conditions.push(Condition::Class(ident(s, i)?));
            }
            b'#' => {
                *i += 1;
                c.conditions.push(Condition::Id(ident(s, i)?));
            }
            b'[' => {
                let end = s[*i..].find(']')? + *i;
                let inner = s[*i + 1..end].trim();
                *i = end + 1;
                let ops = ["~=", "|=", "^=", "$=", "*=", "!=", "="];
                let found = ops.iter().find_map(|op| inner.find(op).map(|p| (p, *op)));
                c.conditions.push(match found {
                    Some((p, op)) => {
                        let value = inner[p + op.len()..].trim().trim_matches(['\'', '"']).to_owned();
                        if op == "!=" {
                            return None;
                        }
                        Condition::Attr(inner[..p].trim().to_ascii_lowercase(), Some((op.to_owned(), value)))
                    }
                    None => Condition::Attr(inner.to_ascii_lowercase(), None),
                });
            }
            b':' => {
                *i += 1;
                if b.get(*i) == Some(&b':') {
                    return None;
                }
                let name = ident(s, i)?.to_ascii_lowercase();
                if b.get(*i) == Some(&b'(') {
                    if name != "not" || negated {
                        return None;
                    }
                    *i += 1;
                    let inner = compound(s, i, true)?;
                    if b.get(*i) != Some(&b')') {
                        return None;
                    }
                    *i += 1;
                    c.conditions.push(Condition::Not(Box::new(inner)));
                    continue;
                }
                c.conditions.push(match name.as_str() {
                    "hover" | "visited" => Condition::Never,
                    "first-child" => Condition::FirstChild,
                    "last-child" => Condition::LastChild,
                    "only-child" => Condition::OnlyChild,
                    "first-of-type" if c.element.is_some() => Condition::FirstOfType,
                    "last-of-type" if c.element.is_some() => Condition::LastOfType,
                    "only-of-type" => Condition::OnlyOfType,
                    "empty" => Condition::Empty,
                    "root" => Condition::Root,
                    "link" => Condition::Link,
                    "checked" => Condition::Checked,
                    _ => return None,
                });
            }
            _ => break,
        }
        any = true;
    }
    any.then_some(c)
}

/// `CssSelectorConverter::toXPath()`: the compounds of a selector, or `None` when it throws.
fn parse_selector(s: &str) -> Option<Vec<Compound>> {
    let s = s.trim();
    let mut i = 0;
    let mut out = Vec::new();
    let mut combinator = ' ';
    loop {
        let mut c = compound(s, &mut i, false)?;
        c.combinator = combinator;
        out.push(c);
        let mut saw_space = false;
        while i < s.len() && s.as_bytes()[i] == b' ' {
            i += 1;
            saw_space = true;
        }
        if i >= s.len() {
            break;
        }
        combinator = match s.as_bytes()[i] {
            b'>' | b'+' | b'~' => {
                let c = s.as_bytes()[i] as char;
                i += 1;
                while i < s.len() && s.as_bytes()[i] == b' ' {
                    i += 1;
                }
                c
            }
            _ if saw_space => ' ',
            _ => return None,
        };
    }
    Some(out)
}

fn element_children(dom: &Dom, parent: usize) -> Vec<usize> {
    dom.nodes[parent].children.iter().copied().filter(|&c| dom.name(c).is_some()).collect()
}

fn parent_element(dom: &Dom, id: usize) -> Option<usize> {
    dom.nodes[id].parent.filter(|&p| dom.name(p).is_some())
}

fn matches_compound(dom: &Dom, id: usize, c: &Compound) -> bool {
    let Some(name) = dom.name(id) else { return false };
    if c.element.as_deref().is_some_and(|e| e != name) {
        return false;
    }
    c.conditions.iter().all(|cond| matches_condition(dom, id, name, cond))
}

fn matches_condition(dom: &Dom, id: usize, name: &str, cond: &Condition) -> bool {
    let siblings = || parent_element(dom, id).map(|p| element_children(dom, p));
    match cond {
        Condition::Class(class) => dom.attr(id, "class").is_some_and(|v| v.split([' ', '\t', '\n', '\r']).any(|t| t == class)),
        Condition::Id(value) => dom.attr(id, "id") == Some(value.as_str()),
        Condition::Attr(attr, test) => match (dom.attr(id, attr), test) {
            (None, _) => false,
            (Some(_), None) => true,
            (Some(v), Some((op, want))) => match op.as_str() {
                "=" => v == want,
                "~=" => v.split([' ', '\t', '\n', '\r']).any(|t| t == want),
                "|=" => v == want || v.starts_with(&format!("{want}-")),
                "^=" => !want.is_empty() && v.starts_with(want.as_str()),
                "$=" => !want.is_empty() && v.ends_with(want.as_str()),
                "*=" => !want.is_empty() && v.contains(want.as_str()),
                _ => false,
            },
        },
        Condition::Never => false,
        Condition::FirstChild => siblings().is_some_and(|s| s.first() == Some(&id)),
        Condition::LastChild => siblings().is_some_and(|s| s.last() == Some(&id)),
        Condition::OnlyChild => siblings().is_some_and(|s| s.len() == 1),
        Condition::FirstOfType | Condition::LastOfType | Condition::OnlyOfType => {
            let same: Vec<usize> = match cond {
                Condition::OnlyOfType => dom.nodes[id].parent.map(|p| dom.nodes[p].children.iter().copied().filter(|&c| dom.name(c) == Some(name)).collect()).unwrap_or_default(),
                _ => siblings().unwrap_or_default().into_iter().filter(|&c| dom.name(c) == Some(name)).collect(),
            };
            match cond {
                Condition::FirstOfType => same.first() == Some(&id),
                Condition::LastOfType => same.last() == Some(&id),
                _ => same.len() == 1,
            }
        }
        Condition::Empty => dom.nodes[id].children.iter().all(|&c| matches!(&dom.nodes[c].kind, dom::Kind::Comment(_)) || matches!(&dom.nodes[c].kind, dom::Kind::Text(t) if t.is_empty())),
        Condition::Root => parent_element(dom, id).is_none(),
        Condition::Link => dom.attr(id, "href").is_some() && matches!(name, "a" | "link" | "area"),
        Condition::Checked => {
            (dom.attr(id, "selected").is_some() && name == "option")
                || (dom.attr(id, "checked").is_some() && matches!(name, "input" | "command") && matches!(dom.attr(id, "type"), Some("checkbox" | "radio")))
        }
        Condition::Not(inner) => {
            if inner.conditions.is_empty() && inner.element.is_none() {
                return false;
            }
            !matches_compound(dom, id, inner)
        }
    }
}

/// Whether the element matches the whole selector, from its last compound back.
fn matches(dom: &Dom, id: usize, selector: &[Compound]) -> bool {
    let Some((last, rest)) = selector.split_last() else { return false };
    if !matches_compound(dom, id, last) {
        return false;
    }
    if rest.is_empty() {
        return true;
    }
    match last.combinator {
        '>' => parent_element(dom, id).is_some_and(|p| matches(dom, p, rest)),
        '+' => previous_elements(dom, id).first().is_some_and(|&p| matches(dom, p, rest)),
        '~' => previous_elements(dom, id).into_iter().any(|p| matches(dom, p, rest)),
        _ => {
            let mut cur = parent_element(dom, id);
            while let Some(p) = cur {
                if matches(dom, p, rest) {
                    return true;
                }
                cur = parent_element(dom, p);
            }
            false
        }
    }
}

/// The element siblings before `id`, the nearest first.
fn previous_elements(dom: &Dom, id: usize) -> Vec<usize> {
    let Some(parent) = dom.nodes[id].parent else { return Vec::new() };
    let siblings = &dom.nodes[parent].children;
    let pos = siblings.iter().position(|&c| c == id).unwrap_or(0);
    siblings[..pos].iter().rev().copied().filter(|&c| dom.name(c).is_some()).collect()
}

/// `calculatePropertiesToBeApplied()`.
fn apply(properties: &[Property], existing: &mut Vec<Property>) {
    for property in properties {
        match existing.iter().position(|p| p.name == property.name) {
            Some(pos) => {
                let old = &existing[pos];
                if old.important() && !property.important() {
                    continue;
                }
                let overrule = (!old.important() && property.important()) || old.specificity <= property.specificity;
                if overrule {
                    existing.remove(pos);
                    existing.push(property.clone());
                }
            }
            None => existing.push(property.clone()),
        }
    }
}

/// `CssToInlineStyles::convert( $html, $css )`, for a document without `<style>` elements.
pub fn convert(html: &str, css: &str) -> String {
    let mut dom = dom::parse(html, dom::Options::default());
    let mut rules = rules(css);
    rules.sort_by(|a, b| a.specificity.cmp(&b.specificity).then(a.order.cmp(&b.order)));
    let elements = dom.elements();
    let mut storage: Vec<(usize, Vec<Property>)> = Vec::new();
    for rule in &rules {
        let Some(selector) = parse_selector(&rule.selector) else { continue };
        for &id in &elements {
            if !matches(&dom, id, &selector) {
                continue;
            }
            match storage.iter_mut().find(|(e, _)| *e == id) {
                Some((_, props)) => apply(&rule.properties, props),
                None => {
                    let mut props = Vec::new();
                    apply(&rule.properties, &mut props);
                    storage.push((id, props));
                }
            }
        }
    }
    for (id, props) in storage {
        if props.is_empty() {
            continue;
        }
        // `inlineCssOnElement()`: the element's own declarations win and come last.
        let inline = properties(dom.attr(id, "style").unwrap_or(""), (0, 0, 0));
        let mut own: Vec<Property> = Vec::new();
        for p in inline {
            match own.iter_mut().find(|o| o.name == p.name) {
                Some(o) => *o = p,
                None => own.push(p),
            }
        }
        let mut list: Vec<&Property> = props.iter().filter(|p| !own.iter().any(|o| o.name == p.name)).collect();
        list.extend(own.iter());
        let style = list.iter().map(|p| format!("{}: {};", p.name, p.value)).collect::<Vec<_>>().join(" ");
        dom.set_attr(id, "style", &style);
    }
    let Some(root) = dom.document_element() else { return String::new() };
    let html = dom.save(root, true);
    let mut doctype = String::new();
    for &c in &dom.nodes[0].children {
        if c != root {
            doctype.push_str(&dom.save(c, true));
        }
    }
    let mut doctype = trim(&doctype).to_owned();
    if doctype == "<!DOCTYPE html>" {
        doctype = doctype.to_lowercase();
    }
    format!("{doctype}\n{}", trim(&html))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specificity_like_the_processor() {
        assert_eq!(specificity("body"), (0, 0, 1));
        assert_eq!(specificity(".content .field-value"), (0, 2, 0));
        assert_eq!(specificity("h1 a:visited"), (0, 1, 2));
        assert_eq!(specificity("tr:not(.smart-tag)>.field-value span"), (0, 2, 2));
        assert_eq!(specificity("#outlook a"), (1, 0, 1));
    }

    #[test]
    fn selectors() {
        let dom = dom::parse("<html><body><table class=\"body x\"><tr class=\"smart-tag\"><td class=\"field-value\"><span>a</span></td><td>b</td></tr></table></body></html>", dom::Options::default());
        let find = |s: &str| {
            let sel = parse_selector(s).unwrap();
            dom.elements().into_iter().filter(|&e| matches(&dom, e, &sel)).map(|e| dom.name(e).unwrap().to_owned()).collect::<Vec<_>>()
        };
        assert_eq!(find("table.body"), ["table"]);
        assert_eq!(find("tr:not(.smart-tag)>.field-value span"), Vec::<String>::new());
        assert_eq!(find(".body td:last-child"), ["td"]);
        assert_eq!(find("td>*:last-child"), ["span"]);
        assert_eq!(find("a:visited"), Vec::<String>::new());
        assert!(parse_selector("a:active").is_none());
        assert!(parse_selector("@media(max-width: 600px)").is_none());
    }
}
