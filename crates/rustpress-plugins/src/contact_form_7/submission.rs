//! `WPCF7_Submission` (`includes/submission.php`) up to the mail: what was posted, the validation of the schema and
//! of the modules, consent, the spam checks and the uploaded files. Sending the mail and the response are left to
//! the caller, which can wait for the mailer.

use std::sync::LazyLock;

use regex::Regex;
use rustpress_core::native::{Attachment, Email, RestFile};
use serde_json::{Map, Value};

use super::form::ContactForm;
use super::form_tag::{FormTag, features};
use super::formatter::{Side, strip_whitespaces};
use super::mail::{Component, Compose, Lang, Special, Tags, Template, compose as compose_mail, php_string, replace_tags};
use super::posted::{PostData, Posted, clean};
use super::swv::{self, FormData, Messages, SubmittedFile};

/// What a submission is checked with besides the request.
pub struct Env<'a> {
    pub form: &'a ContactForm,
    /// The form-tags of the form (`scan_form_tags()`).
    pub tags: &'a [FormTag],
    pub special: &'a Special,
    pub lang: &'a Lang<'a>,
    pub format_date: &'a (dyn Fn(&str, &str) -> Option<String> + Sync),
    /// `wp_hash( $data, $scheme )`.
    pub hash: &'a (dyn Fn(&str, &str) -> String + Sync),
    pub today: chrono::NaiveDate,
    /// The signed-in user.
    pub user: Option<&'a rustpress_core::Viewer>,
    /// The option `disallowed_keys`.
    pub disallowed_keys: &'a str,
}

/// The request of a submission.
pub struct Request<'a> {
    pub fields: &'a [(String, String)],
    pub files: &'a [RestFile],
    /// `REMOTE_ADDR` when it is an IP address.
    pub remote_ip: String,
    pub user_agent: String,
    pub unit_tag: String,
    pub now: i64,
}

/// A field that did not validate.
#[derive(Debug, Clone, PartialEq)]
pub struct Invalid {
    pub field: String,
    pub reason: String,
    /// The `id` of the field's control.
    pub idref: Option<String>,
}

/// The state of a submission.
#[derive(Debug, Clone)]
pub struct Submission {
    pub status: String,
    pub response: String,
    /// `$_POST`.
    pub post: PostData,
    /// `get_posted_data()`.
    pub posted: PostData,
    pub posted_hash: String,
    pub invalid: Vec<Invalid>,
    /// The files moved out of the upload, by field.
    pub uploaded: Vec<(String, Vec<Attachment>)>,
}

impl Submission {
    pub fn is(&self, status: &str) -> bool {
        self.status == status
    }

    /// The names of the uploaded files by field, for the mail tags.
    pub fn uploaded_names(&self) -> Vec<(String, Vec<String>)> {
        self.uploaded.iter().map(|(field, files)| (field.clone(), files.iter().map(|f| f.filename.clone()).collect())).collect()
    }

    /// `get_result()` with the contact form's ID first (`submit()`).
    pub fn result(&self, form_id: u64) -> Map<String, Value> {
        let mut result = Map::new();
        result.insert("contact_form_id".into(), Value::from(form_id));
        result.insert("status".into(), Value::from(self.status.clone()));
        result.insert("message".into(), Value::from(self.response.clone()));
        if self.is("validation_failed") {
            // Replaced by the list of the REST response.
            result.insert("invalid_fields".into(), Value::Null);
        }
        let hash = if matches!(self.status.as_str(), "init" | "validation_failed" | "acceptance_missing" | "spam") { String::new() } else { self.posted_hash.clone() };
        result.insert("posted_data_hash".into(), Value::from(hash));
        result
    }
}

/// The mail tags of a message replaced, without markup (`filter_message()`).
pub fn filter_message(text: &str, tags: &Tags<'_>) -> String {
    rustpress_core::formatting::wp_strip_all_tags(&replace_tags(text, false, false, Component::None, tags))
}

