//! `enshrined/svg-sanitize` (the version SVG Support bundles) with the parts of libxml it relies on.
//!
//! The library loads the file with `DOMDocument::loadXML()` (blanks between elements dropped, entities not
//! substituted in text), removes every element and attribute that is not on its lists, unsafe `href`s, remote
//! `url(...)` references, `use` elements that point elsewhere or nest too deep, comments, processing instructions and
//! the doctype, and saves the document again with `formatOutput` (or minified). This module does the same on its
//! own tree: what libxml would refuse to load is refused here, and the output follows `xmlDocDumpFormatMemory()`.
//!
//! One difference: entities a document declares are expanded in attribute values (as libxml does) but references
//! to them in text are dropped (libxml keeps them as entity reference nodes, which the sanitizer removes) – the
//! same result.

use std::collections::{HashMap, HashSet};

/// `AllowedTags::getTags()`, lower case.
const TAGS: &[&str] = &[
    "a", "font", "image", "style", "svg", "altglyph", "altglyphdef", "altglyphitem", "animatecolor", "animatemotion", "animatetransform", "circle", "clippath", "defs", "desc", "ellipse", "filter", "g", "glyph",
    "glyphref", "hkern", "line", "lineargradient", "marker", "mask", "metadata", "mpath", "path", "pattern", "polygon", "polyline", "radialgradient", "rect", "stop", "switch", "symbol", "text", "textpath", "title",
    "tref", "tspan", "use", "view", "vkern", "feblend", "fecolormatrix", "fecomponenttransfer", "fecomposite", "feconvolvematrix", "fediffuselighting", "fedisplacementmap", "fedistantlight", "feflood", "fefunca",
    "fefuncb", "fefuncg", "fefuncr", "fegaussianblur", "femerge", "femergenode", "femorphology", "feoffset", "fepointlight", "fespecularlighting", "fespotlight", "fetile", "feturbulence", "#text",
];

/// `AllowedAttributes::getAttributes()`, lower case.
const ATTRIBUTES: &[&str] = &[
    "about", "accept", "action", "align", "alt", "autocomplete", "background", "bgcolor", "border", "cellpadding", "cellspacing", "checked", "cite", "class", "clear", "color", "cols", "colspan", "coords",
    "crossorigin", "datetime", "default", "dir", "disabled", "download", "enctype", "encoding", "face", "for", "headers", "height", "hidden", "high", "href", "hreflang", "id", "integrity", "ismap", "label", "lang",
    "list", "loop", "low", "max", "maxlength", "media", "method", "min", "multiple", "name", "noshade", "novalidate", "nowrap", "open", "optimum", "pattern", "placeholder", "poster", "preload", "pubdate",
    "radiogroup", "readonly", "rel", "required", "rev", "reversed", "role", "rows", "rowspan", "spellcheck", "scope", "selected", "shape", "size", "sizes", "span", "srclang", "start", "src", "srcset", "step",
    "style", "summary", "tabindex", "title", "type", "usemap", "valign", "value", "version", "width", "xmlns", "accent-height", "accumulate", "additivive", "alignment-baseline", "ascent", "attributename",
    "attributetype", "azimuth", "basefrequency", "baseline-shift", "begin", "bias", "by", "clip", "clip-path", "clip-rule", "color-interpolation", "color-interpolation-filters", "color-profile",
    "color-rendering", "cx", "cy", "d", "dx", "dy", "diffuseconstant", "direction", "display", "divisor", "dominant-baseline", "dur", "edgemode", "elevation", "end", "fill", "fill-opacity", "fill-rule", "filter",
    "filterunits", "flood-color", "flood-opacity", "font-family", "font-size", "font-size-adjust", "font-stretch", "font-style", "font-variant", "font-weight", "fx", "fy", "g1", "g2", "glyph-name", "glyphref",
    "gradientunits", "gradienttransform", "image-rendering", "in", "in2", "k", "k1", "k2", "k3", "k4", "kerning", "keypoints", "keysplines", "keytimes", "lengthadjust", "letter-spacing", "kernelmatrix",
    "kernelunitlength", "lighting-color", "local", "marker-end", "marker-mid", "marker-start", "markerheight", "markerunits", "markerwidth", "maskcontentunits", "maskunits", "mask", "mode", "numoctaves",
    "offset", "operator", "opacity", "order", "orient", "orientation", "origin", "overflow", "paint-order", "path", "pathlength", "patterncontentunits", "patterntransform", "patternunits", "points",
    "preservealpha", "preserveaspectratio", "r", "rx", "ry", "radius", "refx", "refy", "repeatcount", "repeatdur", "restart", "result", "rotate", "scale", "seed", "shape-rendering", "specularconstant",
    "specularexponent", "spreadmethod", "stddeviation", "stitchtiles", "stop-color", "stop-opacity", "stroke-dasharray", "stroke-dashoffset", "stroke-linecap", "stroke-linejoin", "stroke-miterlimit",
    "stroke-opacity", "stroke", "stroke-width", "surfacescale", "targetx", "targety", "transform", "text-anchor", "text-decoration", "text-rendering", "textlength", "u1", "u2", "unicode", "values", "viewbox",
    "visibility", "vector-effect", "vert-adv-y", "vert-origin-x", "vert-origin-y", "word-spacing", "wrap", "writing-mode", "xchannelselector", "ychannelselector", "x", "x1", "x2", "y", "y1", "y2", "z",
    "zoomandpan", "accent", "accentunder", "bevelled", "close", "columnsalign", "columnlines", "columnspan", "denomalign", "depth", "displaystyle", "fence", "frame", "largeop", "length", "linethickness",
    "lspace", "lquote", "mathbackground", "mathcolor", "mathsize", "mathvariant", "maxsize", "minsize", "movablelimits", "notation", "numalign", "rowalign", "rowlines", "rowspacing", "rspace", "rquote",
    "scriptlevel", "scriptminsize", "scriptsizemultiplier", "selection", "separator", "separators", "slope", "stretchy", "subscriptshift", "supscriptshift", "symmetric", "voffset", "xlink:href", "xml:id",
    "xlink:title", "xml:space", "xmlns:xlink",
];

