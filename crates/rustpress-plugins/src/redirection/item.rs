//! `Red_Item`: one redirect with its match (`Red_Match` and the classes in `matches/`) and action (`Red_Action`
//! and `actions/`).

use std::net::IpAddr;

use rustpress_core::native::PluginRequest;
use rustpress_core::{Result, Site};
use rustpress_db::RedirectionItem;
use rustpress_phpser::Value;

use super::options::{Options, php_string, truthy};
use super::regex::PcreRegex;
use super::request;
use super::url::{self, FlagInput, FlagValue, Flags, UrlQuery};

/// The `FromNotFrom_Match` targets.
#[derive(Debug, Clone, Default)]
pub struct FromNotFrom {
    pub from: String,
    pub notfrom: String,
}

impl FromNotFrom {
    fn load(values: &Value) -> Self {
        let field = |key: &str| values.get(key).filter(|v| !matches!(v, Value::Null)).map(php_string).unwrap_or_default();
        Self { from: field("url_from"), notfrom: field("url_notfrom") }
    }

    /// `get_matched_target()`.
    fn target(&self, matched: bool) -> Option<String> {
        if !self.from.is_empty() && matched {
            Some(self.from.clone())
        } else if !self.notfrom.is_empty() && !matched {
            Some(self.notfrom.clone())
        } else {
            None
        }
    }
}

/// A match type and its data.
#[derive(Debug, Clone)]
pub enum Match {
    Url { url: String },
    Referrer { regex: bool, referrer: Option<String>, targets: FromNotFrom },
    Agent { regex: bool, agent: Option<String>, targets: FromNotFrom },
    Login { logged_in: String, logged_out: String },
    Header { regex: bool, name: String, value: String, targets: FromNotFrom },
    Cookie { regex: bool, name: String, value: String, targets: FromNotFrom },
    /// `Custom_Match`: a PHP filter decides, which never answers here.
    Custom { targets: FromNotFrom },
    Role { role: String, targets: FromNotFrom },
    Server { server: String, targets: FromNotFrom },
    Ip { ips: Vec<String>, targets: FromNotFrom },
    /// `Page_Match`: only the 404 page; the target is `url` (`FromUrl_Match`).
    Page { url: String },
    Language { language: String, targets: FromNotFrom },
}

/// `@unserialize( $values )`: the array the match data is stored as.
fn unserialize(data: &str) -> Value {
    rustpress_phpser::unserialize(data.as_bytes()).unwrap_or(Value::Bool(false))
}

impl Match {
    /// `Red_Match::create( $name, $data )`.
    fn create(name: &str, data: &str) -> Option<Self> {
        let values = if data.is_empty() { Value::Null } else { unserialize(data) };
        let get = |key: &str| values.get(key).filter(|v| !matches!(v, Value::Null));
        let string = |key: &str| get(key).map(php_string).unwrap_or_default();
        // A field compared with `===`: only a string can be equal.
        let strict = |key: &str| match get(key) {
            Some(Value::Str(b)) => Some(String::from_utf8_lossy(b).into_owned()),
            Some(_) => None,
            None => Some(String::new()),
        };
        let regex = get("regex").is_some_and(truthy);
        let targets = || FromNotFrom::load(&values);
        Some(match name.to_ascii_lowercase().as_str() {
            "url" => Self::Url { url: data.to_owned() },
            "referrer" => Self::Referrer { regex, referrer: strict("referrer"), targets: targets() },
            "agent" => Self::Agent { regex, agent: strict("agent"), targets: targets() },
            "login" => Self::Login { logged_in: string("logged_in"), logged_out: string("logged_out") },
            "header" => Self::Header { regex, name: string("name"), value: string("value"), targets: targets() },
            "cookie" => Self::Cookie { regex, name: string("name"), value: string("value"), targets: targets() },
            "custom" => Self::Custom { targets: targets() },
            "role" => Self::Role { role: string("role"), targets: targets() },
            "server" => Self::Server { server: string("server"), targets: targets() },
            "ip" => Self::Ip { ips: get("ip").and_then(Value::as_array).unwrap_or_default().iter().map(|(_, v)| php_string(v)).collect(), targets: targets() },
            "page" => Self::Page { url: string("url") },
            "language" => Self::Language { language: string("language"), targets: targets() },
            _ => return None,
        })
    }

