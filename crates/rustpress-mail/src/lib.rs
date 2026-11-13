//! `wp_mail()`: a message over SMTP, or through the host's `sendmail` as PHP's `mail()` sends it.
//!
//! The server is named by `RUSTPRESS_SMTP_URL` (`smtp://user:password@host:25`, `smtps://...` for TLS from the start,
//! `smtp://host:587?tls=required` for STARTTLS); the sender by `RUSTPRESS_MAIL_FROM` (an address, or
//! `Name <address>`). Without a server the message goes to `sendmail` (`RUSTPRESS_SENDMAIL`, else
//! `/usr/sbin/sendmail` or `/usr/lib/sendmail`, the `sendmail_path` PHP is built with), which is what `wp_mail()`
//! does through PHPMailer's `mail()` on shared hosting. `RUSTPRESS_SENDMAIL=off`, or a host without `sendmail`,
//! sends nothing, and every message fails, as `wp_mail()` does on a host that cannot send e-mail.

use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart, header::ContentType};
use lettre::{AsyncSendmailTransport, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

/// The places PHP's `sendmail_path` points to on Linux, in the order `configure` looks for them.
const SENDMAIL_PATHS: [&str; 2] = ["/usr/sbin/sendmail", "/usr/lib/sendmail"];

/// How a message leaves the host.
enum Transport {
    Smtp(AsyncSmtpTransport<Tokio1Executor>),
    Sendmail(AsyncSendmailTransport<Tokio1Executor>),
}

/// Where e-mail goes and who it is from.
pub struct Mailer {
    transport: Transport,
    from: Mailbox,
    /// A plugin's filters of the sender (WP Mail SMTP); without them the site's address stays.
    sender: Option<SenderFilter>,
    /// The `wordpress@` address of the site, which `wp_mail()` uses when no `From` header names one.
    default_email: String,
    /// Every message fails without being sent.
    blocked: bool,
}

/// An SMTP server a plugin configured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Smtp {
    pub host: String,
    pub port: u16,
    pub security: Security,
    pub credentials: Option<(String, String)>,
}

/// TLS for [`Smtp`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Security {
    /// TLS from the start (PHPMailer's `ssl`).
    Implicit,
    /// STARTTLS, required (`tls`).
    StartTls,
    /// STARTTLS when the server offers it (`SMTPAutoTLS`).
    Opportunistic,
    Plain,
}

/// How a plugin sends: through its server, through `sendmail`, or (`None`) as configured in the environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    Smtp(Smtp),
    Sendmail,
}

/// The sender of one message: what `wp_mail()` takes from the headers, before the filters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sender {
    pub email: String,
    pub name: String,
    pub default_email: String,
    pub reply_to: Vec<String>,
    pub headers: Vec<(String, String)>,
}

/// `wp_mail_from`, `wp_mail_from_name` and what `phpmailer_init` adds.
pub type SenderFilter = std::sync::Arc<dyn Fn(Sender) -> Sender + Send + Sync>;

impl Mailer {
    /// From the environment; `None` when e-mail cannot be sent (no SMTP server and no `sendmail`). `default_host`
    /// names the site (for the sender `wordpress@host`).
    pub fn from_env(default_host: &str) -> Option<Self> {
        Self::configured(default_host, None, None, false)
    }

    /// With a plugin's settings: its route (else the environment's), its filters of the sender, and whether it
    /// blocks every message.
    pub fn configured(default_host: &str, route: Option<Route>, sender: Option<SenderFilter>, blocked: bool) -> Option<Self> {
        let transport = match route {
            Some(Route::Smtp(smtp)) => Transport::Smtp(smtp_transport(&smtp)?),
            Some(Route::Sendmail) => Transport::Sendmail(AsyncSendmailTransport::new_with_command(sendmail_command(std::env::var("RUSTPRESS_SENDMAIL").ok().as_deref(), |p| std::path::Path::new(p).is_file())?)),
            None => Self::env_transport()?,
        };
        let host = default_host.trim_start_matches("www.");
        let from = std::env::var("RUSTPRESS_MAIL_FROM").ok().filter(|f| !f.is_empty()).unwrap_or_else(|| format!("WordPress <wordpress@{host}>"));
        match from.parse::<Mailbox>() {
            Ok(from) => Some(Self { transport, from, sender, default_email: format!("wordpress@{host}"), blocked }),
            Err(e) => {
                tracing::error!(error = %e, "RUSTPRESS_MAIL_FROM is not a valid sender");
                None
            }
        }
    }

