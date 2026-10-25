//! `WP_Dependencies` with the parts of `WP_Scripts` and `WP_Styles` that differ: registration,
//! the queue, dependency resolution into a print order and the groups (head and footer).

use std::collections::HashMap;

/// Scripts and styles share the engine; a few rules differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Scripts,
    Styles,
}

/// The `$ver` of a registration: `false` (the default version), `null` (no version) or a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ver {
    Default,
    None,
    Value(String),
}

/// The `$args` of a registration: the footer flag `1` of a script or the `media` of a style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arg {
    Int(i64),
    Text(String),
}

/// A value of `$dependency->extra`.
#[derive(Debug, Clone, PartialEq)]
pub enum Extra {
    Text(String),
    Bool(bool),
    Int(i64),
    List(Vec<String>),
}

impl Extra {
    pub fn text(&self) -> Option<&str> {
        match self {
            Extra::Text(s) => Some(s),
            _ => None,
        }
    }

    /// PHP truthiness.
    pub fn truthy(&self) -> bool {
        match self {
            Extra::Text(s) => !s.is_empty() && s != "0",
            Extra::Bool(b) => *b,
            Extra::Int(i) => *i != 0,
            Extra::List(l) => !l.is_empty(),
        }
    }
}

/// `_WP_Dependency`.
#[derive(Debug, Clone)]
pub struct Dependency {
    pub handle: String,
    /// `false` (an alias that only brings its dependencies) is `None`.
    pub src: Option<String>,
    pub deps: Vec<String>,
    pub ver: Ver,
    pub args: Option<Arg>,
    pub extra: HashMap<String, Extra>,
    pub textdomain: Option<String>,
    pub translations_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Dependencies {
    pub kind: Kind,
    registered: Vec<Dependency>,
    index: HashMap<String, usize>,
    pub queue: Vec<String>,
    pub to_do: Vec<String>,
    pub done: Vec<String>,
    /// `$args` given to a handle at enqueue time (`handle?args`).
    pub args: HashMap<String, String>,
    pub groups: HashMap<String, i64>,
    queued_before_register: Vec<(String, Option<String>)>,
}

impl Dependencies {
    pub fn new(kind: Kind) -> Self {
        Self { kind, registered: Vec::new(), index: HashMap::new(), queue: Vec::new(), to_do: Vec::new(), done: Vec::new(), args: HashMap::new(), groups: HashMap::new(), queued_before_register: Vec::new() }
    }

    pub fn registered(&self) -> &[Dependency] {
        &self.registered
    }

    pub fn get(&self, handle: &str) -> Option<&Dependency> {
        self.index.get(handle).map(|i| &self.registered[*i])
    }

    pub fn get_mut(&mut self, handle: &str) -> Option<&mut Dependency> {
        let i = *self.index.get(handle)?;
        Some(&mut self.registered[i])
    }

    pub fn is_registered(&self, handle: &str) -> bool {
        self.index.contains_key(handle)
    }

    /// `add()`: the first registration of a handle wins.
    pub fn add(&mut self, handle: &str, src: Option<String>, deps: Vec<String>, ver: Ver, args: Option<Arg>) -> bool {
        if self.is_registered(handle) {
            return false;
        }
        self.index.insert(handle.to_owned(), self.registered.len());
        self.registered.push(Dependency { handle: handle.to_owned(), src, deps, ver, args, extra: HashMap::new(), textdomain: None, translations_path: None });
        if let Some(position) = self.queued_before_register.iter().position(|(h, _)| h == handle) {
            let (_, args) = self.queued_before_register.remove(position);
            match args {
                Some(args) => self.enqueue(&format!("{handle}?{args}")),
                None => self.enqueue(handle),
            }
        }
        true
    }

