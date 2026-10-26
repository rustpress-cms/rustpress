//! `wp_mail()`: a plain text message over SMTP.
//!
//! The server is named by `RUSTPRESS_SMTP_URL` (`smtp://user:password@host:25`, `smtps://...` for TLS from the start,
//! `smtp://host:587?tls=required` for STARTTLS); the sender by `RUSTPRESS_MAIL_FROM` (an address, or
//! `Name <address>`). Without a server nothing is sent and every message fails, as `wp_mail()` does on a host that
//! cannot send e-mail.

use lettre::message::{Mailbox, header::ContentType};
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

    /// The same with the `From:` and `Reply-To:` lines of `headers` (`"From: ...\nReply-To: ..."`).
    pub async fn send_with_headers(&self, to: &str, subject: &str, message: &str, headers: &str) -> bool {
        let Ok(to) = to.parse::<Mailbox>() else { return false };
        let body = message.replace("\r\n", "\n").replace('\n', "\r\n");
        let mut builder = Message::builder().to(to).subject(subject).header(ContentType::TEXT_PLAIN);
        let mut from = self.from.clone();
        for line in headers.lines() {
            let Some((name, value)) = line.split_once(':') else { continue };
            match name.trim().to_lowercase().as_str() {
                // The address of the site stays; the name is the one the header gives.
                "from" => {
                    if let Ok(given) = value.trim().parse::<Mailbox>() {
                        from = Mailbox::new(given.name, self.from.email.clone());
                    }
                }
                "reply-to" => {
                    if let Ok(reply) = value.trim().parse::<Mailbox>() {
                        builder = builder.reply_to(reply);
                    }
                }
                _ => {}
            }
        }
        let Ok(email) = builder.from(from).body(body) else { return false };
        match self.transport.send(email).await {
            Ok(_) => true,
            Err(e) => {
                tracing::warn!(error = %e, "e-mail could not be sent");
                false
            }
        }
    }
}