    fn env_transport() -> Option<Transport> {
        Some(match std::env::var("RUSTPRESS_SMTP_URL").ok().filter(|u| !u.is_empty()) {
            Some(url) => match AsyncSmtpTransport::<Tokio1Executor>::from_url(&url) {
                Ok(builder) => Transport::Smtp(builder.build()),
                Err(e) => {
                    tracing::error!(error = %e, "RUSTPRESS_SMTP_URL is not a valid address");
                    return None;
                }
            },
            None => Transport::Sendmail(AsyncSendmailTransport::new_with_command(sendmail_command(std::env::var("RUSTPRESS_SENDMAIL").ok().as_deref(), |p| std::path::Path::new(p).is_file())?)),
        })
    }

    /// Sends `message` (plain text, UTF-8) to `to`; `false` when it could not be sent.
    pub async fn send(&self, to: &str, subject: &str, message: &str) -> bool {
        self.send_with_headers(to, subject, message, "").await
    }

    /// The same with the header lines of `headers` (`"From: ...\nReply-To: ..."`).
    pub async fn send_with_headers(&self, to: &str, subject: &str, message: &str, headers: &str) -> bool {
        let headers: Vec<&str> = headers.lines().collect();
        self.send_mail(to, subject, message, &headers, &[]).await
    }

