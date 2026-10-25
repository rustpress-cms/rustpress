//! `WP_Scripts`: registered scripts, their inline data and how they are printed, concatenated into
//! `load-scripts.php` requests where WordPress does that.

use rustpress_core::canonical::rawurlencode;
use rustpress_core::formatting::esc_url_raw;
use serde_json::Value;

use crate::deps::{Arg, Dependencies, Extra, Kind, Ver};
use crate::tags::{Attr, inline_script_tag, script_tag};

/// `load_script_textdomain()`: the JSON of a script's translations, if the language has any.
pub type TranslationLoader = Box<dyn Fn(&str, &str, &str) -> Option<String> + Send + Sync>;

pub struct Scripts {
    pub deps: Dependencies,
    pub base_url: String,
    pub content_url: String,
    pub default_version: String,
    pub default_dirs: Vec<String>,
    /// Whether the active theme (or the admin) prints boolean attributes bare; always true here.
    in_footer: Vec<String>,
    concat: String,
    concat_version: String,
    do_concat: bool,
    print_html: String,
    print_code: String,
    /// `can_compress_scripts`: `c=1` in the `load-scripts.php` address.
    pub compress: bool,
    out: String,
    translations: Option<TranslationLoader>,
}

impl Scripts {
    pub fn new(base_url: &str, content_url: &str, default_version: &str) -> Self {
        Self {
            deps: Dependencies::new(Kind::Scripts),
            base_url: base_url.to_owned(),
            content_url: content_url.to_owned(),
            default_version: default_version.to_owned(),
            default_dirs: vec!["/wp-admin/js/".to_owned(), "/wp-includes/js/".to_owned()],
            in_footer: Vec::new(),
            concat: String::new(),
            concat_version: String::new(),
            do_concat: false,
            print_html: String::new(),
            print_code: String::new(),
            compress: false,
            out: String::new(),
            translations: None,
        }
    }

    pub fn set_translation_loader(&mut self, loader: TranslationLoader) {
        self.translations = Some(loader);
    }

    // ------------------------------------------------------------------ registration

    /// `$scripts->add()`.
    pub fn add(&mut self, handle: &str, src: Option<&str>, deps: &[&str], ver: Ver, args: Option<Arg>) -> bool {
        self.deps.add(handle, src.map(str::to_owned), deps.iter().map(|d| (*d).to_owned()).collect(), ver, args)
    }

    pub fn enqueue(&mut self, handle: &str) {
        self.deps.enqueue(handle);
    }

    pub fn add_data(&mut self, handle: &str, key: &str, value: Extra) -> bool {
        self.deps.add_data(handle, key, value)
    }

    /// `add_inline_script()`: `position` is `before` or (anything else) `after`.
    pub fn add_inline_script(&mut self, handle: &str, data: &str, position: &str) -> bool {
        if data.is_empty() {
            return false;
        }
        let position = if position == "after" { "after" } else { "before" };
        let mut list = match self.deps.get_data(handle, position) {
            Some(Extra::List(list)) => list.clone(),
            _ => Vec::new(),
        };
        list.push(data.to_owned());
        self.deps.add_data(handle, position, Extra::List(list))
    }

    /// `localize()` with the data already as JSON (`wp_json_encode( $l10n, JSON_HEX_TAG | JSON_UNESCAPED_SLASHES )`).
    pub fn localize(&mut self, handle: &str, object_name: &str, l10n: &Value) -> bool {
        let handle = if handle == "jquery" { "jquery-core" } else { handle };
        // Scalar values lose their HTML entities before they are encoded (`html_entity_decode()`).
        let mut l10n = l10n.clone();
        if let Value::Object(map) = &mut l10n {
            for value in map.values_mut() {
                // Every scalar becomes a string: `(string) $value`.
                match value {
                    Value::String(text) => *text = rustpress_core::formatting::html_entity_decode(text),
                    Value::Number(n) => *value = Value::String(n.to_string()),
                    Value::Bool(b) => *value = Value::String(if *b { "1".into() } else { String::new() }),
                    _ => {}
                }
            }
        }
        let json = serde_json::to_string(&l10n).unwrap_or_default().replace('<', "\\u003C").replace('>', "\\u003E");
        let mut script = format!("var {object_name} = {json};");
        if let Some(Extra::Text(data)) = self.deps.get_data(handle, "data")
            && !data.is_empty()
        {
            script = format!("{data}\n{script}");
        }
        self.deps.add_data(handle, "data", Extra::Text(script))
    }