const XLINK: &str = "http://www.w3.org/1999/xlink";
const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";
/// `$useThreshold` and `$useNestingLimit`.
const USE_THRESHOLD: u64 = 1000;
const USE_NESTING_LIMIT: usize = 15;

#[derive(Debug, Clone)]
struct Attr {
    /// The prefix as written when it is bound (`xlink`), else empty (an unbound `p:x` keeps the whole name as `local`).
    prefix: String,
    local: String,
    namespace: Option<String>,
    /// The value with every reference replaced (what `getAttribute()` returns).
    value: String,
    /// How libxml writes a value that refers to declared entities: the references stay (`&ns_svg;`).
    written: Option<String>,
}

impl Attr {
    /// `$attr->nodeName`.
    fn name(&self) -> String {
        if self.prefix.is_empty() { self.local.clone() } else { format!("{}:{}", self.prefix, self.local) }
    }
}

#[derive(Debug, Clone)]
struct Element {
    uid: usize,
    /// `tagName`: the name as written.
    name: String,
    namespace: Option<String>,
    /// `xmlns` and `xmlns:p` declarations, printed before the attributes: name and value as written out.
    ns_decls: Vec<(String, String)>,
    attrs: Vec<Attr>,
    children: Vec<Node>,
}

impl Element {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|a| a.name() == name).map(|a| a.value.as_str())
    }

    fn remove_attr(&mut self, name: &str) {
        if let Some(index) = self.attrs.iter().position(|a| a.name() == name) {
            self.attrs.remove(index);
        }
    }

    /// `Helper::getElementHref()`.
    fn href(&self) -> Option<&str> {
        self.attr("href").or_else(|| self.attrs.iter().find(|a| a.namespace.as_deref() == Some(XLINK) && a.local == "href").map(|a| a.value.as_str()))
    }
}

#[derive(Debug, Clone)]
enum Node {
    Element(Element),
    Text(String),
    Cdata(String),
    /// Comments, processing instructions and references to declared entities: removed by the sanitizer.
    Other,
}

struct Document {
    version: String,
    encoding: Option<String>,
    standalone: Option<bool>,
    /// The nodes of the document: the root element and what surrounds it.
    children: Vec<Node>,
}

/// Why a file could not be loaded.
#[derive(Debug)]
struct Invalid;

fn is_blank(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}

/// The predefined entities and character references; `None` for another name.
fn predefined(name: &str) -> Option<String> {
    match name {
        "lt" => Some("<".into()),
        "gt" => Some(">".into()),
        "amp" => Some("&".into()),
        "apos" => Some("'".into()),
        "quot" => Some("\"".into()),
        _ => {
            let number = name.strip_prefix('#')?;
            let code = match number.strip_prefix('x') {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => number.parse().ok()?,
            };
            let c = char::from_u32(code)?;
            // Characters XML does not allow.
            if matches!(code, 0x1..=0x8 | 0xB | 0xC | 0xE..=0x1F | 0xFFFE | 0xFFFF) || code == 0 {
                return None;
            }
            Some(c.to_string())
        }
    }
}

