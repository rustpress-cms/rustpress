//! `WP_Styles`: registered stylesheets, inline styles and how they are printed.

use rustpress_core::canonical::rawurlencode;
use rustpress_core::formatting::{esc_attr, esc_url};

use crate::deps::{Arg, Dependencies, Extra, Kind, Ver};
use crate::tags::inline_style_tag;

pub struct Styles {
    pub deps: Dependencies,
    pub base_url: String,
    pub content_url: String,
    pub default_version: String,
    pub text_direction: String,
    pub default_dirs: Vec<String>,
    /// `can_compress_scripts`: `c=1` in the `load-styles.php` address.
    pub compress: bool,
    concat: String,
    do_concat: bool,
    print_html: String,
    print_code: String,
    out: String,
}

impl Styles {
    pub fn new(base_url: &str, content_url: &str, default_version: &str, rtl: bool) -> Self {
        Self {
            deps: Dependencies::new(Kind::Styles),
            base_url: base_url.to_owned(),
            content_url: content_url.to_owned(),
            default_version: default_version.to_owned(),
            text_direction: if rtl { "rtl" } else { "ltr" }.to_owned(),
            default_dirs: vec!["/wp-admin/".to_owned(), "/wp-includes/css/".to_owned()],
            compress: false,
            concat: String::new(),
            do_concat: false,
            print_html: String::new(),
            print_code: String::new(),
            out: String::new(),
        }
    }

    /// `$styles->add()`.
    pub fn add(&mut self, handle: &str, src: Option<&str>, deps: &[&str], ver: Ver, media: Option<&str>) -> bool {
        self.deps.add(handle, src.map(str::to_owned), deps.iter().map(|d| (*d).to_owned()).collect(), ver, media.map(|m| Arg::Text(m.to_owned())))
    }

    pub fn enqueue(&mut self, handle: &str) {
        self.deps.enqueue(handle);
    }

    pub fn add_data(&mut self, handle: &str, key: &str, value: Extra) -> bool {
        self.deps.add_data(handle, key, value)
    }

    /// `add_inline_style()`.
    pub fn add_inline_style(&mut self, handle: &str, code: &str) -> bool {
        if code.is_empty() {
            return false;
        }
        let mut list = match self.deps.get_data(handle, "after") {
            Some(Extra::List(list)) => list.clone(),
            _ => Vec::new(),
        };
        list.push(code.to_owned());
        self.deps.add_data(handle, "after", Extra::List(list))
    }

    /// `print_admin_styles()`: everything queued, as HTML.
    pub fn print_admin_styles(&mut self, concatenate: bool) -> String {
        self.do_concat = concatenate;
        self.do_items();
        self.print_styles();
        self.reset();
        std::mem::take(&mut self.out)
    }

    fn reset(&mut self) {
        self.do_concat = false;
        self.concat.clear();
        self.print_html.clear();
        self.print_code.clear();
    }

    /// `_print_styles()`.
    fn print_styles(&mut self) {
        let zip = if self.compress { 1 } else { 0 };
        let concat = self.concat.trim_matches([',', ' ']).to_owned();
        if !concat.is_empty() {
            let source_url = format!("css-inline-concat-{concat}");
            let mut chunks = String::new();
            for (key, chunk) in concat.as_bytes().chunks(128).enumerate() {
                chunks.push_str(&format!("&load%5Bchunk_{key}%5D={}", String::from_utf8_lossy(chunk)));
            }
            let href = format!("{}/wp-admin/load-styles.php?c={zip}&dir={}{chunks}&ver={}", self.base_url, self.text_direction, self.default_version);
            self.out.push_str(&format!("<link rel='stylesheet' href='{}' media='all' />\n", esc_attr(&href)));
            if !self.print_code.is_empty() {
                let contents = format!("\n{}\n/*# sourceURL={} */\n", self.print_code, rawurlencode(&source_url));
                self.out.push_str(&format!("<style>{contents}</style>\n"));
            }
        }
        if !self.print_html.is_empty() {
            let html = std::mem::take(&mut self.print_html);
            self.out.push_str(&html);
        }
    }