/// `wpcf7_messages()`: the messages of the plugin and its modules with their default texts.
pub const DEFAULT_MESSAGES: &[(&str, &str)] = &[
    ("mail_sent_ok", "Thank you for your message. It has been sent."),
    ("mail_sent_ng", "There was an error trying to send your message. Please try again later."),
    ("validation_error", "One or more fields have an error. Please check and try again."),
    ("spam", "There was an error trying to send your message. Please try again later."),
    ("accept_terms", "You must accept the terms and conditions before sending your message."),
    ("invalid_required", "Please fill out this field."),
    ("invalid_too_long", "This field has a too long input."),
    ("invalid_too_short", "This field has a too short input."),
    ("upload_failed", "There was an unknown error uploading the file."),
    ("upload_file_type_invalid", "You are not allowed to upload files of this type."),
    ("upload_file_too_large", "The uploaded file is too large."),
    ("upload_failed_php_error", "There was an error uploading the file."),
    ("invalid_date", "Please enter a date in YYYY-MM-DD format."),
    ("date_too_early", "This field has a too early date."),
    ("date_too_late", "This field has a too late date."),
    ("invalid_number", "Please enter a number."),
    ("number_too_small", "This field has a too small number."),
    ("number_too_large", "This field has a too large number."),
    ("quiz_answer_not_correct", "The answer to the quiz is incorrect."),
    ("captcha_not_match", "Your entered code is incorrect."),
    ("invalid_email", "Please enter an email address."),
    ("invalid_url", "Please enter a URL."),
    ("invalid_tel", "Please enter a telephone number."),
];

/// `message( $status, false )`: the stored message of the form for `status`. `upgrade()` fills the messages a form
/// has not stored with the defaults (translated by `tr`); `''` for an unknown status.
pub fn stored_message(form: &ContactForm, status: &str, tr: &dyn Fn(&str) -> String) -> String {
    let stored = form.messages.as_ref();
    if let Some(message) = stored.and_then(|m| m.get(status)) {
        return php_string(message);
    }
    // A property that is no array keeps what it is.
    if stored.is_some_and(|m| m.as_array().is_none()) {
        return String::new();
    }
    DEFAULT_MESSAGES.iter().find(|(key, _)| *key == status).map(|(_, text)| tr(text)).unwrap_or_default()
}

/// The rules of the form with its messages.
pub fn schema(env: &Env<'_>, tags: &Tags<'_>) -> Vec<swv::Rule> {
    let message = |status: &str| filter_message(&stored_message(env.form, status, env.lang.tr), tags);
    let messages = Messages {
        message: &message,
        undefined_value: filter_message(&(env.lang.tr)("Undefined value was submitted through this field."), tags),
        too_many_items: filter_message(&(env.lang.tr)("Too many items are selected."), tags),
    };
    swv::schema(env.tags, &messages, env.today)
}

/// `wpcf7_canonicalize( $text, array( 'strto' => 'as-is' ) )`.
pub fn canonicalize_as_is(text: &str) -> String {
    static SEPARATORS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\r\n\t ]+").unwrap());
    let text = rustpress_core::formatting::html_entity_decode(text);
    strip_whitespaces(&SEPARATORS.replace_all(&text, " "), Side::Both).to_owned()
}

/// `WPCF7_Pipes::do_pipe()` over the pipes of every tag named `name`.
fn do_pipe(tags: &[FormTag], name: &str, input: &str) -> String {
    let wanted = canonicalize_as_is(input);
    tags.iter().filter(|t| t.name == name).flat_map(|t| t.pipes.iter()).find(|p| canonicalize_as_is(&p.before) == wanted).map_or_else(|| input.to_owned(), |p| p.after.clone())
}

/// `wpcf7_antiscript_file_name()`.
pub fn antiscript_file_name(name: &str) -> String {
    static DASHES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\x09-\x0D\x20\xA0\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}\x{FEFF}-]+").unwrap());
    static INVISIBLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\pC\pZ]+").unwrap());
    let name = strip_whitespaces(name, Side::Both);
    let name = name.rsplit(['/', '\\']).next().unwrap_or_default();
    let removed: String = name.chars().filter(|c| !"?[]/\\=<>:;,'\"&$#*()|~`!{}%+’«»”“\0".contains(*c)).collect();
    let name = DASHES.replace_all(&removed, "-");
    let name = INVISIBLE.replace_all(&name, "").into_owned();
    let mut parts: Vec<&str> = name.split('.').collect();
    if parts.len() < 2 {
        return name;
    }
    let mut safelist: Vec<&str> = Vec::new();
    for group in ["audio/*", "video/*", "image/*"] {
        safelist.extend(swv::mime_extensions(group));
    }
    safelist.extend(["pdf", "txt"]);
    let safe = |part: &str| safelist.contains(&part.to_lowercase().as_str());
    let first = parts.remove(0).to_owned();
    let extension = parts.pop().unwrap_or_default();
    let mut out = first;
    for part in parts {
        out.push('.');
        out.push_str(part);
        if !safe(part) {
            out.push('_');
        }
    }
    if safe(extension) {
        out.push('.');
        out.push_str(extension);
    } else {
        out.push_str(&format!(".{extension}_.txt"));
    }
    out
}