    /// `is_match( $url )`.
    fn is_match(&self, ctx: &Ctx<'_>) -> bool {
        let request = ctx.request;
        match self {
            Self::Url { .. } => true,
            Self::Referrer { regex, referrer, .. } => {
                let Some(referrer) = referrer else { return false };
                let actual = request::referrer(request);
                if *regex { PcreRegex::new(referrer, true).is_match(&actual) } else { actual == *referrer }
            }
            Self::Agent { regex, agent, .. } => {
                let Some(agent) = agent else { return false };
                let actual = request::user_agent(request);
                if *regex { PcreRegex::new(agent, true).is_match(&actual) } else { actual == *agent }
            }
            Self::Login { .. } => request.viewer.is_some(),
            Self::Header { regex, name, value, .. } => match request::header(request, name) {
                Some(actual) if *regex => PcreRegex::new(value, true).is_match(&actual),
                Some(actual) => actual == *value,
                None => false,
            },
            Self::Cookie { regex, name, value, .. } => match request::cookie(request, name) {
                Some(actual) if *regex => PcreRegex::new(value, true).is_match(&actual),
                Some(actual) => actual == *value,
                None => false,
            },
            // `apply_filters( $filter, false, $url )`.
            Self::Custom { .. } => false,
            Self::Role { role, .. } => !role.is_empty() && request.viewer.is_some_and(|v| v.can(role)),
            Self::Server { server, .. } => host_of(server).is_some_and(|host| host == request::server_name(request)),
            Self::Ip { ips, .. } => {
                // `inet_pton()` of both sides; an address that is none compares as `false`.
                let current = request::ip(request, ctx.options).parse::<IpAddr>().ok();
                ips.iter().any(|ip| ip.parse::<IpAddr>().ok() == current)
            }
            Self::Page { .. } => ctx.is_404,
            Self::Language { language, .. } => {
                let requested = request::accept_language(request);
                language.split(',').any(|l| requested.iter().any(|r| r == l))
            }
        }
    }

    /// `get_target_url( $original_url, $matched_url, $flags, $is_matched )`.
    fn target_url(&self, original: &str, source: &str, flags: &Flags, matched: bool) -> Option<String> {
        let target = match self {
            Self::Url { url } => Some(url.clone()),
            Self::Login { logged_in, logged_out } => {
                if matched && !logged_in.is_empty() {
                    Some(logged_in.clone())
                } else if !matched && !logged_out.is_empty() {
                    Some(logged_out.clone())
                } else {
                    None
                }
            }
            Self::Page { url } => matched.then(|| url.clone()),
            Self::Referrer { targets, .. }
            | Self::Agent { targets, .. }
            | Self::Header { targets, .. }
            | Self::Cookie { targets, .. }
            | Self::Custom { targets, .. }
            | Self::Role { targets, .. }
            | Self::Server { targets, .. }
            | Self::Ip { targets, .. }
            | Self::Language { targets, .. } => targets.target(matched),
        }?;
        Some(if flags.regex { regex_target(source, &target, original, flags) } else { target })
    }

    pub fn is_page(&self) -> bool {
        matches!(self, Self::Page { .. })
    }
}

/// `wp_parse_url( $url, PHP_URL_HOST )`.
fn host_of(url: &str) -> Option<String> {
    let rest = url.split_once("//")?.1;
    let host = rest.split(['/', '?', '#']).next()?;
    let host = host.rsplit_once('@').map_or(host, |(_, h)| h);
    let host = if host.starts_with('[') { host.split_once(']').map_or(host, |(h, _)| h).to_owned() + "]" } else { host.split(':').next().unwrap_or(host).to_owned() };
    (!host.is_empty()).then_some(host)
}

/// `Red_Match::get_target_regex_url()` with `keep_target_relative()`.
fn regex_target(source: &str, target: &str, requested: &str, flags: &Flags) -> String {
    let replaced = PcreRegex::new(source, flags.case).replace(target, requested);
    if !target.starts_with('/') || target.starts_with("//") {
        return replaced;
    }
    let check: String = replaced.replace('\\', "/").chars().filter(|c| !matches!(*c as u32, 0..=0x20 | 0x7F)).collect();
    if !check.starts_with("//") {
        return replaced;
    }
    let rest = replaced.trim_start_matches(|c: char| c == '/' || c == '\\' || matches!(c as u32, 0..=0x20 | 0x7F));
    format!("/{rest}")
}

