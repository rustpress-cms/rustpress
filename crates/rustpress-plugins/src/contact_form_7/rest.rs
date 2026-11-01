//! `WPCF7_REST_Controller` (`includes/rest-api.php`): the routes of `contact-form-7/v1`. A submission is checked and
//! mailed here; the routes that change forms belong to the administration and answer only their permission errors.

use rustpress_core::native::{Email, RestAnswer, RestRequest};
use rustpress_core::{Result, Site};
use serde_json::{Map, Value, json};

use super::form::ContactForm;
use super::form_tag::{self, FormTag};
use super::mail::{Lang, PostInfo, Special, Tags, UserInfo};
use super::posted::PostData;
use super::submission::{self, Env};
use super::{DOMAIN, Loaded};

/// `rest_authorization_required_code()`.
fn authorization_code(request: &RestRequest<'_>) -> u16 {
    if request.user.is_some() { 403 } else { 401 }
}

/// The page of the form's language: the texts are translated when the form speaks the site's language.
fn same_language(site: &Site, form: &ContactForm) -> bool {
    form.locale.is_empty() || form.locale == site.locale()
}

/// The quiz questions a form asks anew (`wpcf7_quiz_ajax_refill()`): `None` without a quiz.
fn quiz_refill(tags: &[FormTag]) -> Option<Value> {
    let mut refill = Map::new();
    for tag in tags.iter().filter(|t| t.tag_type == "quiz" && !t.name.is_empty()) {
        let (question, answer) = match tag.pipes.len() {
            0 => ("1+1=?".to_owned(), "2".to_owned()),
            n => {
                let pipe = &tag.pipes[(super::rand_index() % n as u64) as usize];
                (pipe.before.clone(), pipe.after.clone())
            }
        };
        let answer = super::controls::canonicalize(&answer, true);
        refill.insert(tag.name.clone(), json!([question, rustpress_blocks::commenter::wp_hash(&answer, "wpcf7_quiz")]));
    }
    (!refill.is_empty()).then_some(Value::Object(refill))
}

/// `$request->get_param( $name )`: the body, else the query string.
fn param(request: &RestRequest<'_>, name: &str) -> Option<String> {
    if let Some((_, value)) = request.fields.iter().rev().find(|(k, _)| k == name) {
        return Some(value.clone());
    }
    rustpress_core::QueryVars::parse(request.query).get(name).map(str::to_owned)
}

impl Loaded {
    /// The answer of an endpoint of `contact-form-7/v1`.
    pub async fn rest_answer(&self, request: &RestRequest<'_>) -> Result<Option<RestAnswer>> {
        let Some(rest) = request.route.strip_prefix("/contact-form-7/v1/") else { return Ok(None) };
        let site = request.site;
        let tr = |text: &str| site.trd(DOMAIN, text);
        let method = if request.method == "HEAD" { "GET" } else { request.method };
        let segments: Vec<&str> = rest.split('/').collect();
        let numeric = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
        let can = |capability: &str| request.user.is_some_and(|u| u.can(capability));
        let answer = match segments.as_slice() {
            ["contact-forms"] => match method {
                "GET" if !can("edit_posts") => RestAnswer::forbidden("wpcf7_forbidden", &tr("You are not allowed to access contact forms."), authorization_code(request)),
                "GET" => self.list(request),
                "POST" if !can("publish_pages") => RestAnswer::forbidden("wpcf7_forbidden", &tr("You are not allowed to create a contact form."), authorization_code(request)),
                _ => return Ok(None),
            },
            ["contact-forms", id] if numeric(id) => match method {
                "GET" | "POST" | "PUT" | "PATCH" | "DELETE" if !can("publish_pages") => {
                    RestAnswer::forbidden("wpcf7_forbidden", &tr("You are not allowed to access the requested contact form."), authorization_code(request))
                }
                _ => return Ok(None),
            },
            ["contact-forms", id, "feedback"] if numeric(id) && method == "POST" => self.feedback(id, request).await?,
            ["contact-forms", id, "feedback", "schema"] if numeric(id) && method == "GET" => match self.by_id(id) {
                Some(form) => RestAnswer::ok(self.schema_of(site, form)),
                None => RestAnswer::error("wpcf7_not_found", &tr("The requested contact form was not found."), 404),
            },
            ["contact-forms", id, "refill"] if numeric(id) && method == "GET" => match self.by_id(id) {
                Some(form) => RestAnswer::ok(quiz_refill(&form_tag::scan(&form.form)).map_or_else(|| json!([]), |quiz| json!({"quiz": quiz}))),
                None => RestAnswer::error("wpcf7_not_found", &tr("The requested contact form was not found."), 404),
            },
            _ => return Ok(None),
        };
        Ok(Some(answer))
    }