/// A piece of text: characters, or a reference to a declared entity.
enum Piece {
    /// Literal characters (`literal` true) or the replacement of a character reference.
    Chars(String, bool),
    Entity,
}

/// Splits text at its references.
fn pieces(text: &str, entities: &HashMap<String, String>) -> Result<Vec<Piece>, Invalid> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        if start > 0 {
            out.push(Piece::Chars(rest[..start].to_owned(), true));
        }
        let after = &rest[start + 1..];
        let end = after.find(';').ok_or(Invalid)?;
        let name = &after[..end];
        if let Some(value) = predefined(name) {
            out.push(Piece::Chars(value, false));
        } else if entities.contains_key(name) {
            out.push(Piece::Entity);
        } else {
            return Err(Invalid);
        }
        rest = &after[end + 1..];
    }
    if !rest.is_empty() {
        out.push(Piece::Chars(rest.to_owned(), true));
    }
    Ok(out)
}

/// An attribute value as libxml keeps it.
struct AttrValue {
    /// Every reference replaced, whitespace normalized.
    value: String,
    /// The value with references to declared entities left as they are (the namespace URI libxml takes from an
    /// `xmlns` attribute).
    literal: String,
    /// The written form when the value refers to declared entities.
    written: Option<String>,
}

/// The value of an attribute after reference substitution and whitespace normalization.
fn attribute_value(raw: &str, entities: &HashMap<String, String>) -> Result<AttrValue, Invalid> {
    if raw.contains('<') {
        return Err(Invalid);
    }
    let normalize = |text: &str| text.chars().map(|c| if matches!(c, '\t' | '\n' | '\r') { ' ' } else { c }).collect::<String>();
    let mut value = String::new();
    let mut literal = String::new();
    let mut written = String::new();
    let mut refers = false;
    let mut rest = raw;
    while let Some(start) = rest.find('&') {
        let text = normalize(&rest[..start]);
        value.push_str(&text);
        literal.push_str(&text);
        escape_attribute(&text, &mut written);
        let after = &rest[start + 1..];
        let end = after.find(';').ok_or(Invalid)?;
        let name = &after[..end];
        match predefined(name) {
            Some(text) => {
                value.push_str(&text);
                literal.push_str(&text);
                escape_attribute(&text, &mut written);
            }
            None => {
                let replacement = entities.get(name).ok_or(Invalid)?;
                value.push_str(&normalize(replacement));
                literal.push_str(&format!("&{name};"));
                written.push_str(&format!("&{name};"));
                refers = true;
            }
        }
        rest = &after[end + 1..];
    }
    let text = normalize(rest);
    value.push_str(&text);
    literal.push_str(&text);
    escape_attribute(&text, &mut written);
    Ok(AttrValue { value, literal, written: refers.then_some(written) })
}

/// An element being parsed.
struct Open {
    element: Element,
    /// The prefixes it declares.
    scope: Vec<(String, String)>,
    /// `xml:space="preserve"` applies.
    preserve: bool,
}