    /// `add_data()`, with the checks of `WP_Scripts::add_data()`.
    pub fn add_data(&mut self, handle: &str, key: &str, value: Extra) -> bool {
        let kind = self.kind;
        let Some(dep) = self.get_mut(handle) else { return false };
        if key == "conditional" {
            dep.deps.clear();
        }
        if kind == Kind::Scripts {
            match key {
                "strategy" => {
                    let delayed = matches!(&value, Extra::Text(s) if s == "defer" || s == "async");
                    if value.truthy() && !delayed {
                        return false;
                    }
                    if dep.src.is_none() && delayed {
                        return false;
                    }
                }
                "fetchpriority" => {
                    let valid = matches!(&value, Extra::Text(s) if matches!(s.as_str(), "auto" | "low" | "high"));
                    if (value.truthy() && !valid) || dep.src.is_none() {
                        return false;
                    }
                }
                _ => {}
            }
        }
        dep.extra.insert(key.to_owned(), value);
        true
    }

    pub fn get_data(&self, handle: &str, key: &str) -> Option<&Extra> {
        self.get(handle)?.extra.get(key)
    }

    pub fn remove(&mut self, handle: &str) {
        if let Some(i) = self.index.remove(handle) {
            self.registered.remove(i);
            self.index = self.registered.iter().enumerate().map(|(i, d)| (d.handle.clone(), i)).collect();
        }
    }

    /// `enqueue()`: `handle` or `handle?args`.
    pub fn enqueue(&mut self, handle: &str) {
        let (name, args) = match handle.split_once('?') {
            Some((name, args)) => (name, Some(args)),
            None => (handle, None),
        };
        if self.is_registered(name) {
            if !self.queue.iter().any(|h| h == name) {
                self.queue.push(name.to_owned());
                if let Some(args) = args {
                    self.args.insert(name.to_owned(), args.to_owned());
                }
            }
        } else {
            self.queued_before_register.retain(|(h, _)| h != name);
            self.queued_before_register.push((name.to_owned(), args.map(str::to_owned)));
        }
    }

    pub fn dequeue(&mut self, handle: &str) {
        let name = handle.split('?').next().unwrap_or(handle);
        if let Some(position) = self.queue.iter().position(|h| h == name) {
            self.queue.remove(position);
            self.args.remove(name);
        } else {
            self.queued_before_register.retain(|(h, _)| h != name);
        }
    }

    /// `query( $handle, 'enqueued' )`: queued, or a dependency of something queued.
    pub fn is_enqueued(&self, handle: &str) -> bool {
        if self.queue.iter().any(|h| h == handle) {
            return true;
        }
        let mut seen: Vec<&str> = Vec::new();
        let mut stack: Vec<&str> = self.queue.iter().map(String::as_str).collect();
        while let Some(current) = stack.pop() {
            if seen.contains(&current) {
                continue;
            }
            seen.push(current);
            if let Some(dep) = self.get(current) {
                for d in &dep.deps {
                    if d == handle {
                        return true;
                    }
                    stack.push(d);
                }
            }
        }
        false
    }

    /// `set_group()`.
    fn set_group(&mut self, handle: &str, group: Option<i64>) -> bool {
        let mut calculated: i64;
        if self.kind == Kind::Scripts {
            calculated = match self.get(handle).and_then(|d| d.args.as_ref()) {
                Some(Arg::Int(1)) => 1,
                _ => match self.get_data(handle, "group") {
                    Some(Extra::Int(i)) => *i,
                    Some(Extra::Text(s)) => s.trim().parse().unwrap_or(0),
                    _ => 0,
                },
            };
            if let Some(group) = group
                && calculated > group
            {
                calculated = group;
            }
        } else {
            // Styles: the group of the caller, else 0.
            calculated = group.unwrap_or(0);
        }
        if self.groups.get(handle).is_some_and(|g| *g <= calculated) {
            return false;
        }
        self.groups.insert(handle.to_owned(), calculated);
        true
    }

