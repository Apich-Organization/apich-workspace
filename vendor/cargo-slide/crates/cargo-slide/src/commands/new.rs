use std::path::Path;

/// Create and initialize a new presentation project directory
pub fn execute(
    name: &str,
    include_rust: bool,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let project_dir = Path::new(name);
    if project_dir.exists() {
        return Err(format!("Directory already exists: {name}").into());
    }
    super::init::execute(project_dir, include_rust)
}