/// `DOMDocument::loadXML()` with `preserveWhiteSpace = false`.
fn parse(text: &str) -> Result<Document, Invalid> {
    use xmlparser::{ElementEnd, EntityDefinition, Token, Tokenizer};
    let mut doc = Document { version: "1.0".into(), encoding: None, standalone: None, children: Vec::new() };
    let mut entities: HashMap<String, String> = HashMap::new();
    let mut stack: Vec<Open> = Vec::new();
    let mut scopes: Vec<(String, String)> = vec![("xml".into(), XML_NS.into())];
    let mut next_uid = 0usize;
    let mut root_seen = false;
    // The start tag being read: name, attributes as written.
    let mut pending: Option<(String, String, Vec<(String, String, String)>)> = None;
    let lookup = |scopes: &[(String, String)], prefix: &str| scopes.iter().rev().find(|(p, _)| p == prefix).map(|(_, uri)| uri.clone());
    let tokens: Vec<Token<'_>> = Tokenizer::from(text).collect::<Result<Vec<_>, _>>().map_err(|_| Invalid)?;
    let mut index = 0;
    while index < tokens.len() {
        let token = &tokens[index];
        index += 1;
        match token {
            Token::Declaration { version, encoding, standalone, .. } => {
                doc.version = version.as_str().to_owned();
                doc.encoding = encoding.map(|e| e.as_str().to_owned());
                doc.standalone = *standalone;
            }
            Token::EntityDeclaration { name, definition, .. } => {
                if let EntityDefinition::EntityValue(value) = definition {
                    let mut decoded = String::new();
                    for piece in pieces(value.as_str(), &HashMap::new()).unwrap_or_default() {
                        if let Piece::Chars(text, _) = piece {
                            decoded.push_str(&text);
                        }
                    }
                    entities.entry(name.as_str().to_owned()).or_insert(decoded);
                }
            }
            Token::ProcessingInstruction { .. } | Token::Comment { .. } => {
                if let Some(open) = stack.last_mut() {
                    open.element.children.push(Node::Other);
                }
            }
            Token::DtdStart { .. } | Token::EmptyDtd { .. } | Token::DtdEnd { .. } => {}
            Token::ElementStart { prefix, local, .. } => {
                if stack.is_empty() && root_seen {
                    return Err(Invalid);
                }
                pending = Some((prefix.as_str().to_owned(), local.as_str().to_owned(), Vec::new()));
            }
            Token::Attribute { prefix, local, value, .. } => {
                let Some((_, _, attrs)) = pending.as_mut() else { return Err(Invalid) };
                attrs.push((prefix.as_str().to_owned(), local.as_str().to_owned(), value.as_str().to_owned()));
            }
            Token::ElementEnd { end, .. } => match end {
                ElementEnd::Open | ElementEnd::Empty => {
                    let Some((prefix, local, raw_attrs)) = pending.take() else { return Err(Invalid) };
                    // Namespace declarations first, then the attributes resolve against them.
                    let mut scope = Vec::new();
                    let mut ns_decls = Vec::new();
                    let mut names = HashSet::new();
                    let mut values = Vec::new();
                    for (attr_prefix, attr_local, raw) in &raw_attrs {
                        let qualified = if attr_prefix.is_empty() { attr_local.clone() } else { format!("{attr_prefix}:{attr_local}") };
                        if !names.insert(qualified.clone()) {
                            return Err(Invalid);
                        }
                        let parsed = attribute_value(raw, &entities)?;
                        if (attr_prefix.is_empty() && attr_local == "xmlns") || attr_prefix == "xmlns" {
                            let prefix = if attr_prefix.is_empty() { String::new() } else { attr_local.clone() };
                            let mut shown = String::new();
                            escape_attribute(&parsed.value, &mut shown);
                            ns_decls.push((qualified, parsed.written.clone().unwrap_or(shown)));
                            scope.push((prefix, parsed.literal));
                        } else {
                            values.push((attr_prefix.clone(), attr_local.clone(), parsed));
                        }
                    }
                    scopes.extend(scope.iter().cloned());
                    let attrs: Vec<Attr> = values
                        .into_iter()
                        .map(|(attr_prefix, attr_local, parsed)| {
                            let (value, written) = (parsed.value, parsed.written);
                            if attr_prefix.is_empty() {
                                return Attr { prefix: String::new(), local: attr_local, namespace: None, value, written };
                            }
                            match lookup(&scopes, &attr_prefix) {
                                Some(uri) => Attr { prefix: attr_prefix, local: attr_local, namespace: Some(uri), value, written },
                                // An unbound prefix stays part of the name.
                                None => Attr { prefix: String::new(), local: format!("{attr_prefix}:{attr_local}"), namespace: None, value, written },
                            }
                        })
                        .collect();
                    let name = if prefix.is_empty() { local.clone() } else { format!("{prefix}:{local}") };
                    let namespace = lookup(&scopes, &prefix).filter(|uri| !uri.is_empty());
                    let preserve = match attrs.iter().find(|a| a.namespace.as_deref() == Some(XML_NS) && a.local == "space").map(|a| a.value.as_str()) {
                        Some("preserve") => true,
                        Some("default") => false,
                        _ => stack.last().is_some_and(|o| o.preserve),
                    };
                    let element = Element { uid: next_uid, name, namespace, ns_decls, attrs, children: Vec::new() };
                    next_uid += 1;
                    let open = Open { element, scope, preserve };
                    if matches!(end, ElementEnd::Empty) {
                        close(open, &mut stack, &mut scopes, &mut doc, &mut root_seen);
                    } else {
                        stack.push(open);
                    }
                }
                ElementEnd::Close(prefix, local) => {
                    let Some(open) = stack.pop() else { return Err(Invalid) };
                    let written = if prefix.as_str().is_empty() { local.as_str().to_owned() } else { format!("{}:{}", prefix.as_str(), local.as_str()) };
                    if written != open.element.name {
                        return Err(Invalid);
                    }
                    close(open, &mut stack, &mut scopes, &mut doc, &mut root_seen);
                }
            },
            Token::Text { text } => {
                let raw = text.as_str();
                let Some(open) = stack.last_mut() else {
                    // Outside the root only blanks may stand.
                    if raw.chars().all(is_blank) {
                        continue;
                    }
                    return Err(Invalid);
                };
                let followed_by_end = matches!(tokens.get(index), Some(Token::ElementEnd { end: ElementEnd::Close(..), .. }));
                let parts = pieces(raw, &entities)?;
                let count = parts.len();
                for (i, part) in parts.into_iter().enumerate() {
                    match part {
                        Piece::Entity => open.element.children.push(Node::Other),
                        Piece::Chars(chars, literal) => {
                            let last = i + 1 == count;
                            if literal && last && chars.chars().all(is_blank) && !open.preserve && is_ignorable(&open.element, followed_by_end) {
                                continue;
                            }
                            match open.element.children.last_mut() {
                                Some(Node::Text(previous)) => previous.push_str(&chars),
                                _ => open.element.children.push(Node::Text(chars)),
                            }
                        }
                    }
                }
            }
            Token::Cdata { text, .. } => {
                let Some(open) = stack.last_mut() else { return Err(Invalid) };
                open.element.children.push(Node::Cdata(text.as_str().to_owned()));
            }
        }
    }
    if !stack.is_empty() || !root_seen || pending.is_some() {
        return Err(Invalid);
    }
    Ok(doc)
}