/// The type PHPMailer gives a file by its extension.
fn mime_of(filename: &str) -> String {
    let ext = filename.rsplit_once('.').map(|(_, e)| e.to_lowercase()).unwrap_or_default();
    let known = [("pdf", "application/pdf"), ("txt", "text/plain"), ("jpg", "image/jpeg"), ("jpeg", "image/jpeg"), ("png", "image/png"), ("gif", "image/gif"), ("webp", "image/webp"), ("zip", "application/zip"), ("doc", "application/msword")];
    known.iter().find(|(e, _)| *e == ext).map_or("application/octet-stream", |(_, t)| t).to_owned()
}

/// What the browser posted, as `setup_posted_data()` leaves it.
fn setup_posted_data(env: &Env<'_>, post: &PostData) -> PostData {
    let mut posted = PostData(post.0.iter().filter(|(k, _)| !k.starts_with('_')).map(|(k, v)| (k.clone(), clean(v))).collect());
    let mut seen: Vec<&str> = Vec::new();
    for tag in env.tags {
        let f = features(&tag.tag_type);
        if !f.name_attr || f.not_for_mail || tag.name.is_empty() || seen.contains(&tag.name.as_str()) {
            continue;
        }
        seen.push(&tag.name);
        let mut value = posted.get(&tag.name).cloned().unwrap_or(Posted::Str(String::new()));
        if f.selectable_values {
            let items: Vec<(String, Posted)> = match value {
                Posted::Str(s) if s.is_empty() => Vec::new(),
                Posted::Str(s) => vec![("0".to_owned(), Posted::Str(s))],
                Posted::Map(items) => items,
            };
            let piped = items.into_iter().map(|(k, v)| (k, v.map(&|s| do_pipe(env.tags, &tag.name, s)))).collect::<Vec<_>>();
            value = Posted::Map(piped);
        }
        // `wpcf7_posted_data_checkbox()`: the text of a `free_text` option joins the last value.
        if matches!(tag.tag_type.as_str(), "checkbox" | "checkbox*" | "radio") && tag.has_option("free_text") {
            let mut items = match value {
                Posted::Map(items) => items,
                Posted::Str(s) => vec![("0".to_owned(), Posted::Str(s))],
            };
            if let Some((_, last)) = items.pop() {
                let free_text = post.string(&format!("_wpcf7_free_text_{}", tag.name));
                let last = last.flatten().into_iter().next().unwrap_or_default();
                let next = items.iter().filter_map(|(k, _)| k.parse::<i64>().ok()).max().map_or(0, |n| n + 1);
                items.push((next.to_string(), Posted::Str(super::formatter::php_trim(&format!("{last} {free_text}")).to_owned())));
            }
            value = Posted::Map(items);
        }
        match posted.0.iter_mut().find(|(k, _)| *k == tag.name) {
            Some(slot) => slot.1 = value,
            None => posted.0.push((tag.name.clone(), value)),
        }
    }
    posted
}

