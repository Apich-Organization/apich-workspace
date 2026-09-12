//! Self-hosted Git HTTP server.
//!
//! Lets an external `git clone`/`git push` talk to a project directly via the
//! `git http-backend` CGI program rather than reimplementing smart HTTP.
//!
//! Design note on why there's a *separate bare mirror repo*, not the project's own working `.git`:
//! `git-http-backend` accepting a push (`git-receive-pack`) directly into a normal, checked-out
//! repository is a well-known footgun -- unless the repo is configured for it, an incoming push
//! updates the ref but leaves the working tree and index stale/inconsistent with HEAD. So pushes
//! from outside always land in `.apich/git-server.git` (a real bare repo, never checked out by
//! git itself), and `sync_working_from_mirror_and_snapshot` is what deliberately, explicitly
//! brings that into the actual project working tree afterwards -- under this code's control, not
//! git's own ref-update hooks.

use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

fn bare_mirror_path(project_root: &Path) -> PathBuf {
    project_root.join(".apich").join("git-server.git")
}

/// Create the bare mirror repo if it doesn't exist yet, seeding it from the project's own
/// working `.git` (if any) so a first clone isn't empty.
pub async fn ensure_git_server_mirror(project_root: &Path) -> io::Result<PathBuf> {
    let apich_dir = project_root.join(".apich");
    let bare_path = bare_mirror_path(project_root);
    if bare_path.exists() {
        return Ok(bare_path);
    }
    tokio::fs::create_dir_all(&apich_dir).await?;
    let init_out = Command::new("git")
        .arg("init")
        .arg("--bare")
        .arg(&bare_path)
        .output()
        .await?;
    if !init_out.status.success() {
        return Err(io::Error::other(format!(
            "git init --bare failed: {}",
            String::from_utf8_lossy(&init_out.stderr)
        )));
    }
    if project_root.join(".git").exists() {
        let _ = Command::new("git")
            .arg("-C")
            .arg(project_root)
            .arg("push")
            .arg(&bare_path)
            .arg("--all")
            .output()
            .await;
        let _ = point_bare_head_at_working_branch(project_root, &bare_path).await;
    }
    Ok(bare_path)
}

/// `git init --bare` gives the mirror its own default HEAD (historically `master`, or whatever
/// `init.defaultBranch` is configured to), which has no reason to match the working repo's actual
/// branch name (this project's is `main`). Without fixing HEAD, `git clone` of the mirror resolves
/// to a ref that was never pushed and checks out nothing -- a real, live-verified failure mode,
/// not a hypothetical one. Point the mirror's HEAD at whichever branch the working repo is
/// actually on so a clone lands on real content.
async fn point_bare_head_at_working_branch(
    project_root: &Path,
    bare_path: &Path,
) -> io::Result<()> {
    let branch_out = Command::new("git")
        .arg("-C")
        .arg(project_root)
        .arg("branch")
        .arg("--show-current")
        .output()
        .await?;
    let branch = String::from_utf8_lossy(&branch_out.stdout)
        .trim()
        .to_string();
    if branch.is_empty() {
        return Ok(());
    }
    let _ = Command::new("git")
        .arg("--git-dir")
        .arg(bare_path)
        .arg("symbolic-ref")
        .arg("HEAD")
        .arg(format!("refs/heads/{branch}"))
        .output()
        .await?;
    Ok(())
}

/// Push the working repo's branches into the bare mirror so external clones see the latest
/// exported Git history. Called after `git_export_commit` ("Sync to Git"). Force-push is safe
/// here specifically because the working repo is always the authoritative source in this
/// direction -- the bare mirror is never edited directly except by `sync_working_from_mirror...`
/// running in the opposite direction under this module's own control.
pub async fn sync_mirror_from_working(project_root: &Path) -> io::Result<()> {
    let bare_path = ensure_git_server_mirror(project_root).await?;
    let _ = Command::new("git")
        .arg("-C")
        .arg(project_root)
        .arg("push")
        .arg(&bare_path)
        .arg("--all")
        .arg("--force")
        .output()
        .await?;
    point_bare_head_at_working_branch(project_root, &bare_path).await?;
    Ok(())
}

