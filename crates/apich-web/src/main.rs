#![recursion_limit = "512"]

pub use apich_web::app::EMBEDDED_CSS;

use apich_db::{
    CreateOrganizationDto, CreateProjectDto, CreateTeamDto, CreateUserDto, Database,
    PostgresConfig, PostgresContainer, UserRole,
};
use apich_sandbox::SandboxManager;
use apich_vcs::api::ProjectVcs;
use apich_web::AppState;
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use tracing::info;

/// Top-level full HTML page shell wrapper
pub fn render_html_page(title: &str, content_html: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - APICH Technical Workspace</title>
    <style>{}</style>
</head>
<body>
    <div id="app">{}</div>
</body>
</html>"#,
        title, EMBEDDED_CSS, content_html
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,apich_web=debug".into()),
        )
        .init();

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()?;

    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| format!("http://localhost:{}", port));
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "apich_development_secret_key_change_in_production".to_string());
    let workspace_dir = PathBuf::from(std::env::var("APICH_STORAGE_DIR").unwrap_or_else(|_| "./scratch/workspace".to_string()));

    tokio::fs::create_dir_all(&workspace_dir).await?;

    info!("Starting APICH Web Server on port {}", port);

    // Default development Postgres configuration
    let pg_config = PostgresConfig::builder("./scratch/pg_data", "./scratch/pg_backup")
        .container_name("apich-postgres-web")
        .host_port(5432)
        .database("apich_workspace")
        .admin_user("postgres")
        .admin_password("postgres")
        .selinux_relabel(true)
        .build();

    let container = PostgresContainer::new(pg_config.clone());
    info!("Ensuring PostgreSQL container 'apich-postgres-web' is running...");
    if let Err(e) = container.ensure_running().await {
        tracing::warn!("Note on starting container via Podman: {}", e);
    } else if let Err(e) = container.wait_ready(Duration::from_secs(30)).await {
        tracing::warn!("Note waiting for Postgres container: {}", e);
    }

    let db = Arc::new(Database::connect_admin(&pg_config, "localhost").await?);
    db.run_migrations().await?;

    // Auto-seed default administrative credentials and sample workspace if database is uninitialized
    let repo = db.repository();
    let existing_users = repo.list_users().await?;
    if existing_users.is_empty() {
        info!("Database initialized without users. Seeding default platform administrator and demo workspace...");
        let admin_user = repo
            .create_user(CreateUserDto {
                username: "admin".to_string(),
                email: "admin@apich.org".to_string(),
                password_hash: apich_web::auth::hash_password("Admin123!")
                    .map_err(std::io::Error::other)?,
                display_name: "APICH Administrator".to_string(),
                role: Some(UserRole::Admin),
                is_platform_admin: Some(true),
                storage_quota_bytes: None,
            })
            .await?;

        let demo_org = repo
            .create_organization(
                admin_user.id,
                CreateOrganizationDto {
                    name: "APICH Research Lab".to_string(),
                    slug: "apich-lab".to_string(),
                    description: Some("Core Research & Development Organization".to_string()),
                },
            )
            .await?;

        let demo_team = repo
            .create_team(
                admin_user.id,
                CreateTeamDto {
                    org_id: demo_org.id,
                    parent_team_id: None,
                    name: "Systems Engineering".to_string(),
                    slug: "systems-eng".to_string(),
                    description: Some("Core infrastructure & technical systems".to_string()),
                },
            )
            .await?;

        let demo_proj_dir = workspace_dir.join("apich-lab").join("demo-project");
        tokio::fs::create_dir_all(&demo_proj_dir).await?;
        tokio::fs::write(
            demo_proj_dir.join("README.md"),
            "# Demo Project\n\nWelcome to your APICH workspace! Edit this file and see automatic snapshots.\n",
        )
        .await?;
        let _ = ProjectVcs::open_or_init(&demo_proj_dir);

        let _demo_proj = repo
            .create_project(CreateProjectDto {
                org_id: demo_org.id,
                team_id: Some(demo_team.id),
                owner_id: admin_user.id,
                name: "Demo Project".to_string(),
                slug: "demo-project".to_string(),
                description: Some("Getting started with APICH workspace".to_string()),
                storage_path: demo_proj_dir.to_string_lossy().to_string(),
                settings: None,
            })
            .await?;

        info!("Default bootstrap complete: Admin user 'admin' (pass: Admin123!) created with 'Demo Project'.");
    }

    let sandbox_image = std::env::var("APICH_SANDBOX_IMAGE")
        .unwrap_or_else(|_| "docker.io/library/alpine:latest".to_string());
    let sandbox_manager = Arc::new(SandboxManager::new(&workspace_dir, &sandbox_image));
    let state = AppState::new(
        db,
        sandbox_manager,
        workspace_dir,
        base_url,
        jwt_secret,
    );

    let app = apich_web::create_app(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    let banner = format!(
        "\n========================================================================\n\
         🚀 APICH Web Server started successfully!\n\
         🌐 Dashboard URL:    http://127.0.0.1:{port}/\n\
         🔑 Login URL:        http://127.0.0.1:{port}/login\n\
         👤 Default Admin:    admin  /  Admin123!\n\
         📁 Projects Page:    http://127.0.0.1:{port}/projects/demo-project\n\
         🏢 Orgs Admin:       http://127.0.0.1:{port}/admin/orgs\n\
         ⚙️ Platform Admin:   http://127.0.0.1:{port}/admin/platform\n\
         ========================================================================"
    );
    println!("{}", banner);
    info!("Server listening on http://{}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}