/// Runs a submission up to the mail (`proceed()` before `before_send_mail()`). The status stays `init` when the
/// mail is to be sent.
pub fn prepare(env: &Env<'_>, request: &Request<'_>) -> Submission {
    let post = PostData::from_fields(request.fields);
    let posted = setup_posted_data(env, &post);
    let mut flat: Vec<String> = vec![(request.now as f64 / 1800.0).ceil().to_string(), request.remote_ip.clone(), request.unit_tag.clone()];
    for (_, value) in &posted.0 {
        flat.extend(value.flatten());
    }
    let posted_hash = (env.hash)(&super::posted::flat_join(&flat, ", "), "wpcf7_submission");
    let mut sub = Submission { status: "init".into(), response: String::new(), post, posted, posted_hash, invalid: Vec::new(), uploaded: Vec::new() };
    let submitted_files: Vec<SubmittedFile> = request.files.iter().filter(|f| !f.filename.is_empty()).map(|f| SubmittedFile { field: f.field.trim_end_matches("[]").to_owned(), name: strip_whitespaces(&f.filename, Side::Both).to_owned(), size: f.bytes.len() as u64 }).collect();

    let names = sub.uploaded_names();
    let tags_ctx = Tags { tags: env.tags, posted: Some(&sub.posted), post: &sub.post, uploaded: &names, special: env.special, lang: env.lang, format_date: env.format_date };
    let message = |status: &str| filter_message(&stored_message(env.form, status, env.lang.tr), &tags_ctx);

    // `validate()`: the schema, then the modules' filters of each tag that is not a file.
    let mut invalid: Vec<Invalid> = Vec::new();
    let invalidate = |invalid: &mut Vec<Invalid>, tag: &FormTag, reason: String| {
        if tag.name.is_empty() || !super::form_tag::is_name(&tag.name) || invalid.iter().any(|i| i.field == tag.name) {
            return;
        }
        let idref = tag.get_option("id", "id").filter(|id| super::form_tag::is_name(id) && !id.starts_with("wpcf7"));
        invalid.push(Invalid { field: tag.name.clone(), reason, idref });
    };
    let rules = schema(env, &tags_ctx);
    let data = FormData { post: &sub.post, files: &submitted_files };
    for (field, error) in swv::validate(&rules, &data) {
        if let Some(tag) = env.tags.iter().find(|t| t.name == field) {
            invalidate(&mut invalid, tag, error);
        }
    }
    let acceptance_as_validation = env.form.is_true("acceptance_as_validation");
    for tag in env.tags.iter().filter(|t| !features(&t.tag_type).file_uploading) {
        match tag.tag_type.as_str() {
            "quiz" => {
                let answer = super::controls::canonicalize(&sub.post.string(&tag.name), true);
                let expected = sub.post.string(&format!("_wpcf7_quiz_answer_{}", tag.name));
                if (env.hash)(&answer, "wpcf7_quiz") != expected {
                    invalidate(&mut invalid, tag, message("quiz_answer_not_correct"));
                }
            }
            "acceptance" if acceptance_as_validation && !tag.has_option("optional") => {
                let value = sub.post.get(&tag.name).map(clean).is_some_and(|v| v.is_truthy());
                if tag.has_option("invert") == value {
                    invalidate(&mut invalid, tag, message("accept_terms"));
                }
            }
            _ => {}
        }
    }
    let (mut status, mut response) = (String::from("init"), String::new());
    if !invalid.is_empty() {
        status = "validation_failed".into();
        response = message("validation_error");
    }
    // `wpcf7_acceptance_filter()`.
    if status == "init" {
        let accepted = env.tags.iter().filter(|t| t.tag_type == "acceptance" && !t.name.is_empty() && !t.has_option("optional")).all(|tag| {
            let value = sub.post.get(&tag.name).map(clean).is_some_and(|v| v.is_truthy());
            tag.has_option("invert") != value
        });
        if !accepted {
            status = "acceptance_missing".into();
            response = message("accept_terms");
        }
    }
    if status == "init" && is_spam(env, &sub, request) {
        status = "spam".into();
        response = message("spam");
    }
    sub.invalid = invalid;
    sub.status = status;
    sub.response = response;
    // `unship_uploaded_files()`: the files of the file fields, renamed safely.
    if sub.is("init") {
        for tag in env.tags.iter().filter(|t| features(&t.tag_type).file_uploading) {
            let files: Vec<Attachment> = request
                .files
                .iter()
                .filter(|f| f.field.trim_end_matches("[]") == tag.name && !f.filename.is_empty())
                .map(|f| {
                    let filename = antiscript_file_name(&canonicalize_as_is(&f.filename));
                    Attachment { content_type: mime_of(&filename), filename, bytes: f.bytes.clone() }
                })
                .collect();
            if files.is_empty() {
                continue;
            }
            let hashes: Vec<String> = files.iter().map(|f| sha256_bytes(&f.bytes)).collect();
            let empty = sub.posted.get(&tag.name).is_none_or(|v| !v.is_truthy());
            if empty {
                let value = Posted::Str(hashes.join(" "));
                match sub.posted.0.iter_mut().find(|(k, _)| *k == tag.name) {
                    Some(slot) => slot.1 = value,
                    None => sub.posted.0.push((tag.name.clone(), value)),
                }
            }
            sub.uploaded.push((tag.name.clone(), files));
        }
    }
    sub
}

