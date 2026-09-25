use crate::error::WebError;
use crate::error::WebResult;
use apich_db::SystemSettings;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::Tls;
use lettre::transport::smtp::client::TlsParameters;
use lettre::AsyncSmtpTransport;
use lettre::AsyncTransport;
use lettre::Message;
use lettre::Tokio1Executor;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::info;

/// Record of a dispatched email message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentEmail {
    /// Recipient email address.
    pub to: String,
    /// Subject line.
    pub subject: String,
    /// Message body content.
    pub body: String,
}

/// Service for preparing and dispatching notification and invitation emails.
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
    /// Creates a new mailer service instance.
    pub fn new() -> Self {
        Self {
            sent_emails: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Retrieve sent emails (especially useful for tests and audit inspections)
    pub fn get_sent_emails(&self) -> Vec<SentEmail> {
        self.sent_emails
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Clear recorded sent emails
    pub fn clear_sent_emails(&self) {
        self.sent_emails
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }

    /// Configure TLS transport mode based on system settings and host/port heuristics.
    fn build_tls_config(settings: &SystemSettings, host: &str, port: u16) -> WebResult<Tls> {
        let force_tls = settings.smtp_force_tls
            || std::env::var("SMTP_FORCE_TLS")
                .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
                .unwrap_or(false)
            || port == 465;

        if force_tls {
            let tls_params = TlsParameters::builder(host.to_string())
                .build()
                .map_err(|e| {
                    WebError::Internal(format!("Failed to configure Force TLS / SMTPS: {e}"))
                })?;
            Ok(Tls::Wrapper(tls_params))
        } else if settings.smtp_use_tls {
            let tls_params = TlsParameters::builder(host.to_string())
                .build()
                .map_err(|e| WebError::Internal(format!("Failed to configure STARTTLS: {e}")))?;
            Ok(Tls::Required(tls_params))
        } else {
            Ok(Tls::None)
        }
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
        let registration_url = format!("{base_url}/register?token={invite_token}");
        let subject = format!("Invitation to join APICH Workspace from {inviter_name}");
        let target = node_name.unwrap_or("the platform");
        let body = format!(
            "Hello,\n\n{inviter_name} has invited you to join {target} on APICH Technical & Academic Workspace.\n\n\
            Click the link below or copy it to your browser to complete your registration:\n\
            {registration_url}\n\n\
            This invitation link is valid for 7 days.\n\n\
            Welcome to the APICH research environment!"
        );

        self.send_mail(settings, to_email, &subject, &body).await
    }

    /// Send a welcome email to a newly registered user
    pub async fn send_welcome_email(
        &self,
        settings: &SystemSettings,
        to_email: &str,
        username: &str,
        display_name: &str,
        base_url: &str,
    ) -> WebResult<()> {
        let subject = "Welcome to APICH Technical & Academic Workspace".to_string();
        let body = format!(
            "Hello {display_name},\n\n\
            Welcome to APICH Technical & Academic Workspace! Your account has been created successfully.\n\n\
            Username: {username}\n\
            Login URL: {base_url}/login\n\n\
            You can now start managing research notes, Typst & LaTeX papers, and collaborative slide decks.\n\n\
            Best regards,\n\
            The APICH Team"
        );
        self.send_mail(settings, to_email, &subject, &body).await
    }

    /// Send password reset notification email containing temporary credentials
    pub async fn send_password_reset_email(
        &self,
        settings: &SystemSettings,
        to_email: &str,
        username: &str,
        temp_password: &str,
        base_url: &str,
    ) -> WebResult<()> {
        let subject = "[APICH] Temporary Password for Your Account".to_string();
        let body = format!(
            "Hello {username},\n\n\
            An administrator has reset your password on APICH Technical & Academic Workspace.\n\n\
            Your Temporary Credentials:\n\
            Username: {username}\n\
            Temporary Password: {temp_password}\n\
            Login URL: {base_url}/login\n\n\
            For account security, please sign in and immediately update your password under Account Settings.\n\n\
            Best regards,\n\
            The APICH Team"
        );
        self.send_mail(settings, to_email, &subject, &body).await
    }

    /// Send account provisioning email with credentials when an admin creates a user
    pub async fn send_account_created_email(
        &self,
        settings: &SystemSettings,
        to_email: &str,
        username: &str,
        display_name: &str,
        initial_password: &str,
        base_url: &str,
    ) -> WebResult<()> {
        let subject = "Your APICH Workspace Account Has Been Created".to_string();
        let body = format!(
            "Hello {display_name},\n\n\
            An administrator has created an account for you on APICH Technical & Academic Workspace.\n\n\
            Account Details:\n\
            Username: {username}\n\
            Temporary Password: {initial_password}\n\
            Login URL: {base_url}/login\n\n\
            Please sign in and set a custom password under Account Settings.\n\n\
            Welcome to the APICH research environment!\n\
            The APICH Team"
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
        self.sent_emails
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(SentEmail {
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

            let from_header = format!("\"{from_name}\" <{from_addr}>");

            let email = Message::builder()
                .from(
                    from_header
                        .parse()
                        .map_err(|e| WebError::Internal(format!("Invalid from address: {e}")))?,
                )
                .to(to
                    .parse()
                    .map_err(|e| WebError::BadRequest(format!("Invalid recipient email: {e}")))?)
                .subject(subject)
                .header(ContentType::TEXT_PLAIN)
                .body(body.to_string())
                .map_err(|e| WebError::Internal(format!("Failed to build email message: {e}")))?;

            let tls_config = Self::build_tls_config(settings, host, port)?;
            let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
                .port(port)
                .tls(tls_config)
                .timeout(Some(std::time::Duration::from_secs(10)));

            if let (Some(user), Some(pwd)) = (&settings.smtp_username, &settings.smtp_password) {
                if !user.is_empty() && !pwd.is_empty() {
                    builder = builder.credentials(Credentials::new(user.clone(), pwd.clone()));
                }
            }

            let mailer = builder.build();
            mailer
                .send(email)
                .await
                .map_err(|e| WebError::Internal(format!("SMTP transport failure: {e}")))?;
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

        let security_label = if settings.smtp_force_tls || settings.smtp_port == Some(465) {
            "Force TLS / SMTPS (Implicit TLS - Port 465)"
        } else if settings.smtp_use_tls {
            "STARTTLS (Explicit TLS Upgrade - Port 587)"
        } else {
            "Disabled / Plaintext (Port 25)"
        };

        let subject = "[APICH] SMTP Delivery Test Verification";
        let body = format!(
            "Hello,\n\n\
            This is an automated test notification confirming that the outgoing SMTP mail server on APICH Technical & Academic Workspace is configured properly.\n\n\
            Configuration Details:\n\
            - Host: {}\n\
            - Port: {}\n\
            - Security: {}\n\
            - Force TLS (SMTPS): {}\n\
            - Sender: {} <{}>\n\
            - Status: Active & Operational\n\n\
            If you received this message, outbound emailing is fully functional!",
            host.unwrap_or("simulated-localhost"),
            settings.smtp_port.unwrap_or(587),
            security_label,
            if settings.smtp_force_tls { "Enabled (Direct Wrapper)" } else { "Disabled" },
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

            let from_header = format!("\"{from_name}\" <{from_addr}>");

            let email = Message::builder()
                .from(
                    from_header
                        .parse()
                        .map_err(|e| WebError::Internal(format!("Invalid from address: {e}")))?,
                )
                .to(recipient
                    .parse()
                    .map_err(|e| WebError::BadRequest(format!("Invalid recipient email: {e}")))?)
                .subject(subject)
                .header(ContentType::TEXT_PLAIN)
                .body(body.clone())
                .map_err(|e| WebError::Internal(format!("Failed to build email message: {e}")))?;

            let tls_config = Self::build_tls_config(settings, host, port)?;
            let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
                .port(port)
                .tls(tls_config);

            if let (Some(ref user), Some(ref pwd)) =
                (&settings.smtp_username, &settings.smtp_password)
            {
                if !user.is_empty() && !pwd.is_empty() {
                    builder = builder.credentials(Credentials::new(user.clone(), pwd.clone()));
                }
            }

            let mailer = builder.build();
            mailer
                .send(email)
                .await
                .map_err(|e| WebError::Internal(format!("SMTP transport failure: {e}")))?;
        }

        // Always record in sent log for test assertions & auditing
        self.sent_emails
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(SentEmail {
                to: recipient.to_string(),
                subject: subject.to_string(),
                body,
            });

        Ok(())
    }

    /// Send a two-factor authentication verification code email.
    pub async fn send_2fa_code_email(
        &self,
        settings: &SystemSettings,
        to_email: &str,
        username: &str,
        code: &str,
    ) -> WebResult<()> {
        let subject = format!("[APICH] Your Verification Code: {code}");
        let body = format!(
            "Hello {username},\n\n\
            Your two-factor authentication verification code is:\n\n\
                {code}\n\n\
            This code will expire in 10 minutes.\n\n\
            If you did not attempt to sign in to your APICH account, please secure your password immediately.\n\n\
            — The APICH Team\n"
        );

        self.send_mail(settings, to_email, &subject, &body).await
    }

    /// Send a document or presentation share notification email.
    pub async fn send_file_share_email(
        &self,
        settings: &SystemSettings,
        to_email: &str,
        sender_name: &str,
        project_name: &str,
        file_name: &str,
        slide_url: Option<&str>,
        pdf_url: Option<&str>,
        custom_note: Option<&str>,
    ) -> WebResult<()> {
        let subject = format!("[APICH] {sender_name} shared \"{file_name}\" with you ({project_name})");
        let note_text = match custom_note {
            Some(n) if !n.trim().is_empty() => format!("\nMessage from {sender_name}:\n\"{n}\"\n"),
            _ => String::new(),
        };
        let mut links = String::new();
        if let Some(s_url) = slide_url {
            links.push_str(&format!("\n• Interactive In-Browser Presentation / Slides:\n  {s_url}\n"));
        }
        if let Some(p_url) = pdf_url {
            links.push_str(&format!("\n• Rendered Document & PDF Viewer:\n  {p_url}\n"));
        }

        let body = format!(
            "Hello,\n\n\
            {sender_name} has shared a document with you from project \"{project_name}\" on APICH Technical & Academic Workspace.\n\
            {note_text}\
            {links}\n\
            You can open the link in any modern web browser to view the document or presentation, and submit comments.\n\n\
            Best regards,\n\
            The APICH Team"
        );

        self.send_mail(settings, to_email, &subject, &body).await
    }
}