    /// `get_contact_forms()`: `WPCF7_ContactForm::find()` with the parameters of the request.
    fn list(&self, request: &RestRequest<'_>) -> RestAnswer {
        let int = |name: &str| param(request, name).map(|v| v.trim().parse::<i64>().unwrap_or(0));
        let mut forms: Vec<&ContactForm> = self.findable().collect();
        if let Some(search) = param(request, "search").filter(|s| !s.trim().is_empty()) {
            let terms: Vec<String> = search.split_whitespace().map(str::to_lowercase).collect();
            forms.retain(|f| terms.iter().all(|t| f.title.to_lowercase().contains(t) || f.form.to_lowercase().contains(t)));
        }
        let descending = param(request, "order").is_some_and(|o| o.eq_ignore_ascii_case("desc"));
        match param(request, "orderby").as_deref().unwrap_or("ID") {
            "title" => forms.sort_by(|a, b| a.title.cmp(&b.title).then(a.id.cmp(&b.id))),
            "name" | "slug" => forms.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id))),
            _ => forms.sort_by_key(|f| f.id),
        }
        if descending {
            forms.reverse();
        }
        let offset = int("offset").unwrap_or(0).max(0) as usize;
        let per_page = int("per_page").unwrap_or(-1);
        let items: Vec<Value> = forms
            .into_iter()
            .skip(offset)
            .take(if per_page < 0 { usize::MAX } else { per_page as usize })
            .map(|f| json!({"id": f.id, "hash": f.short_hash(7), "slug": f.name, "title": f.title, "locale": f.locale}))
            .collect();
        RestAnswer::ok(Value::Array(items))
    }

    /// `get_schema()` of a form, outside of a submission.
    fn schema_of(&self, site: &Site, form: &ContactForm) -> Value {
        let tags = form_tag::scan(&form.form);
        let special = site_special(site);
        let tr = |text: &str| site.trd(DOMAIN, text);
        let trx = |text: &str, context: &str| site.i18n().pgettext(DOMAIN, context, text);
        let lang = Lang { tr: &tr, trx: &trx, list_separator: site.tr(", ") };
        let format_date = |value: &str, format: &str| format_date(site, value, format);
        let empty = PostData::default();
        let tags_ctx = Tags { tags: &tags, posted: None, post: &empty, uploaded: &[], special: &special, lang: &lang, format_date: &format_date };
        let hash = |data: &str, scheme: &str| rustpress_blocks::commenter::wp_hash(data, scheme);
        let env = Env { form, tags: &tags, special: &special, lang: &lang, format_date: &format_date, hash: &hash, today: today(site), user: None, disallowed_keys: "" };
        super::swv::schema_json(&form.locale, &submission::schema(&env, &tags_ctx))
    }

    /// `create_feedback()`: a submission of the form `id`.
    async fn feedback(&self, id: &str, request: &RestRequest<'_>) -> Result<RestAnswer> {
        let site = request.site;
        let tr = |text: &str| site.trd(DOMAIN, text);
        if !request.content_type.starts_with("multipart/form-data") {
            return Ok(RestAnswer::error("wpcf7_unsupported_media_type", &tr("The request payload format is not supported."), 415));
        }
        let Some(form) = self.by_id(id) else { return Ok(RestAnswer::error("wpcf7_not_found", &tr("The requested contact form was not found."), 404)) };
        let unit_tag = submission::sanitize_unit_tag(&param(request, "_wpcf7_unit_tag").unwrap_or_default());
        if unit_tag.is_empty() {
            return Ok(RestAnswer::error("wpcf7_unit_tag_not_found", &tr("There is no valid unit tag."), 400));
        }
        let tags = form_tag::scan(&form.form);
        let (mut result, invalid) = if form.is_true("subscribers_only") && request.user.is_none() {
            let mut result = Map::new();
            result.insert("contact_form_id".into(), Value::from(form.id));
            result.insert("status".into(), Value::from("error"));
            result.insert("message".into(), Value::from(tr("This contact form is available only for logged in users.")));
            (result, Vec::new())
        } else {
            self.submit(form, &tags, &unit_tag, request).await?
        };
        // `create_feedback()`: where the response goes, and the invalid fields as a list.
        result.insert("into".into(), Value::from(format!("#{unit_tag}")));
        let list: Vec<Value> = invalid
            .into_iter()
            .filter(|item| form_tag::is_name(&item.field))
            .map(|item| {
                let field = item.field.replace('.', "_");
                json!({"field": field, "message": item.reason, "idref": item.idref, "error_id": format!("{unit_tag}-ve-{field}")})
            })
            .collect();
        result.insert("invalid_fields".into(), Value::Array(list));
        if let Some(quiz) = quiz_refill(&tags) {
            result.insert("quiz".into(), quiz);
        }
        Ok(RestAnswer::ok(Value::Object(result)))
    }

    /// `WPCF7_ContactForm::submit()`: the result of the submission and its invalid fields.
    async fn submit(&self, form: &ContactForm, tags: &[FormTag], unit_tag: &str, request: &RestRequest<'_>) -> Result<(Map<String, Value>, Vec<submission::Invalid>)> {
        let site = request.site;
        let post = PostData::from_fields(request.fields);
        let special = submission_special(site, form, request, &post).await?;
        let same = same_language(site, form);
        let tr = |text: &str| if same { site.trd(DOMAIN, text) } else { text.to_owned() };
        let trx = |text: &str, context: &str| if same { site.i18n().pgettext(DOMAIN, context, text) } else { text.to_owned() };
        let lang = Lang { tr: &tr, trx: &trx, list_separator: if same { site.tr(", ") } else { ", ".to_owned() } };
        let format_date = |value: &str, format: &str| format_date(site, value, format);
        let hash = |data: &str, scheme: &str| rustpress_blocks::commenter::wp_hash(data, scheme);
        let disallowed_keys = site.options.get("disallowed_keys").unwrap_or_default().to_owned();
        let env = Env { form, tags, special: &special, lang: &lang, format_date: &format_date, hash: &hash, today: today(site), user: request.user, disallowed_keys: &disallowed_keys };
        let req = submission::Request {
            fields: request.fields,
            files: request.files,
            remote_ip: special.remote_ip.clone(),
            user_agent: special.user_agent.clone(),
            unit_tag: unit_tag.to_owned(),
            now: request.now,
        };
        let mut sub = submission::prepare(&env, &req);
        if sub.is("init") {
            let skip = form.is_true("demo_mode") || form.is_true("skip_mail");
            let sent = skip || send(&submission::mails(&env, &sub), request).await;
            submission::conclude(&env, &mut sub, sent);
        }
        let mut result = sub.result(form.id);
        if form.is_true("demo_mode") {
            result.insert("demo_mode".into(), Value::Bool(true));
        }
        let invalid = if sub.is("validation_failed") { sub.invalid } else { Vec::new() };
        Ok((result, invalid))
    }
}