/// The mails of a submission that got through (`mail()`): `mail`, then `mail_2` when it is active.
pub fn mails(env: &Env<'_>, sub: &Submission) -> Vec<Email> {
    let names = sub.uploaded_names();
    let special = Special { invalid_fields: sub.invalid.len(), ..env.special.clone() };
    let tags = Tags { tags: env.tags, posted: Some(&sub.posted), post: &sub.post, uploaded: &names, special: &special, lang: env.lang, format_date: env.format_date };
    let content_file = |_: &str| None;
    let compose = Compose { tags: &tags, locale: &env.form.locale, files: &sub.uploaded, content_file: &content_file };
    let mut template = env.form.mail.as_ref().map(Template::from_php).unwrap_or_default();
    // `upgrade()`: a mail without a recipient goes to the administrator.
    if env.form.mail.as_ref().is_some_and(|m| m.as_array().is_some() && m.get("recipient").is_none()) {
        template.recipient = env.special.admin_email.clone();
    }
    let mut out = vec![compose_mail(&template, &compose)];
    if let Some(second) = env.form.mail_2.as_ref().map(Template::from_php).filter(|t| t.active) {
        out.push(compose_mail(&second, &compose));
    }
    out
}

/// The status and message once the mail was sent, or not.
pub fn conclude(env: &Env<'_>, sub: &mut Submission, sent: bool) {
    sub.status = if sent { "mail_sent" } else { "mail_failed" }.to_owned();
    let names = sub.uploaded_names();
    let tags = Tags { tags: env.tags, posted: Some(&sub.posted), post: &sub.post, uploaded: &names, special: env.special, lang: env.lang, format_date: env.format_date };
    sub.response = filter_message(&stored_message(env.form, if sent { "mail_sent_ok" } else { "mail_sent_ng" }, env.lang.tr), &tags);
}

/// `hash_file( 'sha256', $path )`.
fn sha256_bytes(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// `spam()`: the checks of the plugin and of `wpcf7_disallowed_list()`.
fn is_spam(env: &Env<'_>, sub: &Submission, request: &Request<'_>) -> bool {
    if env.form.is_true("subscribers_only") && env.user.is_some() {
        return false;
    }
    let mut spam = request.user_agent.len() < 2;
    // `verify_nonce()`: only forms for subscribers check the nonce.
    if env.form.is_true("subscribers_only")
        && let Some(user) = env.user
        && !user.check_nonce(&sub.post.string("_wpnonce"), "wp_rest")
    {
        spam = true;
    }
    if spam {
        return true;
    }
    let mut target: Vec<String> = sub.posted.0.iter().flat_map(|(_, v)| v.flatten()).collect();
    target.push(request.remote_ip.clone());
    target.push(request.user_agent.clone());
    let target = target.join("\n").to_ascii_lowercase();
    env.disallowed_keys.trim().split('\n').map(str::trim).filter(|w| (2..=256).contains(&w.len())).any(|word| target.contains(&word.to_ascii_lowercase()))
}

/// The unit tag as `wpcf7_sanitize_unit_tag()` leaves it.
pub fn sanitize_unit_tag(tag: &str) -> String {
    tag.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_lose_what_could_run() {
        assert_eq!(antiscript_file_name("my file (1).pdf"), "my-file-1.pdf");
        assert_eq!(antiscript_file_name("evil.php"), "evil.php_.txt");
        assert_eq!(antiscript_file_name("a.php.jpg"), "a.php_.jpg");
        assert_eq!(antiscript_file_name("../../x.txt"), "x.txt");
        assert_eq!(antiscript_file_name("noext"), "noext");
    }

    #[test]
    fn unit_tags() {
        assert_eq!(sanitize_unit_tag("wpcf7-f5-p2-o1<script>"), "wpcf7-f5-p2-o1script");
    }
}