    fn do_items(&mut self) {
        let queue = self.deps.queue.clone();
        self.deps.all_deps(&queue, false, None);
        let to_do = self.deps.to_do.clone();
        for handle in to_do {
            if !self.deps.done.contains(&handle) && self.deps.is_registered(&handle) && self.do_item(&handle) {
                self.deps.done.push(handle.clone());
            }
            self.deps.to_do.retain(|h| h != &handle);
        }
    }

    fn in_default_dir(&self, src: &str) -> bool {
        self.default_dirs.is_empty() || self.default_dirs.iter().any(|d| src.starts_with(d.as_str()))
    }

    /// `print_inline_style( $handle, false )`.
    fn inline_style_text(&self, handle: &str) -> Option<String> {
        let Some(Extra::List(list)) = self.deps.get_data(handle, "after") else { return None };
        if list.is_empty() {
            return None;
        }
        let mut output = list.clone();
        if !self.do_concat {
            let source_url = match self.deps.get_data(handle, "inlined_src") {
                Some(Extra::Text(src)) if list.len() == 1 && !src.is_empty() => rustpress_core::formatting::esc_url_raw(src),
                _ => rawurlencode(&format!("{handle}-inline-css")),
            };
            output.push(format!("/*# sourceURL={source_url} */"));
        }
        Some(output.join("\n"))
    }

    /// `_css_href()`.
    fn css_href(&self, src: &str, ver: &Ver, handle: &str) -> String {
        let mut src = src.to_owned();
        let absolute = src.starts_with("http://") || src.starts_with("https://") || src.starts_with("//");
        if !absolute && !(!self.content_url.is_empty() && src.starts_with(&self.content_url)) {
            src = format!("{}{src}", self.base_url);
        }
        let ver_to_add = match ver {
            Ver::Default => self.default_version.clone(),
            Ver::None => String::new(),
            Ver::Value(v) if v.is_empty() => self.default_version.clone(),
            Ver::Value(v) => v.clone(),
        };
        let added_args = self.deps.args.get(handle).cloned().unwrap_or_default();
        if !ver_to_add.is_empty() || !added_args.is_empty() {
            let fragment = src.find('#').map(|i| src[i..].to_owned());
            if let Some(fragment) = &fragment {
                src.truncate(src.len() - fragment.len());
            }
            if !ver_to_add.is_empty() {
                src.push_str(&format!("{}ver={}", if src.contains('?') { '&' } else { '?' }, rawurlencode(&ver_to_add)));
            }
            if !added_args.is_empty() {
                src.push_str(&format!("{}{added_args}", if src.contains('?') { '&' } else { '?' }));
            }
            if let Some(fragment) = fragment {
                src.push_str(&fragment);
            }
        }
        esc_url(&src)
    }

