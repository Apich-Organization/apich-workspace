use crate::{
    auth::PasskeyManager,
    mailer::MailerService,
    services::{IdentityService, ProjectManager, SsoService},
};
use apich_db::Database;
use apich_sandbox::SandboxManager;
use std::{path::PathBuf, sync::Arc};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub sandbox_manager: Arc<SandboxManager>,
    pub project_manager: Arc<ProjectManager>,
    pub identity_service: Arc<IdentityService>,
    pub sso_service: Arc<SsoService>,
    pub mailer: Arc<MailerService>,
    pub passkey_manager: Arc<PasskeyManager>,
    pub base_url: String,
}

impl AppState {
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
