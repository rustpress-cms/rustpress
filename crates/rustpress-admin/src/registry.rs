//! The scripts and styles WordPress registers by default, read from `wp-includes/script-loader.php` and
//! `wp-includes/assets/script-loader-packages.php`, so that a different WordPress version brings its
//! own list.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use regex::Regex;
use rustpress_core::php_lit::{Hooks, Lit, Parser, Vars};
use serde_json::Value;

use crate::deps::{Arg, Extra, Ver};
use crate::scripts::Scripts;
use crate::styles::Styles;

/// What the registration code asks the site about.
#[derive(Debug, Clone)]
pub struct LoaderEnv {
    /// The `wp-includes` directory of the WordPress files.
    pub includes: PathBuf,
    /// `site_url()`, without a trailing slash.
    pub site_url: String,
    pub content_url: String,
    /// `get_bloginfo( 'version' )`.
    pub version: String,
    pub rtl: bool,
    /// `wp_should_load_separate_core_block_assets()`: true on the frontend of a block theme, never in the admin.
    pub separate_block_assets: bool,
    /// `$concatenate_scripts && $compress_scripts`: TinyMCE comes as one compressed file then.
    pub compressed_tinymce: bool,
}

/// Lit -> JSON the way PHP arrays encode: a list when the keys are 0..n, else an object.
pub fn lit_to_json(lit: &Lit) -> Value {
    match lit {
        Lit::Null | Lit::Unknown => Value::Null,
        Lit::Bool(b) => Value::Bool(*b),
        Lit::Int(i) => Value::from(*i),
        Lit::Float(f) => Value::from(*f),
        Lit::Str(s) => Value::String(s.clone()),
        Lit::Array(items) => {
            if items.iter().all(|(key, _)| key.is_none()) {
                Value::Array(items.iter().map(|(_, v)| lit_to_json(v)).collect())
            } else {
                let mut map = serde_json::Map::new();
                let mut next = 0i64;
                for (key, value) in items {
                    let name = match key {
                        Some(Lit::Str(s)) => s.clone(),
                        Some(Lit::Int(i)) => {
                            next = next.max(*i + 1);
                            i.to_string()
                        }
                        _ => {
                            next += 1;
                            (next - 1).to_string()
                        }
                    };
                    map.insert(name, lit_to_json(value));
                }
                Value::Object(map)
            }
        }
    }
}

fn strings(lit: &Lit) -> Vec<String> {
    lit.strings()
}