    /// `wp_mail( $to, $subject, $message, $headers, $attachments )`: `to` may name several addresses separated by
    /// commas; of the headers `From` gives the name (the address of the site stays), `Content-Type: text/html` makes an
    /// HTML message, `Cc`, `Bcc` and `Reply-To` add addresses, and any other header is passed on.
    pub async fn send_mail(&self, to: &str, subject: &str, message: &str, headers: &[&str], attachments: &[Attached<'_>]) -> bool {
        if self.blocked {
            return false;
        }
        let Some(email) = self.build(to, subject, message, headers, attachments) else { return false };
        let sent = match &self.transport {
            Transport::Smtp(smtp) => smtp.send(email).await.map(|_| ()).map_err(|e| e.to_string()),
            Transport::Sendmail(sendmail) => sendmail.send(email).await.map_err(|e| e.to_string()),
        };
        match sent {
            Ok(()) => true,
            Err(e) => {
                tracing::warn!(error = %e, "e-mail could not be sent");
                false
            }
        }
    }

    fn build(&self, to: &str, subject: &str, message: &str, headers: &[&str], attachments: &[Attached<'_>]) -> Option<Message> {
        match &self.sender {
            Some(filter) => build_filtered(filter, &self.default_email, to, subject, message, headers, attachments),
            None => build_message(&self.from, to, subject, message, headers, attachments),
        }
    }
}

/// The transport of a plugin's SMTP server, as PHPMailer connects: `ssl` with TLS from the start, `tls` with
/// STARTTLS, else STARTTLS when offered (`SMTPAutoTLS`) or none.
fn smtp_transport(smtp: &Smtp) -> Option<AsyncSmtpTransport<Tokio1Executor>> {
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::transport::smtp::client::{Tls, TlsParameters};
    let tls = || match TlsParameters::new(smtp.host.clone()) {
        Ok(params) => Some(params),
        Err(e) => {
            tracing::error!(error = %e, host = smtp.host, "no TLS for the SMTP server");
            None
        }
    };
    let tls = match smtp.security {
        Security::Implicit => Tls::Wrapper(tls()?),
        Security::StartTls => Tls::Required(tls()?),
        Security::Opportunistic => Tls::Opportunistic(tls()?),
        Security::Plain => Tls::None,
    };
    let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(smtp.host.as_str()).port(smtp.port).tls(tls).timeout(Some(std::time::Duration::from_secs(30)));
    if let Some((user, password)) = &smtp.credentials {
        builder = builder.credentials(Credentials::new(user.clone(), password.clone()));
    }
    Some(builder.build())
}

/// `wp_mail()`'s parsing of the `From` header: `Name <address>` (quotes removed from the name), or an address.
fn header_from(value: &str) -> (Option<String>, Option<String>) {
    match value.find('<') {
        Some(at) => {
            let name = (at > 0).then(|| value[..at].replace('"', "").trim().to_owned());
            (name, Some(value[at + 1..].replace('>', "").trim().to_owned()))
        }
        None if !value.trim().is_empty() => (None, Some(value.trim().to_owned())),
        None => (None, None),
    }
}

/// A message whose sender a plugin's filters decide (`wp_mail_from`, `wp_mail_from_name`, `phpmailer_init`).
fn build_filtered(filter: &SenderFilter, default_email: &str, to: &str, subject: &str, message: &str, headers: &[&str], attachments: &[Attached<'_>]) -> Option<Message> {
    let (mut name, mut email) = (None, None);
    let mut reply_to = Vec::new();
    let mut rest: Vec<&str> = Vec::new();
    for line in headers {
        let Some((key, value)) = line.split_once(':') else { continue };
        match key.trim().to_lowercase().as_str() {
            "from" => (name, email) = header_from(value.trim()),
            "reply-to" => {
                reply_to.extend(value.split(',').map(str::trim).filter(|a| !a.is_empty()).map(str::to_owned));
                rest.push(line);
            }
            _ => rest.push(line),
        }
    }
    let sender = filter(Sender { email: email.unwrap_or_else(|| default_email.to_owned()), name: name.unwrap_or_else(|| "WordPress".to_owned()), default_email: default_email.to_owned(), reply_to: reply_to.clone(), headers: Vec::new() });
    let from = Mailbox::new(Some(sender.name.clone()).filter(|n| !n.is_empty()), sender.email.parse().ok()?);
    let added: Vec<String> = sender.reply_to.iter().filter(|a| !reply_to.contains(a)).map(|a| format!("Reply-To: {a}")).chain(sender.headers.iter().map(|(k, v)| format!("{k}: {v}"))).collect();
    rest.extend(added.iter().map(String::as_str));
    build_message_from(from, to, subject, message, &rest, attachments)
}

/// The `sendmail` program: the one `RUSTPRESS_SENDMAIL` names (`off` for none), else the first of
/// [`SENDMAIL_PATHS`] that exists.
fn sendmail_command(configured: Option<&str>, exists: impl Fn(&str) -> bool) -> Option<String> {
    match configured.map(str::trim).filter(|c| !c.is_empty()) {
        Some(off) if off.eq_ignore_ascii_case("off") => None,
        Some(path) => Some(path.to_owned()),
        None => SENDMAIL_PATHS.iter().find(|p| exists(p)).map(|p| (*p).to_owned()),
    }
}

/// A file attached to a message.
pub struct Attached<'a> {
    pub filename: &'a str,
    pub content_type: &'a str,
    pub bytes: &'a [u8],
}

/// The addresses of a comma-separated list that parse.
fn mailboxes(list: &str) -> Vec<Mailbox> {
    list.split(',').map(str::trim).filter(|a| !a.is_empty()).filter_map(|a| a.parse::<Mailbox>().ok()).collect()
}

fn build_message(site_from: &Mailbox, to: &str, subject: &str, message: &str, headers: &[&str], attachments: &[Attached<'_>]) -> Option<Message> {
    // The address of the site stays; the name is the one a `From` header gives.
    let mut from = site_from.clone();
    for line in headers {
        if let Some((name, value)) = line.split_once(':')
            && name.trim().eq_ignore_ascii_case("from")
            && let Ok(given) = value.trim().parse::<Mailbox>()
        {
            from = Mailbox::new(given.name, site_from.email.clone());
        }
    }
    build_message_from(from, to, subject, message, headers, attachments)
}

/// The message with its sender decided; a `From` header in `headers` is left out.
fn build_message_from(from: Mailbox, to: &str, subject: &str, message: &str, headers: &[&str], attachments: &[Attached<'_>]) -> Option<Message> {
    let recipients = mailboxes(to);
    if recipients.is_empty() {
        return None;
    }
    let mut builder = Message::builder().subject(subject);
    for recipient in recipients {
        builder = builder.to(recipient);
    }
    let mut html = false;
    for line in headers {
        let Some((name, value)) = line.split_once(':') else { continue };
        let value = value.trim();
        match name.trim().to_lowercase().as_str() {
            "from" => {}
            "reply-to" => {
                for reply in mailboxes(value) {
                    builder = builder.reply_to(reply);
                }
            }
            "cc" => {
                for cc in mailboxes(value) {
                    builder = builder.cc(cc);
                }
            }
            "bcc" => {
                for bcc in mailboxes(value) {
                    builder = builder.bcc(bcc);
                }
            }
            "content-type" => html = value.split(';').next().is_some_and(|t| t.trim().eq_ignore_ascii_case("text/html")),
            // Set by the builder.
            "to" | "subject" | "mime-version" | "content-transfer-encoding" | "date" | "message-id" => {}
            _ => {
                if let Ok(header) = lettre::message::header::HeaderName::new_from_ascii(name.trim().to_owned()) {
                    builder = builder.raw_header(lettre::message::header::HeaderValue::new(header, value.to_owned()));
                }
            }
        }
    }
    let body = message.replace("\r\n", "\n").replace('\n', "\r\n");
    let text = if html { SinglePart::builder().header(ContentType::TEXT_HTML).body(body) } else { SinglePart::builder().header(ContentType::TEXT_PLAIN).body(body) };
    let builder = builder.from(from);
    if attachments.is_empty() {
        return builder.singlepart(text).ok();
    }
    let mut parts = MultiPart::mixed().singlepart(text);
    for file in attachments {
        let content_type = ContentType::parse(file.content_type).unwrap_or_else(|_| ContentType::parse("application/octet-stream").expect("a valid type"));
        parts = parts.singlepart(Attachment::new(file.filename.to_owned()).body(file.bytes.to_vec(), content_type));
    }
    builder.multipart(parts).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn formatted(message: &Message) -> String {
        String::from_utf8_lossy(&message.formatted()).into_owned()
    }

    #[test]
    fn headers_of_wp_mail() {
        let site: Mailbox = "WordPress <wordpress@example.org>".parse().unwrap();
        let headers = ["From: Site <other@example.net>", "Reply-To: Jane <jane@example.com>", "Cc: a@example.com, b@example.com", "X-Extra: yes"];
        let message = build_message(&site, "x@example.com, y@example.com", "Hi", "line\nline", &headers, &[]).unwrap();
        let text = formatted(&message);
        assert!(text.contains("From: Site <wordpress@example.org>"), "{text}");
        assert!(text.contains("Reply-To: Jane <jane@example.com>"), "{text}");
        assert!(text.contains("To: x@example.com, y@example.com"), "{text}");
        assert!(text.contains("Cc: a@example.com, b@example.com"), "{text}");
        assert!(text.contains("X-Extra: yes"), "{text}");
        assert!(text.contains("Content-Type: text/plain; charset=utf-8"), "{text}");
        assert!(text.contains("line\r\nline"), "{text}");
    }

    #[test]
    fn html_and_attachments() {
        let site: Mailbox = "wordpress@example.org".parse().unwrap();
        let file = Attached { filename: "a.txt", content_type: "text/plain", bytes: b"hello" };
        let message = build_message(&site, "x@example.com", "Hi", "<p>Hi</p>", &["Content-Type: text/html"], &[file]).unwrap();
        let text = formatted(&message);
        assert!(text.contains("multipart/mixed"), "{text}");
        assert!(text.contains("Content-Type: text/html; charset=utf-8"), "{text}");
        assert!(text.contains("filename=\"a.txt\""), "{text}");
    }

    #[test]
    fn sendmail_like_php() {
        assert_eq!(sendmail_command(None, |p| p == "/usr/lib/sendmail").as_deref(), Some("/usr/lib/sendmail"));
        assert_eq!(sendmail_command(None, |_| true).as_deref(), Some("/usr/sbin/sendmail"));
        assert_eq!(sendmail_command(None, |_| false), None);
        assert_eq!(sendmail_command(Some("off"), |_| true), None);
        assert_eq!(sendmail_command(Some("/opt/bin/msmtp"), |_| false).as_deref(), Some("/opt/bin/msmtp"));
    }

    #[test]
    fn a_plugin_decides_the_sender() {
        let filter: SenderFilter = std::sync::Arc::new(|mut sender: Sender| {
            if sender.email == sender.default_email {
                sender.email = "site@example.org".into();
                sender.name = "Die Seite".into();
            }
            sender.reply_to.push("jemand@example.net".into());
            sender.headers.push(("X-Mailer".into(), "WPMailSMTP/Mailer/smtp 4.10.4".into()));
            sender
        });
        let text = formatted(&build_filtered(&filter, "wordpress@example.org", "x@example.com", "Hi", "x", &[], &[]).unwrap());
        assert!(text.contains("From: \"Die Seite\" <site@example.org>") || text.contains("From: Die Seite <site@example.org>"), "{text}");
        assert!(text.contains("Reply-To: jemand@example.net"), "{text}");
        assert!(text.contains("X-Mailer: WPMailSMTP/Mailer/smtp 4.10.4"), "{text}");
        let text = formatted(&build_filtered(&filter, "wordpress@example.org", "x@example.com", "Hi", "x", &["From: \"Jemand\" <jemand@example.net>"], &[]).unwrap());
        assert!(text.contains("From: Jemand <jemand@example.net>"), "{text}");
    }

    #[test]
    fn no_recipient_no_message() {
        let site: Mailbox = "wordpress@example.org".parse().unwrap();
        assert!(build_message(&site, "not an address", "Hi", "x", &[], &[]).is_none());
    }
}