    /// `all_deps()`: fills `to_do` with the handles and, before each, what it depends on.
    pub fn all_deps(&mut self, handles: &[String], recursion: bool, group: Option<i64>) -> bool {
        if handles.is_empty() {
            return false;
        }
        for raw in handles {
            let (handle, extra_args) = match raw.split_once('?') {
                Some((h, a)) => (h.to_owned(), Some(a.to_owned())),
                None => (raw.clone(), None),
            };
            let queued = self.to_do.contains(&handle);
            if self.done.contains(&handle) {
                continue;
            }
            let moved = self.set_group(&handle, group);
            let new_group = self.groups.get(&handle).copied();
            if queued && !moved {
                continue;
            }
            let mut keep_going = true;
            match self.get(&handle) {
                None => keep_going = false,
                Some(dep) if dep.deps.iter().any(|d| !self.is_registered(d)) => keep_going = false,
                Some(dep) if !dep.deps.is_empty() => {
                    let deps = dep.deps.clone();
                    if !self.all_deps(&deps, true, new_group) {
                        keep_going = false;
                    }
                }
                _ => {}
            }
            if !keep_going {
                if recursion {
                    return false;
                }
                continue;
            }
            if queued {
                continue;
            }
            if let Some(args) = extra_args {
                self.args.insert(handle.clone(), args);
            }
            self.to_do.push(handle);
        }
        true
    }

    /// `get_etag( $load )`.
    pub fn etag(&self, load: &[String], wp_version: &str) -> String {
        let mut etag = format!("WP:{wp_version};");
        for handle in load {
            if let Some(dep) = self.get(handle) {
                let ver = match &dep.ver {
                    Ver::Value(v) => v.clone(),
                    _ => wp_version.to_owned(),
                };
                etag.push_str(&format!("{handle}:{ver};"));
            }
        }
        format!("W/\"{}\"", md5_hex(&etag))
    }

    /// `get_dependents()`: the registered handles that list `handle` as a dependency.
    pub fn dependents(&self, handle: &str) -> Vec<String> {
        self.registered.iter().filter(|d| d.deps.iter().any(|x| x == handle)).map(|d| d.handle.clone()).collect()
    }
}

/// The lower-case hex md5 of a string.
pub fn md5_hex(text: &str) -> String {
    // A small md5: the admin crate has no md5 dependency of its own.
    rustpress_auth::salts::md5_hex(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scripts() -> Dependencies {
        let mut d = Dependencies::new(Kind::Scripts);
        d.add("jquery-core", Some("/core.js".into()), vec![], Ver::Value("3".into()), Some(Arg::Int(1)));
        d.add("jquery", None, vec!["jquery-core".into(), "jquery-migrate".into()], Ver::Value("3".into()), None);
        d.add("jquery-migrate", Some("/migrate.js".into()), vec![], Ver::Default, None);
        d.add("common", Some("/common.js".into()), vec!["jquery".into()], Ver::Default, Some(Arg::Int(1)));
        d
    }

    #[test]
    fn dependencies_come_first_and_aliases_are_kept() {
        let mut d = scripts();
        d.enqueue("common");
        let queue = d.queue.clone();
        assert!(d.all_deps(&queue, false, None));
        assert_eq!(d.to_do, ["jquery-core", "jquery-migrate", "jquery", "common"]);
    }

    #[test]
    fn a_footer_script_pulls_its_dependencies_into_the_footer_but_not_the_other_way() {
        let mut d = scripts();
        d.add("head", Some("/head.js".into()), vec!["jquery-core".into()], Ver::Default, None);
        d.enqueue("common");
        d.enqueue("head");
        let queue = d.queue.clone();
        d.all_deps(&queue, false, None);
        assert_eq!(d.groups["common"], 1);
        // `jquery-core` is a footer script, but the head script needs it in the head.
        assert_eq!(d.groups["jquery-core"], 0);
    }

    #[test]
    fn missing_dependencies_drop_the_handle() {
        let mut d = scripts();
        d.add("broken", Some("/b.js".into()), vec!["nope".into()], Ver::Default, None);
        d.enqueue("broken");
        let queue = d.queue.clone();
        d.all_deps(&queue, false, None);
        assert!(d.to_do.is_empty());
    }

    #[test]
    fn enqueue_before_register_waits() {
        let mut d = Dependencies::new(Kind::Styles);
        d.enqueue("late?x=1");
        d.add("late", Some("/late.css".into()), vec![], Ver::Default, None);
        assert_eq!(d.queue, ["late"]);
        assert_eq!(d.args["late"], "x=1");
    }

    #[test]
    fn first_registration_wins() {
        let mut d = scripts();
        assert!(!d.add("common", Some("/other.js".into()), vec![], Ver::Default, None));
        assert_eq!(d.get("common").unwrap().src.as_deref(), Some("/common.js"));
    }
}