/// `areBlanks()` of libxml for blanks right before markup.
fn is_ignorable(parent: &Element, followed_by_end: bool) -> bool {
    if parent.children.is_empty() && followed_by_end {
        return false;
    }
    match parent.children.last() {
        None => true,
        Some(Node::Text(_)) => false,
        Some(_) => !matches!(parent.children.first(), Some(Node::Text(_))),
    }
}

fn close(open: Open, stack: &mut [Open], scopes: &mut Vec<(String, String)>, doc: &mut Document, root_seen: &mut bool) {
    scopes.truncate(scopes.len() - open.scope.len());
    match stack.last_mut() {
        Some(parent) => parent.element.children.push(Node::Element(open.element)),
        None => {
            *root_seen = true;
            doc.children.push(Node::Element(open.element));
        }
    }
}

// ------------------------------------------------------------------ references of `use`

/// `Subject`: an element with an `id`.
struct Subject {
    id: String,
    uid: usize,
    /// The `href` of the element itself.
    href: Option<String>,
    /// `$useCollection`: the subjects it uses, by index, with how often.
    uses: Vec<(usize, u64)>,
}

/// `Resolver::collect()`: the `use` elements to remove; `None` where the library fails (a self reference).
fn resolve(doc: &Document) -> Option<(Vec<Subject>, HashSet<usize>)> {
    let root = doc.children.iter().find_map(|n| if let Node::Element(e) = n { Some(e) } else { None })?;
    // `XPath::handleDefaultNamespace()`: exactly one root named `svg`.
    if root.name.rsplit(':').next() != Some("svg") {
        return None;
    }
    let default_ns = root.namespace.clone();
    let mut subjects: Vec<Subject> = Vec::new();
    let mut elements: Vec<&Element> = Vec::new();
    fn walk<'a>(element: &'a Element, out: &mut Vec<&'a Element>) {
        out.push(element);
        for child in &element.children {
            if let Node::Element(e) = child {
                walk(e, out);
            }
        }
    }
    walk(root, &mut elements);
    for element in &elements {
        if let Some(id) = element.attr("id") {
            let subject = Subject { id: id.to_owned(), uid: element.uid, href: element.href().map(str::to_owned), uses: Vec::new() };
            match subjects.iter().position(|s| s.id == id) {
                Some(index) => subjects[index] = subject,
                None => subjects.push(subject),
            }
        }
    }
    let by_uid: HashMap<usize, &Element> = elements.iter().map(|e| (e.uid, *e)).collect();
    for index in 0..subjects.len() {
        let element = by_uid[&subjects[index].uid];
        for child in &element.children {
            let Node::Element(child) = child else { continue };
            let is_use = match &default_ns {
                Some(ns) => child.namespace.as_deref() == Some(ns.as_str()) && child.name.rsplit(':').next() == Some("use"),
                None => child.name == "use" && child.namespace.is_none(),
            };
            let has_href = child.attr("href").is_some() || child.attrs.iter().any(|a| a.namespace.as_deref() == Some(XLINK) && a.local == "href");
            if !is_use || !has_href {
                continue;
            }
            let Some(target_id) = child.href().and_then(|h| h.strip_prefix('#')) else { continue };
            let Some(target) = subjects.iter().position(|s| s.id == target_id) else { continue };
            if target == index {
                // `Subject::addUse()` throws for a self usage.
                return None;
            }
            match subjects[index].uses.iter_mut().find(|(t, _)| *t == target) {
                Some((_, count)) => *count += 1,
                None => subjects[index].uses.push((target, 1)),
            }
        }
    }
    let mut remove: HashSet<usize> = HashSet::new();
    fn has_infinite_loop(subjects: &[Subject], index: usize, visited: &mut Vec<usize>, level: usize) -> Result<bool, usize> {
        if level > USE_NESTING_LIMIT {
            return Err(subjects[index].uid);
        }
        if visited.contains(&index) {
            return Ok(true);
        }
        visited.push(index);
        for &(target, _) in &subjects[index].uses {
            if has_infinite_loop(subjects, target, &mut visited.clone(), level + 1)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
    for index in 0..subjects.len() {
        // `in_array( $element, $this->elementsToRemove )` compares DOM objects loosely: any element equals any
        // other, so once something is marked every further subject is skipped.
        if !remove.is_empty() {
            continue;
        }
        let own = subjects[index].href.as_deref().and_then(|h| h.strip_prefix('#')).map(str::to_owned);
        let invalid = if own.as_deref() == Some(subjects[index].id.as_str()) {
            true
        } else {
            match has_infinite_loop(&subjects, index, &mut Vec::new(), 1) {
                Ok(looped) => looped,
                Err(uid) => {
                    remove.insert(uid);
                    true
                }
            }
        };
        if invalid {
            let uses = std::mem::take(&mut subjects[index].uses);
            remove.extend(uses.iter().map(|(target, _)| subjects[*target].uid));
        }
    }
    Some((subjects, remove))
}

/// `Subject::countUse()`.
fn count_use(subjects: &[Subject], index: usize) -> u64 {
    subjects[index].uses.iter().map(|&(target, count)| count * count_use(subjects, target).max(1)).sum()
}

// ------------------------------------------------------------------ cleaning

/// `isHrefSafeValue()`.
fn is_href_safe(value: &str) -> bool {
    if value.is_empty() || value == "0" {
        return true;
    }
    if value.starts_with('#') || value.starts_with('/') || value.starts_with("https://") || value.starts_with("http://") {
        return true;
    }
    let images = ["data:image/png", "data:image/gif", "data:image/jpg", "data:image/jpe", "data:image/pjp"];
    let short = ["data:img/png", "data:img/gif", "data:img/jpg", "data:img/jpe", "data:img/pjp"];
    images.iter().any(|p| value.starts_with(p)) || short.iter().any(|p| value.starts_with(p))
}

/// `hasRemoteReference()`: `url( "//…" )` (in quotes) with a remote scheme.
fn has_remote_reference(value: &str) -> bool {
    let printable: String = value.chars().filter(|c| (' '..='~').contains(c)).collect();
    let value = printable.trim_matches([' ', '\t', '\n', '\r', '\0', '\x0B']);
    let lower = value.to_ascii_lowercase();
    let Some(inner) = lower.strip_prefix("url(").and_then(|r| r.strip_suffix(')')) else { return false };
    let inner = inner.trim_matches(|c: char| c.is_ascii_whitespace());
    let quoted = (inner.starts_with('\'') || inner.starts_with('"')) && (inner.ends_with('\'') || inner.ends_with('"')) && inner.len() >= 2;
    if !quoted {
        return false;
    }
    let target = inner[1..inner.len() - 1].trim_matches(|c: char| c.is_ascii_whitespace()).trim_matches(['\'', '"']);
    let after_scheme = ["https:", "http:", "ftp:", "file:"].iter().find_map(|s| target.strip_prefix(s)).unwrap_or(target);
    after_scheme.starts_with("//")
}

struct Cleaner<'a> {
    subjects: &'a [Subject],
    remove_uses: &'a HashSet<usize>,
}