/// `mail()`: the first mail decides; the others are sent when it went out.
async fn send(mails: &[Email], request: &RestRequest<'_>) -> bool {
    let Some(mailer) = request.mailer else { return false };
    let Some((first, others)) = mails.split_first() else { return false };
    if !mailer.send(&for_wp_mail(first.clone())).await {
        return false;
    }
    for mail in others {
        mailer.send(&for_wp_mail(mail.clone())).await;
    }
    true
}

/// What `wpcf7_phpmailer_init()` leaves of the headers: without `X-WPCF7-Content-Type` and the empty lines.
fn for_wp_mail(mut email: Email) -> Email {
    email.headers.retain(|h| !h.is_empty() && !h.starts_with("X-WPCF7-Content-Type:"));
    email
}

/// Today in the site's time zone.
fn today(site: &Site) -> chrono::NaiveDate {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    let today = rustpress_core::datetime::SiteTimezone::from_options(&site.options).format("Y-m-d", now);
    chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap_or_default()
}

/// `wp_date( $format, date_create( $value, wp_timezone() )->getTimestamp() )`.
fn format_date(site: &Site, value: &str, format: &str) -> Option<String> {
    let timestamp = site.timezone.local_timestamp(&format!("{value} 00:00:00"))?;
    Some(site.date(format, timestamp))
}

