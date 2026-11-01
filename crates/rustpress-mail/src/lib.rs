//! `wp_mail()`: a message over SMTP.
//!
//! The server is named by `RUSTPRESS_SMTP_URL` (`smtp://user:password@host:25`, `smtps://...` for TLS from the start,
//! `smtp://host:587?tls=required` for STARTTLS); the sender by `RUSTPRESS_MAIL_FROM` (an address, or
//! `Name <address>`). Without a server nothing is sent and every message fails, as `wp_mail()` does on a host that
//! cannot send e-mail.

use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart, header::ContentType};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

/// Where e-mail goes and who it is from.
pub struct Mailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl Mailer {
    /// From the environment; `None` when no SMTP server is configured. `default_host` names the site (for the
    /// sender `wordpress@host`).
    pub fn from_env(default_host: &str) -> Option<Self> {
        let url = std::env::var("RUSTPRESS_SMTP_URL").ok().filter(|u| !u.is_empty())?;
        let transport = match AsyncSmtpTransport::<Tokio1Executor>::from_url(&url) {
            Ok(builder) => builder.build(),
            Err(e) => {
                tracing::error!(error = %e, "RUSTPRESS_SMTP_URL is not a valid address");
                return None;
            }
        };
        let host = default_host.trim_start_matches("www.");
        let from = std::env::var("RUSTPRESS_MAIL_FROM").ok().filter(|f| !f.is_empty()).unwrap_or_else(|| format!("WordPress <wordpress@{host}>"));
        match from.parse::<Mailbox>() {
            Ok(from) => Some(Self { transport, from }),
            Err(e) => {
                tracing::error!(error = %e, "RUSTPRESS_MAIL_FROM is not a valid sender");
                None
            }
        }
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
        let Some(email) = self.build(to, subject, message, headers, attachments) else { return false };
        match self.transport.send(email).await {
            Ok(_) => true,
            Err(e) => {
                tracing::warn!(error = %e, "e-mail could not be sent");
                false
            }
        }
    }

    fn build(&self, to: &str, subject: &str, message: &str, headers: &[&str], attachments: &[Attached<'_>]) -> Option<Message> {
        build_message(&self.from, to, subject, message, headers, attachments)
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
    let recipients = mailboxes(to);
    if recipients.is_empty() {
        return None;
    }
    let mut builder = Message::builder().subject(subject);
    for recipient in recipients {
        builder = builder.to(recipient);
    }
    let mut from = site_from.clone();
    let mut html = false;
    for line in headers {
        let Some((name, value)) = line.split_once(':') else { continue };
        let value = value.trim();
        match name.trim().to_lowercase().as_str() {
            // The address of the site stays; the name is the one the header gives.
            "from" => {
                if let Ok(given) = value.parse::<Mailbox>() {
                    from = Mailbox::new(given.name, site_from.email.clone());
                }
            }
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
    fn no_recipient_no_message() {
        let site: Mailbox = "wordpress@example.org".parse().unwrap();
        assert!(build_message(&site, "not an address", "Hi", "x", &[], &[]).is_none());
    }
}