impl Cleaner<'_> {
    /// `startClean()` (with `cleanUnsafeNodes()`): children from the last to the first.
    fn clean(&self, children: &mut Vec<Node>) {
        let mut i = children.len();
        while i > 0 {
            i -= 1;
            let keep = match &mut children[i] {
                Node::Element(element) => self.clean_element(element),
                Node::Text(_) => true,
                Node::Cdata(text) => {
                    let text = std::mem::take(text);
                    children[i] = Node::Text(text);
                    true
                }
                Node::Other => false,
            };
            if !keep {
                children.remove(i);
            }
        }
    }

    /// Whether the element stays; cleans it and what it contains.
    fn clean_element(&self, element: &mut Element) -> bool {
        let lower = element.name.to_lowercase();
        // The same loose `in_array()`: with anything marked, every `use` goes.
        if !self.remove_uses.is_empty() && element.name == "use" {
            return false;
        }
        if !TAGS.contains(&lower.as_str()) {
            return false;
        }
        self.clean_hrefs(element, "");
        // `cleanXlinkHrefs()`: an `xlink:href` whose prefix is not bound goes.
        element.attrs.retain(|a| !(a.prefix.is_empty() && a.local.to_lowercase() == "xlink:href"));
        self.clean_hrefs(element, "xlink");
        // `cleanAttributesOnWhitelist()`, from the last attribute to the first.
        let mut x = element.attrs.len();
        while x > 0 {
            x -= 1;
            let name = element.attrs[x].name();
            let lower = name.to_lowercase();
            let mut removed = false;
            if !ATTRIBUTES.contains(&lower.as_str()) && !lower.starts_with("aria-") && !lower.starts_with("data-") {
                element.remove_attr(&name);
                removed = true;
            }
            if !removed && lower.contains("href") && !is_href_safe(&element.attrs[x].value) {
                element.remove_attr(&name);
                removed = true;
            }
            if !removed && has_remote_reference(&element.attrs[x].value) {
                element.remove_attr(&name);
            }
        }
        if lower == "use" {
            let dirty = element.href().is_some_and(|h| !h.is_empty() && !h.starts_with('#'));
            let exceeding = element.href().and_then(|h| h.strip_prefix('#')).is_some_and(|id| self.subjects.iter().enumerate().any(|(i, s)| s.id == id && count_use(self.subjects, i) >= USE_THRESHOLD));
            if dirty || exceeding {
                return false;
            }
        }
        if lower == "font" && element.attrs.iter().any(|a| matches!(a.name().to_lowercase().as_str(), "face" | "color" | "size")) {
            return false;
        }
        self.clean(&mut element.children);
        true
    }

    /// `cleanHrefAttributes( $element, $prefix )`.
    fn clean_hrefs(&self, element: &mut Element, prefix: &str) {
        let relevant: Vec<Attr> = element.attrs.iter().filter(|a| a.local.to_lowercase() == "href" && a.prefix.to_lowercase() == prefix).cloned().collect();
        for attr in relevant {
            let name = attr.name();
            if !is_href_safe(&attr.value) {
                element.remove_attr(&name);
                continue;
            }
            let lower = name.to_lowercase();
            if !ATTRIBUTES.contains(&name.as_str()) && ATTRIBUTES.contains(&lower.as_str()) {
                element.remove_attr(&name);
                let (new_prefix, new_local) = match lower.split_once(':') {
                    Some((p, l)) if !attr.prefix.is_empty() => (p.to_owned(), l.to_owned()),
                    _ => (String::new(), lower.clone()),
                };
                element.attrs.push(Attr { prefix: new_prefix, local: new_local, namespace: attr.namespace.clone(), value: attr.value.clone(), written: attr.written.clone() });
            }
        }
    }
}