/// An action type with its code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Url(i64),
    Error(i64),
    Nothing,
    Random(i64),
    Pass,
}

impl Action {
    /// `Red_Action::create( $name, $code )`.
    fn create(name: &str, code: i64) -> Option<Self> {
        Some(match name {
            "url" => Self::Url(code),
            "error" => Self::Error(code),
            "nothing" => Self::Nothing,
            "random" => Self::Random(code),
            "pass" => Self::Pass,
            _ => return None,
        })
    }

    /// `needs_target()`.
    fn needs_target(self) -> bool {
        matches!(self, Self::Url(_) | Self::Pass)
    }
}

/// What a match is checked against.
pub struct Ctx<'a> {
    pub site: &'a Site,
    pub request: &'a PluginRequest<'a>,
    pub options: &'a Options,
    pub is_404: bool,
}

/// A redirect loaded from its row.
#[derive(Debug, Clone)]
pub struct Item {
    pub id: u64,
    pub url: String,
    pub position: u64,
    pub action_code: i64,
    enabled: bool,
    flags: Flags,
    log_exclude: bool,
    pub matcher: Option<Match>,
    pub action: Option<Action>,
}

/// A redirect that fired: its action and target.
pub struct Fired {
    pub action: Action,
    pub target: Option<String>,
}

impl Item {
    /// `Red_Item::load_from_data()`.
    pub fn load(row: &RedirectionItem, options: &Options) -> Self {
        let match_type = if row.match_type.is_empty() { "url" } else { &row.match_type };
        let action_type = if row.action_type.is_empty() { "nothing" } else { &row.action_type };
        let action_data = row.action_data.clone().unwrap_or_default();
        let from_options = |regex: Option<FlagValue>| FlagInput {
            query: FlagValue::from_php(options.get("flag_query")),
            case: FlagValue::from_php(options.get("flag_case")),
            trailing: FlagValue::from_php(options.get("flag_trailing")),
            regex: regex.or_else(|| FlagValue::from_php(options.get("flag_regex"))),
        };
        let mut flags = Flags::default();
        flags.set(&from_options(Some(FlagValue::Bool(row.regex))));
        let mut log_exclude = false;
        if let Some(data) = row.match_data.as_deref().filter(|d| !d.is_empty())
            && let Ok(serde_json::Value::Object(json)) = serde_json::from_str::<serde_json::Value>(data)
        {
            if let Some(serde_json::Value::Object(source)) = json.get("source") {
                // `array_merge( $options, $json['source'] )`.
                let mut input = from_options(None);
                let pick = |key: &str, fallback: Option<FlagValue>| match source.get(key) {
                    Some(v) => FlagValue::from_json(v),
                    None => fallback,
                };
                input = FlagInput { query: pick("flag_query", input.query), case: pick("flag_case", input.case), trailing: pick("flag_trailing", input.trailing), regex: pick("flag_regex", input.regex) };
                flags.set(&input);
            }
            if let Some(options) = json.get("options").filter(|o| o.is_object() || o.is_array()) {
                log_exclude = options.get("log_exclude") == Some(&serde_json::Value::Bool(true));
            }
        }
        Self {
            id: row.id,
            url: row.url.clone(),
            position: row.position,
            action_code: row.action_code,
            enabled: row.status == "enabled",
            flags,
            log_exclude,
            matcher: Match::create(match_type, &action_data),
            action: Action::create(action_type, row.action_code),
        }
    }

    /// `Red_Item::get_for_matched_url()`: the redirects of a URL by position, then by id.
    pub async fn for_url(site: &Site, options: &Options, url: &str) -> Result<Vec<Item>> {
        let rows = site.db.redirection_items_for(&url::match_url(url), &url::match_url_with_params(url)).await?;
        let mut items: Vec<Item> = rows.iter().map(|row| Item::load(row, options)).collect();
        items.sort_by(|a, b| a.position.cmp(&b.position).then(a.id.cmp(&b.id)));
        Ok(items)
    }