/// The text of `function name( ... ) { ... }` in a PHP file (its braces matched).
pub fn function_body<'a>(php: &'a str, name: &str) -> Option<&'a str> {
    let start = php.find(&format!("function {name}("))?;
    let open = start + php[start..].find('{')?;
    let bytes = php.as_bytes();
    let mut depth = 0i32;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&php[open + 1..i]);
                }
            }
            b'\'' | b'"' => {
                let quote = bytes[i];
                i += 1;
                while i < bytes.len() && bytes[i] != quote {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// One call or assignment found in a function body.
enum Statement {
    Assign(String, usize),
    Call { method: String, args_at: usize },
}

/// The statements of a body in source order.
fn statements(body: &str) -> Vec<(usize, Statement)> {
    let call = Regex::new(r"\$(scripts|styles)->(add|add_data|add_inline_script|add_inline_style|localize|set_translations)\s*\(").expect("valid pattern");
    let assign = Regex::new(r"(?m)^[ \t]*\$(\w+)\s*=\s*").expect("valid pattern");
    let mut found: Vec<(usize, Statement)> = Vec::new();
    for m in call.captures_iter(body) {
        let whole = m.get(0).expect("whole match");
        found.push((whole.start(), Statement::Call { method: m[2].to_owned(), args_at: whole.end() }));
    }
    for m in assign.captures_iter(body) {
        let whole = m.get(0).expect("whole match");
        // `==`, `=>` and `.=` are not assignments.
        if body[whole.end()..].starts_with('=') || body[whole.start()..whole.end()].contains("=>") {
            continue;
        }
        found.push((whole.start(), Statement::Assign(m[1].to_owned(), whole.end())));
    }
    found.sort_by_key(|(at, _)| *at);
    found
}

fn hooks(env: &LoaderEnv) -> Hooks {
    let mut h = Hooks::default();
    let site = env.site_url.clone();
    let content = env.content_url.clone();
    let version = env.version.clone();
    h.functions.insert("site_url".into(), Box::new({
        let site = site.clone();
        move |args| Some(Lit::Str(format!("{site}{}", args.first().and_then(Lit::as_str).map(|p| if p.starts_with('/') { p.to_owned() } else { format!("/{p}") }).unwrap_or_default())))
    }));
    h.functions.insert("admin_url".into(), Box::new({
        let site = site.clone();
        move |args| Some(Lit::Str(format!("{site}/wp-admin/{}", args.first().and_then(Lit::as_str).unwrap_or_default())))
    }));
    h.functions.insert("includes_url".into(), Box::new({
        let site = site.clone();
        move |args| Some(Lit::Str(format!("{site}/wp-includes/{}", args.first().and_then(Lit::as_str).unwrap_or_default().trim_start_matches('/'))))
    }));
    h.functions.insert("content_url".into(), Box::new({
        let content = content.clone();
        move |args| Some(Lit::Str(format!("{content}/{}", args.first().and_then(Lit::as_str).unwrap_or_default().trim_start_matches('/'))))
    }));
    h.functions.insert("get_bloginfo".into(), Box::new(move |args| match args.first().and_then(Lit::as_str) {
        Some("version") => Some(Lit::Str(version.clone())),
        _ => None,
    }));
    h.functions.insert("wp_scripts_get_suffix".into(), Box::new(|_| Some(Lit::Str(".min".into()))));
    h.functions.insert("is_admin".into(), Box::new(|_| Some(Lit::Bool(false))));
    h.functions.insert("urlencode".into(), Box::new(|args| args.first().and_then(Lit::as_str).map(|t| Lit::Str(rustpress_core::canonical::php_urlencode(t)))));
    h.props.insert("const::WPINC".into(), Lit::Str("wp-includes".into()));
    h.props.insert("const::ABSPATH".into(), Lit::Str("/var/www/html/".into()));
    h.props.insert("scripts->base_url".into(), Lit::Str(site.clone()));
    h.props.insert("styles->base_url".into(), Lit::Str(site));
    h.props.insert("scripts->content_url".into(), Lit::Str(content.clone()));
    h.props.insert("styles->content_url".into(), Lit::Str(content));
    h
}

/// Which registry a body fills.
enum Target<'a> {
    Scripts(&'a mut Scripts),
    Styles(&'a mut Styles),
}

fn ver_of(lit: Option<&Lit>) -> Ver {
    match lit {
        None | Some(Lit::Bool(false)) => Ver::Default,
        Some(Lit::Null) => Ver::None,
        Some(Lit::Str(s)) => Ver::Value(s.clone()),
        Some(Lit::Int(i)) => Ver::Value(i.to_string()),
        Some(Lit::Float(f)) => Ver::Value(f.to_string()),
        _ => Ver::Default,
    }
}

fn extra_of(lit: &Lit) -> Option<Extra> {
    match lit {
        Lit::Str(s) => Some(Extra::Text(s.clone())),
        Lit::Bool(b) => Some(Extra::Bool(*b)),
        Lit::Int(i) => Some(Extra::Int(*i)),
        Lit::Array(_) => Some(Extra::List(strings(lit))),
        _ => None,
    }
}

/// Runs the registration calls of one function body.
fn run_body(body: &str, vars: &mut Vars, env: &LoaderEnv, mut target: Target<'_>) {
    let hooks = hooks(env);
    for (_, statement) in statements(body) {
        match statement {
            Statement::Assign(name, at) => {
                let value = Parser::new(body, at, vars).with_hooks(&hooks).expr();
                if value != Lit::Unknown {
                    vars.insert(name, value);
                }
            }
            Statement::Call { method, args_at, .. } => {
                let args = Parser::new(body, args_at, vars).with_hooks(&hooks).call_args();
                let Some(handle) = args.first().and_then(Lit::as_str).map(str::to_owned) else { continue };
                match (&mut target, method.as_str()) {
                    (Target::Scripts(s), "add") => {
                        let src = args.get(1).and_then(Lit::as_str);
                        let deps: Vec<String> = args.get(2).map(strings).unwrap_or_default();
                        let deps_refs: Vec<&str> = deps.iter().map(String::as_str).collect();
                        let arg = match args.get(4) {
                            Some(Lit::Int(i)) => Some(Arg::Int(*i)),
                            Some(Lit::Str(t)) => Some(Arg::Text(t.clone())),
                            _ => None,
                        };
                        s.add(&handle, src, &deps_refs, ver_of(args.get(3)), arg);
                    }
                    (Target::Styles(s), "add") => {
                        let src = args.get(1).and_then(Lit::as_str);
                        let deps: Vec<String> = args.get(2).map(strings).unwrap_or_default();
                        let deps_refs: Vec<&str> = deps.iter().map(String::as_str).collect();
                        let media = args.get(4).and_then(Lit::as_str);
                        s.add(&handle, src, &deps_refs, ver_of(args.get(3)), media);
                    }
                    (Target::Scripts(s), "add_data") => {
                        if let (Some(key), Some(value)) = (args.get(1).and_then(Lit::as_str), args.get(2).and_then(extra_of)) {
                            s.add_data(&handle, key, value);
                        }
                    }
                    (Target::Styles(s), "add_data") => {
                        if let (Some(key), Some(value)) = (args.get(1).and_then(Lit::as_str), args.get(2).and_then(extra_of)) {
                            s.add_data(&handle, key, value);
                        }
                    }
                    (Target::Scripts(s), "add_inline_script") => {
                        if let Some(code) = args.get(1).and_then(Lit::as_str) {
                            s.add_inline_script(&handle, code, args.get(2).and_then(Lit::as_str).unwrap_or("after"));
                        }
                    }
                    (Target::Styles(s), "add_inline_style") => {
                        if let Some(code) = args.get(1).and_then(Lit::as_str) {
                            s.add_inline_style(&handle, code);
                        }
                    }
                    (Target::Scripts(s), "localize") => {
                        if let (Some(name), Some(data)) = (args.get(1).and_then(Lit::as_str), args.get(2)) {
                            s.localize(&handle, name, &lit_to_json(data));
                        }
                    }
                    (Target::Scripts(s), "set_translations") => {
                        let domain = args.get(1).and_then(Lit::as_str).unwrap_or("default");
                        let path = args.get(2).and_then(Lit::as_str).unwrap_or_default();
                        s.set_translations(&handle, domain, path);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

fn base_vars() -> Vars {
    let mut vars = Vars::new();
    for (name, value) in [("suffix", ".min"), ("dev_suffix", ".min")] {
        vars.insert(name.to_owned(), Lit::Str(value.to_owned()));
    }
    vars.insert("develop_src".into(), Lit::Bool(false));
    vars
}

/// Every script WordPress registers on its own: `wp_default_scripts()`, the vendor scripts and the
/// packages of `script-loader-packages.php`.
pub fn default_scripts(env: &LoaderEnv) -> Scripts {
    let php = read(&env.includes.join("script-loader.php"));
    let mut scripts = Scripts::new(&env.site_url, &env.content_url, &env.version);
    let mut vars = base_vars();
    vars.insert("guessurl".into(), Lit::Str(env.site_url.clone()));
    if let Some(body) = function_body(&php, "wp_default_scripts") {
        run_body(body, &mut vars, env, Target::Scripts(&mut scripts));
    }
    register_vendor(&mut scripts, &php, env);
    register_packages(&mut scripts, env);
    register_loops(&mut scripts, &php, env);
    // `wp_default_packages_inline_scripts()`: the REST client needs the hooks package.
    if let Some(fetch) = scripts.deps.get_mut("wp-api-fetch")
        && !fetch.deps.iter().any(|d| d == "wp-hooks")
    {
        fetch.deps.push("wp-hooks".to_owned());
    }
    scripts
}

/// What `wp_default_scripts()` and `wp_register_tinymce_scripts()` register in loops or by condition.
fn register_loops(scripts: &mut Scripts, php: &str, env: &LoaderEnv) {
    for handle in ["all", "html5", "flash", "silverlight", "html4"] {
        scripts.add(&format!("plupload-{handle}"), None, &["plupload"], Ver::Value("2.1.1".into()), None);
    }
    let version = read(&env.includes.join("version.php"));
    let tinymce = version.split("$tinymce_version = '").nth(1).and_then(|r| r.split('\'').next()).unwrap_or_default().to_owned();
    let _ = php;
    let dir = format!("{}/wp-includes/js/tinymce/", env.site_url);
    if env.compressed_tinymce {
        scripts.add("wp-tinymce", Some(&format!("{dir}wp-tinymce.js")), &[], Ver::Value(tinymce.clone()), None);
    } else {
        scripts.add("wp-tinymce-root", Some(&format!("{dir}tinymce.min.js")), &[], Ver::Value(tinymce.clone()), None);
        scripts.add("wp-tinymce", Some(&format!("{dir}plugins/compat3x/plugin.min.js")), &["wp-tinymce-root"], Ver::Value(tinymce.clone()), None);
    }
    scripts.add("wp-tinymce-lists", Some(&format!("{dir}plugins/lists/plugin.min.js")), &["wp-tinymce"], Ver::Value(tinymce), None);
}

/// `wp_default_packages_vendor()`: the table of vendor scripts and the two inline scripts.
fn register_vendor(scripts: &mut Scripts, php: &str, env: &LoaderEnv) {
    let Some(body) = function_body(php, "wp_default_packages_vendor") else { return };
    let mut vars = base_vars();
    let hooks = hooks(env);
    let mut tables: HashMap<String, Lit> = HashMap::new();
    for (name, at) in [("vendor_scripts", body.find("$vendor_scripts = ")), ("vendor_scripts_versions", body.find("$vendor_scripts_versions = "))] {
        if let Some(at) = at {
            let start = at + body[at..].find('=').map_or(0, |e| e + 1);
            tables.insert(name.to_owned(), Parser::new(body, start, &vars).with_hooks(&hooks).expr());
        }
    }
    let (Some(Lit::Array(deps)), Some(versions)) = (tables.get("vendor_scripts"), tables.get("vendor_scripts_versions")) else { return };
    for (handle, dependencies) in deps {
        let Some(handle) = handle.as_ref().and_then(Lit::as_str) else { continue };
        let ver = versions.get(handle).and_then(Lit::as_str).map(|v| Ver::Value(v.to_owned())).unwrap_or(Ver::Default);
        let dependencies = dependencies.strings();
        let refs: Vec<&str> = dependencies.iter().map(String::as_str).collect();
        scripts.add(handle, Some(&format!("/wp-includes/js/dist/vendor/{handle}.min.js")), &refs, ver, Some(Arg::Int(1)));
    }
    vars.insert("scripts".into(), Lit::Unknown);
    scripts.add_inline_script("lodash", "window.lodash = _.noConflict();", "after");
}

/// `wp_default_packages_scripts()`.
fn register_packages(scripts: &mut Scripts, env: &LoaderEnv) {
    let text = read(&env.includes.join("assets").join("script-loader-packages.php"));
    let Some(start) = text.find("return") else { return };
    let vars = Vars::new();
    let Lit::Array(packages) = Parser::new(&text, start + "return".len(), &vars).expr() else { return };
    for (file, data) in &packages {
        let Some(file) = file.as_ref().and_then(Lit::as_str) else { continue };
        let basename = file.trim_end_matches(".js").rsplit('/').next().unwrap_or(file);
        let handle = format!("wp-{basename}");
        let path = format!("/wp-includes/js/dist/{basename}.min.js");
        let mut dependencies: Vec<String> = data.get("dependencies").map(strings).unwrap_or_default();
        match handle.as_str() {
            "wp-block-library" => dependencies.push("editor".into()),
            "wp-edit-post" => dependencies.extend(["media-models", "media-views", "postbox", "wp-dom-ready"].map(String::from)),
            "wp-preferences" => dependencies.push("wp-preferences-persistence".into()),
            _ => {}
        }
        let refs: Vec<&str> = dependencies.iter().map(String::as_str).collect();
        let version = data.get("version").and_then(Lit::as_str).unwrap_or_default();
        scripts.add(&handle, Some(&path), &refs, Ver::Value(version.to_owned()), Some(Arg::Int(1)));
        if let Some(modules) = data.get("module_dependencies").filter(|m| matches!(m, Lit::Array(items) if !items.is_empty())) {
            scripts.add_data(&handle, "module_dependencies", Extra::List(strings(modules)));
        }
        if dependencies.iter().any(|d| d == "wp-i18n") {
            scripts.set_translations(&handle, "default", "");
        }
        if handle == "wp-i18n" {
            scripts.add_inline_script(&handle, "wp.i18n.setLocaleData( { 'text direction\u{4}ltr': [ 'ltr' ] } );", "after");
        }
    }
}

/// Every stylesheet WordPress registers on its own (`wp_default_styles()`).
pub fn default_styles(env: &LoaderEnv) -> Styles {
    let php = read(&env.includes.join("script-loader.php"));
    let mut styles = Styles::new(&env.site_url, &env.content_url, &env.version, env.rtl);
    let mut vars = base_vars();
    vars.insert("guessurl".into(), Lit::Str(env.site_url.clone()));
    if let Some(body) = function_body(&php, "wp_default_styles") {
        run_body(body, &mut vars, env, Target::Styles(&mut styles));
        register_package_styles(&mut styles, body, env);
    }
    styles
}

/// The `foreach` loops at the end of `wp_default_styles()`: the stylesheets of the JavaScript packages
/// and the right-to-left variants.
fn register_package_styles(styles: &mut Styles, body: &str, env: &LoaderEnv) {
    let vars = Vars::new();
    let table = |name: &str| -> Option<Lit> {
        let at = body.find(&format!("${name} = "))?;
        let start = at + body[at..].find('=')? + 1;
        Some(Parser::new(body, start, &vars).expr())
    };
    if let Some(Lit::Array(packages)) = table("package_styles") {
        for (package, dependencies) in &packages {
            let Some(package) = package.as_ref().and_then(Lit::as_str) else { continue };
            let handle = format!("wp-{package}");
            let path = match package {
                "block-library" if env.separate_block_assets => format!("/wp-includes/css/dist/{package}/common.min.css"),
                "base-styles" => "/wp-includes/css/dist/base-styles/admin-schemes.min.css".to_owned(),
                "theme" => "/wp-includes/css/dist/theme/design-tokens.min.css".to_owned(),
                _ => format!("/wp-includes/css/dist/{package}/style.min.css"),
            };
            let deps = dependencies.strings();
            let refs: Vec<&str> = deps.iter().map(String::as_str).collect();
            styles.add(&handle, Some(&path), &refs, Ver::Default, None);
            styles.add_data(&handle, "path", Extra::Text(format!("/var/www/html/{path}")));
        }
    }
    if let Some(Lit::Array(rtl)) = table("rtl_styles") {
        for (_, handle) in &rtl {
            let Some(handle) = handle.as_str() else { continue };
            styles.add_data(handle, "rtl", Extra::Text("replace".into()));
            styles.add_data(handle, "suffix", Extra::Text(".min".into()));
        }
    }
}