/// The special mail tags of the site, without a submission.
fn site_special(site: &Site) -> Special {
    Special {
        site_title: site.options.get("blogname").unwrap_or_default().to_owned(),
        site_description: site.options.get("blogdescription").unwrap_or_default().to_owned(),
        site_url: site.home_url(""),
        admin_email: site.options.get("admin_email").unwrap_or_default().to_owned(),
        ..Special::default()
    }
}

/// `setup_meta_data()` and what the special mail tags read of the post and the user.
async fn submission_special(site: &Site, form: &ContactForm, request: &RestRequest<'_>, post: &PostData) -> Result<Special> {
    let mut special = site_special(site);
    special.submission = true;
    special.form_title = form.title.clone();
    special.remote_ip = request.remote_addr.filter(|ip| ip.parse::<std::net::IpAddr>().is_ok()).unwrap_or_default().to_owned();
    special.user_agent = request.user_agent.unwrap_or_default().to_owned();
    // `get_request_url()`: the page the form was sent from, when it is on this site.
    let home = site.home_url("").trim_end_matches('/').to_owned();
    special.url = match request.referer.filter(|r| r.starts_with(&home)) {
        Some(referer) => rustpress_core::formatting::esc_url_raw(referer),
        None => {
            let origin = home.find("://").and_then(|at| home[at + 3..].find('/').map(|slash| &home[..at + 3 + slash])).unwrap_or(&home);
            format!("{origin}{}", super::request_uri(request.path, request.query))
        }
    };
    special.date = site.date(site.options.get("date_format").unwrap_or("F j, Y"), request.now);
    special.time = site.date(site.options.get("time_format").unwrap_or("g:i a"), request.now);
    let container = post.string("_wpcf7_container_post").trim().parse::<u64>().unwrap_or(0);
    if container != 0
        && let Some(p) = site.db.post(container).await?
    {
        let author = site.db.user(p.post_author).await?;
        special.post = Some(PostInfo {
            id: p.id,
            name: p.post_name.clone(),
            title: p.post_title.clone(),
            url: site.permalink(&p).await?,
            author: author.as_ref().map(|u| u.display_name.clone()).unwrap_or_default(),
            author_email: author.map(|u| u.user_email).unwrap_or_default(),
        });
    }
    if let Some(viewer) = request.user
        && let Some(u) = site.db.user(viewer.id).await?
    {
        let meta = |key: &'static str| async move { site.db.user_meta(u.id, key).await.map(Option::unwrap_or_default) };
        special.user = Some(UserInfo {
            first_name: meta("first_name").await?,
            last_name: meta("last_name").await?,
            nickname: meta("nickname").await?,
            login: u.user_login.clone(),
            email: u.user_email.clone(),
            url: u.user_url.clone(),
            display_name: u.display_name.clone(),
        });
    }
    Ok(special)
}
