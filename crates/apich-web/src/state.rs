use crate::auth::PasskeyManager;
use crate::mailer::MailerService;
use crate::services::IdentityService;
use crate::services::ProjectManager;
use crate::services::SsoService;
use apich_db::Database;
use apich_sandbox::SandboxManager;
use std::path::PathBuf;
use std::sync::Arc;

/// Shared application state and dependency injection context passed to all handlers.
#[derive(Clone)]
pub struct AppState {
    /// Database client handle.
    pub db: Arc<Database>,
    /// Sandbox container manager.
    pub sandbox_manager: Arc<SandboxManager>,
    /// Project and repository lifecycle service.
    pub project_manager: Arc<ProjectManager>,
    /// User and team identity service.
    pub identity_service: Arc<IdentityService>,
    /// Single sign-on and `OAuth2` service.
    pub sso_service: Arc<SsoService>,
    /// Email dispatch service.
    pub mailer: Arc<MailerService>,
    /// `WebAuthn` passkey manager.
    pub passkey_manager: Arc<PasskeyManager>,
    /// Base URL of the deployed application.
    pub base_url: String,
}

impl AppState {
    /// Creates a new application state with initialized services and managers.
    #[must_use]
    pub fn new(
        db: Arc<Database>,
        sandbox_manager: Arc<SandboxManager>,
        base_storage_dir: PathBuf,
        base_url: String,
        jwt_secret: String,
    ) -> Self {
        let project_manager = Arc::new(ProjectManager::new(
            Arc::clone(&db),
            Arc::clone(&sandbox_manager),
            base_storage_dir,
        ));

        let identity_service = Arc::new(IdentityService::new(Arc::clone(&db)));
        let sso_service = Arc::new(SsoService::new(
            Arc::clone(&db),
            base_url.clone(),
            jwt_secret,
        ));
        let mailer = Arc::new(MailerService::new());
        let passkey_manager = Arc::new(PasskeyManager::new());

        Self {
            db,
            sandbox_manager,
            project_manager,
            identity_service,
            sso_service,
            mailer,
            passkey_manager,
            base_url,
        }
    }
}