// ------------------------------------------------------------------ output

/// `xmlEscapeContent()`.
fn escape_text(text: &str, out: &mut String) {
    for c in text.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '\r' => out.push_str("&#13;"),
            c => out.push(c),
        }
    }
}

/// `xmlBufAttrSerializeTxtContent()`.
fn escape_attribute(value: &str, out: &mut String) {
    for c in value.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            '\t' => out.push_str("&#9;"),
            c => out.push(c),
        }
    }
}

/// `DOMDocument::saveXML()`: PHP gives every loaded document the encoding UTF-8.
struct Writer;

impl Writer {
    /// `xmlNodeDumpOutputInternal()` with `LIBXML_NOEMPTYTAG`.
    fn element(&self, element: &Element, level: usize, format: bool, out: &mut String) {
        out.push('<');
        out.push_str(&element.name);
        for (name, written) in &element.ns_decls {
            out.push_str(&format!(" {name}=\"{written}\""));
        }
        for attr in &element.attrs {
            out.push_str(&format!(" {}=\"", attr.name()));
            match &attr.written {
                Some(written) => out.push_str(written),
                None => escape_attribute(&attr.value, out),
            }
            out.push('"');
        }
        out.push('>');
        // Text among the children turns the formatting off below this element.
        let format = format && !element.children.iter().any(|c| matches!(c, Node::Text(_)));
        if !element.children.is_empty() {
            if format {
                out.push('\n');
            }
            for child in &element.children {
                match child {
                    Node::Element(e) => {
                        if format {
                            out.push_str(&"  ".repeat(level + 1));
                        }
                        self.element(e, level + 1, format, out);
                    }
                    Node::Text(text) => escape_text(text, out),
                    Node::Cdata(_) | Node::Other => {}
                }
                if format {
                    out.push('\n');
                }
            }
            if format {
                out.push_str(&"  ".repeat(level));
            }
        }
        out.push_str(&format!("</{}>", element.name));
    }