    fn do_item(&mut self, handle: &str) -> bool {
        let Some(obj) = self.deps.get(handle).cloned() else { return false };
        if obj.extra.get("conditional").is_some_and(Extra::truthy) {
            return false;
        }
        let inline_style = self.inline_style_text(handle);
        let inline_tag = inline_style.as_ref().map(|css| format!("{}\n", inline_style_tag(&format!("{handle}-inline-css"), css))).unwrap_or_default();
        let src = obj.src.clone();
        if self.do_concat
            && let Some(src) = &src
            && self.in_default_dir(src)
            && !obj.extra.contains_key("alt")
        {
            self.concat.push_str(&format!("{handle},"));
            if let Some(css) = &inline_style {
                self.print_code.push_str(css);
            }
            return true;
        }
        let media = match &obj.args {
            Some(Arg::Text(m)) => m.clone(),
            _ => "all".to_owned(),
        };
        let Some(src) = src else {
            if !inline_tag.is_empty() {
                if self.do_concat {
                    self.print_html.push_str(&inline_tag);
                } else {
                    self.out.push_str(&inline_tag);
                }
            }
            return true;
        };
        let href = self.css_href(&src, &obj.ver, handle);
        if href.is_empty() {
            return true;
        }
        let alternate = obj.extra.get("alt").is_some_and(Extra::truthy);
        let rel = if alternate { "alternate stylesheet" } else { "stylesheet" };
        let title = match obj.extra.get("title") {
            Some(Extra::Text(t)) if !t.is_empty() => format!(" title='{}'", esc_attr(t)),
            _ => String::new(),
        };
        let mut tag = format!("<link rel='{rel}' id='{}-css'{title} href='{href}' media='{}' />\n", esc_attr(handle), esc_attr(&media));
        if self.text_direction == "rtl"
            && let Some(rtl) = obj.extra.get("rtl").filter(|r| r.truthy())
        {
            let replace = matches!(rtl, Extra::Bool(true)) || rtl.text() == Some("replace");
            let rtl_href = if replace {
                let suffix = obj.extra.get("suffix").and_then(Extra::text).unwrap_or_default();
                self.css_href(&src, &obj.ver, &format!("{handle}-rtl")).replace(&format!("{suffix}.css"), &format!("-rtl{suffix}.css"))
            } else {
                self.css_href(rtl.text().unwrap_or_default(), &obj.ver, &format!("{handle}-rtl"))
            };
            let rtl_tag = format!("<link rel='{rel}' id='{}-rtl-css'{title} href='{rtl_href}' media='{}' />\n", esc_attr(handle), esc_attr(&media));
            if rtl.text() == Some("replace") {
                tag = rtl_tag;
            } else {
                tag.push_str(&rtl_tag);
            }
        }
        if self.do_concat {
            self.print_html.push_str(&tag);
            self.print_html.push_str(&inline_tag);
        } else {
            self.out.push_str(&tag);
            if let Some(css) = &inline_style {
                self.out.push_str(&format!("{}\n", inline_style_tag(&format!("{handle}-inline-css"), css)));
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn styles() -> Styles {
        let mut s = Styles::new("http://example.org", "http://example.org/wp-content", "7.1", false);
        s.add("dashicons", Some("/wp-includes/css/dashicons.min.css"), &[], Ver::Default, None);
        s.add("login", Some("/wp-admin/css/login.min.css"), &["dashicons"], Ver::Default, None);
        s.add("theme", Some("/wp-content/themes/x/style.css"), &["login"], Ver::Value("1.2".into()), None);
        s
    }

    #[test]
    fn default_directory_styles_are_concatenated() {
        let mut s = styles();
        s.enqueue("login");
        let out = s.print_admin_styles(true);
        assert_eq!(out, "<link rel='stylesheet' href='http://example.org/wp-admin/load-styles.php?c=0&amp;dir=ltr&amp;load%5Bchunk_0%5D=dashicons,login&amp;ver=7.1' media='all' />\n");
    }

    #[test]
    fn other_styles_are_linked_with_their_version() {
        let mut s = styles();
        s.enqueue("theme");
        let out = s.print_admin_styles(true);
        assert!(out.contains("load%5Bchunk_0%5D=dashicons,login"), "{out}");
        assert!(out.contains("<link rel='stylesheet' id='theme-css' href='http://example.org/wp-content/themes/x/style.css?ver=1.2' media='all' />\n"), "{out}");
    }

    #[test]
    fn inline_styles_follow_their_stylesheet() {
        let mut s = styles();
        s.add_inline_style("theme", "a{color:red}");
        s.enqueue("theme");
        let out = s.print_admin_styles(false);
        assert!(out.ends_with("<style id=\"theme-inline-css\">\na{color:red}\n/*# sourceURL=theme-inline-css */\n</style>\n"), "{out}");
    }
}