    /// `set_translations()`: the script needs `wp-i18n` and reads the JSON files of `domain`.
    pub fn set_translations(&mut self, handle: &str, domain: &str, path: &str) -> bool {
        let Some(dep) = self.deps.get_mut(handle) else { return false };
        if !dep.deps.iter().any(|d| d == "wp-i18n") {
            dep.deps.push("wp-i18n".to_owned());
        }
        dep.textdomain = Some(domain.to_owned());
        dep.translations_path = Some(path.to_owned());
        true
    }

    // ------------------------------------------------------------------ printing

    /// `print_head_scripts()`: the scripts of the head, as HTML.
    pub fn print_head_scripts(&mut self, concatenate: bool) -> String {
        self.do_concat = concatenate;
        self.do_items(0);
        self.print_scripts();
        self.reset();
        std::mem::take(&mut self.out)
    }

    /// `print_footer_scripts()`.
    pub fn print_footer_scripts(&mut self, concatenate: bool) -> String {
        self.do_concat = concatenate;
        self.do_items(1);
        self.print_scripts();
        self.reset();
        std::mem::take(&mut self.out)
    }

    fn reset(&mut self) {
        self.do_concat = false;
        self.print_code.clear();
        self.concat.clear();
        self.concat_version.clear();
        self.print_html.clear();
    }

    /// `_print_scripts()`: the `load-scripts.php` element for what was concatenated, then what was held back.
    fn print_scripts(&mut self) {
        let zip = if self.compress { 1 } else { 0 };
        let concat = self.concat.trim_matches([',', ' ']).to_owned();
        if !concat.is_empty() {
            if !self.print_code.is_empty() {
                let code = format!("{}\n//# sourceURL={}", self.print_code, rawurlencode(&format!("js-inline-concat-{concat}")));
                self.out.push_str(&inline_script_tag(&code, &[]));
            }
            let mut chunks = String::new();
            let bytes = concat.as_bytes();
            for (key, chunk) in bytes.chunks(128).enumerate() {
                chunks.push_str(&format!("&load%5Bchunk_{key}%5D={}", String::from_utf8_lossy(chunk)));
            }
            let src = format!("{}/wp-admin/load-scripts.php?c={zip}{chunks}&ver={}", self.base_url, self.default_version);
            self.out.push_str(&script_tag(&[("src", Attr::Text(src))]));
        }
        if !self.print_html.is_empty() {
            let html = std::mem::take(&mut self.print_html);
            self.out.push_str(&html);
        }
    }

    /// `do_items()`: resolves the queue and prints the items of one group (0 head, 1 footer).
    fn do_items(&mut self, group: i64) {
        let queue = self.deps.queue.clone();
        self.deps.all_deps(&queue, false, None);
        let to_do = self.deps.to_do.clone();
        for handle in to_do {
            if !self.deps.done.contains(&handle) && self.deps.is_registered(&handle) && self.do_item(&handle, group) {
                self.deps.done.push(handle.clone());
            }
            self.deps.to_do.retain(|h| h != &handle);
        }
    }

    fn group_of(&self, handle: &str) -> i64 {
        self.deps.groups.get(handle).copied().unwrap_or(0)
    }

    fn is_delayed(strategy: &str) -> bool {
        strategy == "defer" || strategy == "async"
    }

    fn strategy_of(&self, handle: &str) -> String {
        match self.deps.get_data(handle, "strategy") {
            Some(Extra::Text(s)) => s.clone(),
            _ => String::new(),
        }
    }

    fn are_all_dependents_in_footer(&self, handle: &str) -> bool {
        self.deps.dependents(handle).iter().all(|d| self.deps.groups.get(d).is_none_or(|g| *g != 0))
    }

    fn has_inline_script(&self, handle: &str, position: &str) -> bool {
        matches!(self.deps.get_data(handle, position), Some(Extra::List(l)) if !l.is_empty())
    }