/// After the bare mirror receives an external push, materialize its default branch into the
/// project's real working directory and take an apich-vcs snapshot of the result -- so a pushed
/// change actually becomes part of the project's tracked history, not just inert bare-repo state.
pub async fn sync_working_from_mirror_and_snapshot(project_root: &Path) -> io::Result<()> {
    let bare_path = bare_mirror_path(project_root);
    if !bare_path.exists() {
        return Ok(());
    }
    let head_out = Command::new("git")
        .arg("--git-dir")
        .arg(&bare_path)
        .arg("symbolic-ref")
        .arg("--short")
        .arg("HEAD")
        .output()
        .await?;
    let branch = String::from_utf8_lossy(&head_out.stdout).trim().to_string();
    if branch.is_empty() {
        return Ok(());
    }

    let checkout_out = Command::new("git")
        .arg("--git-dir")
        .arg(&bare_path)
        .arg("--work-tree")
        .arg(project_root)
        .arg("checkout")
        .arg("-f")
        .arg(&branch)
        .output()
        .await?;
    if !checkout_out.status.success() {
        return Err(io::Error::other(format!(
            "checkout after push failed: {}",
            String::from_utf8_lossy(&checkout_out.stderr)
        )));
    }

    if let Ok(vcs) = apich_vcs::ProjectVcs::open_or_init(project_root) {
        let _ = vcs.snapshot_if_changed(format!("Received git push (branch: {branch})"));
    }
    Ok(())
}

pub struct CgiResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Run `git http-backend` as a real CGI process against the bare mirror and return its parsed
/// response. `path_info` is everything after `/git/:project/` in the request URL (e.g.
/// `info/refs`, `git-upload-pack`).
#[allow(clippy::too_many_arguments)]
pub async fn run_git_http_backend(
    project_root: &Path,
    method: &str,
    path_info: &str,
    query_string: &str,
    content_type: Option<&str>,
    content_encoding: Option<&str>,
    remote_user: &str,
    body: &[u8],
) -> io::Result<CgiResponse> {
    let bare_path = ensure_git_server_mirror(project_root).await?;
    let apich_dir = project_root.join(".apich");
    let bare_name = bare_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("git-server.git");

    let mut cmd = Command::new("git");
    cmd.arg("http-backend");
    cmd.env("GIT_PROJECT_ROOT", &apich_dir);
    cmd.env("GIT_HTTP_EXPORT_ALL", "1");
    cmd.env("REQUEST_METHOD", method);
    cmd.env("PATH_INFO", format!("/{bare_name}/{path_info}"));
    cmd.env("QUERY_STRING", query_string);
    cmd.env("REMOTE_USER", remote_user);
    cmd.env("CONTENT_LENGTH", body.len().to_string());
    if let Some(ct) = content_type {
        cmd.env("CONTENT_TYPE", ct);
    }
    if let Some(ce) = content_encoding {
        cmd.env("CONTENT_ENCODING", ce);
    }
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    let Some(mut stdin) = child.stdin.take() else {
        return Err(io::Error::other("stdin was not piped"));
    };
    let body_owned = body.to_vec();
    let write_task = tokio::spawn(async move {
        let _ = stdin.write_all(&body_owned).await;
        // `stdin` drops here, closing the pipe so git-http-backend sees EOF.
    });

    let output = child.wait_with_output().await?;
    let _ = write_task.await;

    if !output.status.success() && output.stdout.is_empty() {
        return Err(io::Error::other(format!(
            "git http-backend exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(parse_cgi_output(&output.stdout))
}

/// Split a raw CGI response into (status, headers, body): a run of `Header: value` lines, a
/// blank line, then the body. `git http-backend` reports its status via a `Status: 200 OK`-style
/// header rather than an HTTP status line (standard CGI convention), defaulting to 200 if absent.
fn parse_cgi_output(raw: &[u8]) -> CgiResponse {
    let separator = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|i| (i, 4))
        .or_else(|| raw.windows(2).position(|w| w == b"\n\n").map(|i| (i, 2)));

    let Some((split_at, sep_len)) = separator else {
        return CgiResponse {
            status: 200,
            headers: Vec::new(),
            body: raw.to_vec(),
        };
    };

    let header_bytes = raw.get(..split_at).unwrap_or(&[]);
    let body_start = split_at.saturating_add(sep_len);
    let body = raw.get(body_start..).map_or_else(Vec::new, <[u8]>::to_vec);
    let header_text = String::from_utf8_lossy(header_bytes);

    let mut status = 200u16;
    let mut headers = Vec::new();
    for line in header_text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if key.eq_ignore_ascii_case("Status") {
            if let Some(code_str) = value.split_whitespace().next() {
                if let Ok(code) = code_str.parse::<u16>() {
                    status = code;
                }
            }
        } else {
            headers.push((key.to_string(), value.to_string()));
        }
    }

    CgiResponse {
        status,
        headers,
        body,
    }
}