    fn document(&self, doc: &Document, format: bool) -> String {
        let mut out = format!("<?xml version=\"{}\" encoding=\"UTF-8\"", doc.version);
        match doc.standalone {
            Some(true) => out.push_str(" standalone=\"yes\""),
            Some(false) => out.push_str(" standalone=\"no\""),
            None => {}
        }
        out.push_str("?>\n");
        for child in &doc.children {
            if let Node::Element(element) = child {
                self.element(element, 0, format, &mut out);
                out.push('\n');
            }
        }
        out
    }
}

/// `<?php … ?>` and `<?= … ?>` on one line, removed until none is left.
fn strip_php(dirty: &str) -> String {
    let pattern = regex::Regex::new(r"(?i)<\?(=|php)(.+?)\?>").expect("valid pattern");
    let mut text = dirty.to_owned();
    while pattern.is_match(&text) {
        text = pattern.replace_all(&text, "").into_owned();
    }
    text
}

/// `Sanitizer::sanitize()` with `removeRemoteReferences( true )`; `None` when the file cannot be loaded (or the
/// library would fail on it).
pub fn sanitize(dirty: &str, minify: bool) -> Option<String> {
    if dirty.is_empty() {
        return Some(String::new());
    }
    let text = strip_php(dirty).replace("\r\n", "\n").replace('\r', "\n");
    let mut doc = parse(&text).ok()?;
    let (subjects, remove_uses) = resolve(&doc)?;
    let cleaner = Cleaner { subjects: &subjects, remove_uses: &remove_uses };
    cleaner.clean(&mut doc.children);
    let clean = Writer.document(&doc, !minify);
    if minify {
        let mut out = String::with_capacity(clean.len());
        let mut in_space = false;
        for c in clean.chars() {
            if matches!(c, ' ' | '\t' | '\n' | '\x0B' | '\x0C' | '\r') {
                if !in_space {
                    out.push(' ');
                }
                in_space = true;
            } else {
                out.push(c);
                in_space = false;
            }
        }
        return Some(out);
    }
    Some(clean)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_and_handlers_go() {
        let dirty = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- c -->\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\" onload=\"alert(1)\"><script>alert(2)</script>\n  <rect x=\"1\" y=\"2\"/>\n  <a href=\"javascript:alert(3)\"><path d=\"M0 0\"/></a>\n</svg>\n";
        assert_eq!(
            sanitize(dirty, false).as_deref(),
            Some("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\">\n  <rect x=\"1\" y=\"2\"></rect>\n  <a>\n    <path d=\"M0 0\"></path>\n  </a>\n</svg>\n")
        );
    }

    #[test]
    fn bad_documents_are_refused() {
        assert_eq!(sanitize("<svg><g></svg>", false), None);
        assert_eq!(sanitize("<svg>&nbsp;</svg>", false), None);
        assert_eq!(sanitize("<html/>", false), None);
    }

    /// The cases of `tests/data/svg-sanitize` against what the PHP library made of them (`dev/svg-sanitize-golden.sh`).
    #[test]
    fn golden_cases() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("svg-sanitize");
        let mut cases = 0;
        let mut failures = Vec::new();
        for entry in std::fs::read_dir(&dir).expect("golden directory") {
            let path = entry.expect("entry").path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix(".in.svg")).map(str::to_owned) else { continue };
            let dirty = std::fs::read_to_string(&path).expect("input");
            for (minify, suffix) in [(false, "out"), (true, "min")] {
                let expected = std::fs::read_to_string(dir.join(format!("{name}.{suffix}"))).expect("expected output");
                // The PHP script writes `FALSE` for a file the library refuses.
                let expected = if expected == "FALSE" { None } else { Some(expected) };
                cases += 1;
                let actual = sanitize(&dirty, minify);
                if actual != expected {
                    failures.push(format!("{name}.{suffix}:\n  expected {expected:?}\n  actual   {actual:?}"));
                }
            }
        }
        assert!(cases > 0, "no golden cases");
        assert!(failures.is_empty(), "{} of {cases} differ:\n{}", failures.len(), failures.join("\n"));
    }
}