    /// `get_eligible_loading_strategy()`.
    fn eligible_loading_strategy(&self, handle: &str) -> String {
        let intended = self.strategy_of(handle);
        if intended.is_empty() {
            return String::new();
        }
        let initial = if intended == "defer" { Some(vec!["defer"]) } else { None };
        let mut stored = std::collections::HashMap::new();
        let eligible = self.filter_eligible_strategies(handle, initial, &mut Vec::new(), &mut stored);
        if eligible.is_empty() {
            String::new()
        } else if eligible.contains(&"async") {
            "async".to_owned()
        } else {
            "defer".to_owned()
        }
    }

    fn filter_eligible_strategies(&self, handle: &str, eligible: Option<Vec<&'static str>>, checked: &mut Vec<String>, stored: &mut std::collections::HashMap<String, Vec<&'static str>>) -> Vec<&'static str> {
        if let Some(result) = stored.get(handle) {
            return result.clone();
        }
        let mut eligible = eligible.unwrap_or_else(|| vec!["defer", "async"]);
        if checked.iter().any(|h| h == handle) {
            return eligible;
        }
        checked.push(handle.to_owned());
        let Some(dep) = self.deps.get(handle) else { return eligible };
        if !self.deps.is_enqueued(handle) {
            return eligible;
        }
        let intended = self.strategy_of(handle);
        if dep.src.is_some() && intended.is_empty() {
            return Vec::new();
        }
        if self.has_inline_script(handle, "after") {
            return Vec::new();
        }
        if intended == "defer" {
            eligible = vec!["defer"];
        }
        for dependent in self.deps.dependents(handle) {
            if eligible.is_empty() {
                return Vec::new();
            }
            eligible = self.filter_eligible_strategies(&dependent, Some(eligible), checked, stored);
        }
        stored.insert(handle.to_owned(), eligible.clone());
        eligible
    }

    /// `get_highest_fetchpriority_with_dependents()`.
    fn highest_fetchpriority(&self, handle: &str, checked: &mut Vec<String>, stored: &mut std::collections::HashMap<String, &'static str>) -> Option<&'static str> {
        const PRIORITIES: [&str; 3] = ["low", "auto", "high"];
        if let Some(result) = stored.get(handle) {
            return Some(result);
        }
        if checked.iter().any(|h| h == handle) {
            return None;
        }
        checked.push(handle.to_owned());
        if !self.deps.is_enqueued(handle) {
            return None;
        }
        let own = match self.deps.get_data(handle, "fetchpriority") {
            Some(Extra::Text(s)) if PRIORITIES.contains(&s.as_str()) => s.clone(),
            _ => "auto".to_owned(),
        };
        let mut highest = PRIORITIES.iter().position(|p| *p == own).unwrap_or(1);
        if highest != PRIORITIES.len() - 1 {
            for dependent in self.deps.dependents(handle) {
                if let Some(priority) = self.highest_fetchpriority(&dependent, checked, stored) {
                    highest = highest.max(PRIORITIES.iter().position(|p| *p == priority).unwrap_or(1));
                    if highest == PRIORITIES.len() - 1 {
                        break;
                    }
                }
            }
        }
        stored.insert(handle.to_owned(), PRIORITIES[highest]);
        Some(PRIORITIES[highest])
    }

    /// `get_inline_script_data()`.
    fn inline_script_data(&self, handle: &str, position: &str) -> String {
        match self.deps.get_data(handle, position) {
            Some(Extra::List(list)) if !list.is_empty() => {
                let mut parts = list.clone();
                parts.push(format!("//# sourceURL={}", rawurlencode(&format!("{handle}-js-{position}"))));
                parts.join("\n").trim_matches('\n').to_owned()
            }
            _ => String::new(),
        }
    }

    /// `get_inline_script_tag()`.
    fn inline_script_tag_of(&self, handle: &str, position: &str) -> String {
        let js = self.inline_script_data(handle, position);
        if js.is_empty() { String::new() } else { inline_script_tag(&js, &[("id", Attr::Text(format!("{handle}-js-{position}")))]) }
    }

    /// `print_extra_script()`: the data of `localize()`. Returns the code when it is not printed.
    fn print_extra_script(&mut self, handle: &str, display: bool) -> Option<String> {
        let Some(Extra::Text(data)) = self.deps.get_data(handle, "data").cloned() else { return None };
        if data.is_empty() {
            return None;
        }
        let mut output = data;
        if !self.do_concat {
            output.push_str(&format!("\n//# sourceURL={}", rawurlencode(&format!("{handle}-js-extra"))));
        }
        if !display {
            return Some(output);
        }
        self.out.push_str(&inline_script_tag(&output, &[("id", Attr::Text(format!("{handle}-js-extra")))]));
        Some(String::new())
    }

    /// `print_translations()` as a tag, or an empty string.
    fn translations_tag(&self, handle: &str) -> String {
        let Some(dep) = self.deps.get(handle) else { return String::new() };
        let Some(domain) = dep.textdomain.as_deref().filter(|d| !d.is_empty()) else { return String::new() };
        let path = dep.translations_path.as_deref().unwrap_or_default();
        let Some(loader) = &self.translations else { return String::new() };
        let Some(json) = loader(handle, domain, path) else { return String::new() };
        let output = format!(
            "( function( domain, translations ) {{\n\tvar localeData = translations.locale_data[ domain ] || translations.locale_data.messages;\n\tlocaleData[\"\"].domain = domain;\n\twp.i18n.setLocaleData( localeData, domain );\n}} )( \"{domain}\", {json} );\n//# sourceURL={}",
            rawurlencode(&format!("{handle}-js-translations"))
        );
        inline_script_tag(&output, &[("id", Attr::Text(format!("{handle}-js-translations")))])
    }

    /// `in_default_dir()`.
    fn in_default_dir(&self, src: &str) -> bool {
        if self.default_dirs.is_empty() {
            return true;
        }
        if src.starts_with("/wp-includes/js/l10n") {
            return false;
        }
        self.default_dirs.iter().any(|d| src.starts_with(d.as_str()))
    }

    /// `do_item()`.
    fn do_item(&mut self, handle: &str, group: i64) -> bool {
        let Some(obj) = self.deps.get(handle).cloned() else { return false };
        let handle_group = self.group_of(handle);
        if group == 0 && handle_group > 0 {
            self.in_footer.push(handle.to_owned());
            return false;
        }
        if obj.extra.get("conditional").is_some_and(Extra::truthy) {
            return false;
        }
        let strategy = self.eligible_loading_strategy(handle);
        let mut intended = self.strategy_of(handle);
        if !Self::is_delayed(&intended) {
            intended.clear();
        }
        if group == 0 && handle_group == 0 && !intended.is_empty() && !Self::is_delayed(&strategy) && self.are_all_dependents_in_footer(handle) {
            self.in_footer.push(handle.to_owned());
            return false;
        }
        let before_script = self.inline_script_tag_of(handle, "before");
        let after_script = self.inline_script_tag_of(handle, "after");
        let inline_tags = format!("{before_script}{after_script}");
        let translations_stop_concat = obj.textdomain.as_deref().is_some_and(|d| !d.is_empty());
        let translations = self.translations_tag(handle);

        let src = obj.src.clone().unwrap_or_default();
        if self.do_concat {
            if obj.src.is_some() && self.in_default_dir(&src) && (!before_script.is_empty() || !after_script.is_empty() || translations_stop_concat || Self::is_delayed(&strategy)) {
                self.do_concat = false;
                self.print_scripts();
                self.reset();
            } else if obj.src.is_some() && self.in_default_dir(&src) {
                if let Some(code) = self.print_extra_script(handle, false) {
                    self.print_code.push_str(&code);
                }
                self.concat.push_str(&format!("{handle},"));
                let ver = self.version_text(&obj.ver, handle);
                self.concat_version.push_str(&format!("{handle}{ver}"));
                return true;
            }
        }
        self.print_extra_script(handle, true);
        if obj.src.is_none() || src.is_empty() {
            if !inline_tags.is_empty() {
                if self.do_concat {
                    self.print_html.push_str(&inline_tags);
                } else {
                    self.out.push_str(&inline_tags);
                }
            }
            return true;
        }
        let mut src = src;
        let absolute = src.starts_with("http://") || src.starts_with("https://") || src.starts_with("//");
        if !absolute && !(!self.content_url.is_empty() && src.starts_with(&self.content_url)) {
            src = format!("{}{src}", self.base_url);
        }
        let ver_to_add = match &obj.ver {
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
        let src = esc_url_raw(&src);
        if src.is_empty() {
            return true;
        }
        let mut attrs: Vec<(&str, Attr)> = vec![("src", Attr::Text(src)), ("id", Attr::Text(format!("{handle}-js")))];
        if !strategy.is_empty() {
            attrs.push((if strategy == "async" { "async" } else { "defer" }, Attr::Flag));
        }
        if !intended.is_empty() {
            attrs.push(("data-wp-strategy", Attr::Text(intended)));
        }
        let original = match obj.extra.get("fetchpriority") {
            Some(Extra::Text(s)) if matches!(s.as_str(), "auto" | "low" | "high") => s.clone(),
            _ => "auto".to_owned(),
        };
        let actual = self.highest_fetchpriority(handle, &mut Vec::new(), &mut std::collections::HashMap::new()).map_or_else(|| original.clone(), str::to_owned);
        if actual != "auto" {
            attrs.push(("fetchpriority", Attr::Text(actual.clone())));
        }
        if original != actual {
            attrs.push(("data-wp-fetchpriority", Attr::Text(original)));
        }
        let tag = format!("{translations}{before_script}{}{after_script}", script_tag(&attrs));
        if self.do_concat {
            self.print_html.push_str(&tag);
        } else {
            self.out.push_str(&tag);
        }
        true
    }

    /// The `$ver` of `do_item()`: used for the concat version only.
    fn version_text(&self, ver: &Ver, handle: &str) -> String {
        let mut text = match ver {
            Ver::None => String::new(),
            Ver::Value(v) if !v.is_empty() && v != "0" => v.clone(),
            _ => self.default_version.clone(),
        };
        if let Some(args) = self.deps.args.get(handle) {
            text = if text.is_empty() { args.clone() } else { format!("{text}&amp;{args}") };
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scripts() -> Scripts {
        let mut s = Scripts::new("http://example.org", "", "7.1");
        s.add("jquery-core", Some("/wp-includes/js/jquery/jquery.min.js"), &[], Ver::Value("3.7.1".into()), Some(Arg::Int(1)));
        s.add("jquery", None, &["jquery-core"], Ver::Value("3.7.1".into()), None);
        s.add("hooks", Some("/wp-includes/js/dist/hooks.min.js"), &[], Ver::Value("abc".into()), None);
        s.add("theme", Some("/wp-content/themes/x/theme.js"), &["jquery"], Ver::Default, None);
        s
    }

    #[test]
    fn concatenated_scripts_are_one_request() {
        let mut s = scripts();
        s.enqueue("jquery");
        s.enqueue("hooks");
        let head = s.print_head_scripts(true);
        // The alias `jquery` is a head item, so its footer script `jquery-core` is pulled into the head.
        assert!(head.contains("load%5Bchunk_0%5D=jquery-core,hooks&#038;ver=7.1"), "{head}");
        assert_eq!(head.matches("<script").count(), 1);
        assert_eq!(s.print_footer_scripts(true), "");
    }

    #[test]
    fn single_scripts_get_id_and_version() {
        let mut s = scripts();
        s.enqueue("theme");
        let footer = s.print_footer_scripts(false);
        assert!(footer.contains("<script id=\"jquery-core-js\" src=\"http://example.org/wp-includes/js/jquery/jquery.min.js?ver=3.7.1\"></script>"), "{footer}");
        assert!(footer.contains("<script id=\"theme-js\" src=\"http://example.org/wp-content/themes/x/theme.js?ver=7.1\"></script>"), "{footer}");
        // The dependency comes first.
        assert!(footer.find("jquery-core-js").unwrap() < footer.find("theme-js").unwrap());
    }

    #[test]
    fn localized_data_is_printed_before_the_script() {
        let mut s = scripts();
        s.localize("theme", "themeL10n", &serde_json::json!({"a": "x<y"}));
        s.enqueue("theme");
        let out = s.print_footer_scripts(false);
        assert!(out.contains("<script id=\"theme-js-extra\">\nvar themeL10n = {\"a\":\"x\\u003Cy\"};\n//# sourceURL=theme-js-extra\n</script>"), "{out}");
    }
}