    /// `get_match( $requested_url, $original_url )`, with `redirection_visit` (the hit counter and the log).
    pub async fn get_match(&self, ctx: &Ctx<'_>, requested: &str, original: &str) -> Result<Option<Fired>> {
        let (Some(matcher), Some(action)) = (&self.matcher, self.action) else { return Ok(None) };
        if !self.enabled {
            return Ok(None);
        }
        let source = self.url.replace(' ', "%20");
        if !url::is_match(&self.url, requested, &self.flags) {
            return Ok(None);
        }
        let matched = matcher.is_match(ctx);
        let mut target = None;
        if action.needs_target() {
            let found = matcher.target_url(original, &source, &self.flags, matched).map(|t| UrlQuery::add_to_target(&t, original, &self.flags));
            // `redirection_url_target`: `Red_Url_Transform::transform()`.
            let found = match found {
                Some(t) => Some(transform(ctx, &t).await?),
                None => None,
            };
            let Some(found) = found.filter(|t| !t.is_empty() && t != "0") else { return Ok(None) };
            self.visit(ctx, original, &found).await?;
            target = Some(found);
        }
        Ok((matched || target.is_some()).then_some(Fired { action, target }))
    }

    /// `Red_Item::visit()`.
    async fn visit(&self, ctx: &Ctx<'_>, url: &str, target: &str) -> Result<()> {
        let Some(writer) = ctx.request.writer else { return Ok(()) };
        if ctx.options.truthy("track_hits") {
            let sql = format!("UPDATE {} SET last_count=last_count+1, last_access=NOW() WHERE id={}", writer.table("redirection_items"), self.id);
            writer.execute(sql).await?;
        }
        // `Red_Source_Options::can_log()`.
        if !target.is_empty() && !ctx.options.is_int("expire_redirect", -1) && !self.log_exclude {
            let details = request::Details {
                target: Some(target.to_owned()),
                http_code: self.action_code,
                redirect_id: Some(self.id),
                redirect_by: Some("redirection".to_owned()),
                log_headers: ctx.options.is_true("log_header"),
            };
            request::log(ctx.site, ctx.request, ctx.options, "redirection_logs", url, details).await?;
        }
        Ok(())
    }
}

/// `Red_Url_Transform::transform()`: a post ID becomes its permalink, the plugin's shortcodes are replaced.
async fn transform(ctx: &Ctx<'_>, url: &str) -> Result<String> {
    if super::php::is_numeric(url) {
        let id = super::php::intval(url.trim());
        if id > 0
            && let Some(post) = ctx.site.db.post(id as u64).await?
        {
            return ctx.site.permalink(&post).await;
        }
    }
    let url = url.replace("%userid%", "[userid]").replace("%userlogin%", "[userlogin]").replace("%userurl%", "[userurl]");
    const TAGS: [&str; 8] = ["userid", "userlogin", "unixtime", "md5", "upper", "lower", "dashes", "underscores"];
    if !url.contains('[') {
        return Ok(url);
    }
    let mut out = String::new();
    let mut at = 0;
    while let Some(found) = rustpress_core::shortcodes::find(&url, at, &TAGS) {
        out.push_str(&url[at..found.start]);
        if found.open_escape && found.close_escape {
            out.push_str(&url[found.start + 1..found.end - 1]);
        } else {
            let content = found.content.clone().unwrap_or_default();
            let viewer = ctx.request.viewer;
            let replaced = match found.name.as_str() {
                "userid" => viewer.map_or(0, |v| v.id).to_string(),
                "userlogin" => viewer.map(|v| v.login.clone()).unwrap_or_default(),
                "unixtime" => chrono::Utc::now().timestamp().to_string(),
                "md5" => format!("{:x}", md5::compute(content.as_bytes())),
                "upper" => content.to_ascii_uppercase(),
                "lower" => content.to_ascii_lowercase(),
                "dashes" => content.replace(['_', ' '], "-"),
                _ => content.replace(['-', ' '], "_"),
            };
            out.push_str(&replaced);
        }
        at = found.end;
    }
    out.push_str(&url[at..]);
    Ok(out)
}
