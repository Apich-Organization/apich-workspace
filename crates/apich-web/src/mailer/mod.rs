use crate::error::{WebError, WebResult};
use apich_db::SystemSettings;
use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentEmail {
    pub to: String,
    pub subject: String,
    pub body: String,
}

#[derive(Clone)]
pub struct MailerService {
    sent_emails: Arc<Mutex<Vec<SentEmail>>>,
}

impl Default for MailerService {
    fn default() -> Self {
        Self::new()
    }
}

impl MailerService {
    pub fn new() -> Self {
        Self {
            sent_emails: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Retrieve sent emails (especially useful for tests and audit inspections)
    pub fn get_sent_emails(&self) -> Vec<SentEmail> {
        self.sent_emails.lock().unwrap().clone()
    }

    /// Clear recorded sent emails
    pub fn clear_sent_emails(&self) {
        self.sent_emails.lock().unwrap().clear();
    }

    /// Send an invitation email to a user
    pub async fn send_invitation(
        &self,
        settings: &SystemSettings,
        to_email: &str,
        invite_token: &str,
        inviter_name: &str,
        node_name: Option<&str>,
        base_url: &str,
    ) -> WebResult<()> {
        let registration_url = format!("{}/register?token={}", base_url, invite_token);
        let subject = format!("Invitation to join APICH Workspace from {}", inviter_name);
        let target = node_name.unwrap_or("the platform");
        let body = format!(
            "Hello,\n\n{} has invited you to join {} on APICH Technical & Academic Workspace.\n\n\
            Click the link below or copy it to your browser to complete your registration:\n\
            {}\n\n\
            This invitation link is valid for 7 days.\n\n\
            Welcome to the APICH research environment!",
            inviter_name, target, registration_url
        );

        self.send_mail(settings, to_email, &subject, &body).await
    }

    /// Send a general email, dispatching via real SMTP if enabled or recording to in-memory queue
    pub async fn send_mail(
        &self,
        settings: &SystemSettings,
        to: &str,
        subject: &str,
        body: &str,
    ) -> WebResult<()> {
        info!(to = %to, subject = %subject, "Dispatching email notification");

        // Always record in sent log for testing & audit
        self.sent_emails.lock().unwrap().push(SentEmail {
            to: to.to_string(),
            subject: subject.to_string(),
            body: body.to_string(),
        });

        // If SMTP is enabled and configured, send via Lettre
        if settings.smtp_enabled {
            let host = settings
                .smtp_host
                .as_deref()
                .ok_or_else(|| WebError::BadRequest("SMTP host not configured".to_string()))?;
            let port = settings.smtp_port.unwrap_or(587) as u16;
            let from_addr = settings
                .smtp_from_email
                .as_deref()
                .unwrap_or("noreply@apich.org");
            let from_name = settings
                .smtp_from_name
                .as_deref()
                .unwrap_or("APICH Platform");

            let from_header = format!("\"{}\" <{}>", from_name, from_addr);

            let email = Message::builder()
                .from(
                    from_header
                        .parse()
                        .map_err(|e| WebError::Internal(format!("Invalid from address: {}", e)))?,
                )
                .to(to
                    .parse()
                    .map_err(|e| WebError::BadRequest(format!("Invalid recipient email: {}", e)))?)
                .subject(subject)
                .header(ContentType::TEXT_PLAIN)
                .body(body.to_string())
                .map_err(|e| WebError::Internal(format!("Failed to build email message: {}", e)))?;

            let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host).port(port);

            if let (Some(user), Some(pwd)) = (&settings.smtp_username, &settings.smtp_password) {
                if !user.is_empty() && !pwd.is_empty() {
                    builder = builder.credentials(Credentials::new(user.clone(), pwd.clone()));
                }
            }

            let mailer = builder.build();
            mailer
                .send(email)
                .await
                .map_err(|e| WebError::Internal(format!("SMTP transport failure: {}", e)))?;
        }

        Ok(())
    }

    /// Send a test verification email using the configured or simulated settings
    pub async fn send_test_email(
        &self,
        settings: &SystemSettings,
        recipient: &str,
    ) -> WebResult<()> {
        let host = settings
            .smtp_host
            .as_deref()
            .filter(|h| !h.trim().is_empty());

        let subject = "[APICH] SMTP Delivery Test Verification";
        let body = format!(
            "Hello,\n\n\
            This is an automated test notification confirming that the outgoing SMTP mail server on APICH Technical & Academic Workspace is configured properly.\n\n\
            Configuration Details:\n\
            - Host: {}\n\
            - Port: {}\n\
            - Security (TLS): {}\n\
            - Sender: {} <{}>\n\
            - Status: Active & Operational\n\n\
            If you received this message, outbound emailing is fully functional!",
            host.unwrap_or("simulated-localhost"),
            settings.smtp_port.unwrap_or(587),
            if settings.smtp_use_tls { "Enabled" } else { "Disabled/Plain" },
            settings.smtp_from_name.as_deref().unwrap_or("APICH Platform"),
            settings.smtp_from_email.as_deref().unwrap_or("noreply@apich.org"),
        );

        // If host is configured and non-empty, attempt real connection & delivery
        if let Some(host) = host {
            let port = settings.smtp_port.unwrap_or(587) as u16;
            let from_addr = settings
                .smtp_from_email
                .as_deref()
                .unwrap_or("noreply@apich.org");
            let from_name = settings
                .smtp_from_name
                .as_deref()
                .unwrap_or("APICH Platform");

            let from_header = format!("\"{}\" <{}>", from_name, from_addr);

            let email = Message::builder()
                .from(
                    from_header
                        .parse()
                        .map_err(|e| WebError::Internal(format!("Invalid from address: {}", e)))?,
                )
                .to(recipient
                    .parse()
                    .map_err(|e| WebError::BadRequest(format!("Invalid recipient email: {}", e)))?)
                .subject(subject)
                .header(ContentType::TEXT_PLAIN)
                .body(body.clone())
                .map_err(|e| WebError::Internal(format!("Failed to build email message: {}", e)))?;

            let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host).port(port);

            if let (Some(ref user), Some(ref pwd)) = (&settings.smtp_username, &settings.smtp_password) {
                if !user.is_empty() && !pwd.is_empty() {
                    builder = builder.credentials(Credentials::new(user.clone(), pwd.clone()));
                }
            }

            let mailer = builder.build();
            mailer
                .send(email)
                .await
                .map_err(|e| WebError::Internal(format!("SMTP transport failure: {}", e)))?;
        }

        // Always record in sent log for test assertions & auditing
        self.sent_emails.lock().unwrap().push(SentEmail {
            to: recipient.to_string(),
            subject: subject.to_string(),
            body,
        });

        Ok(())
    }
}

