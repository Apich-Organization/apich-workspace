use clap::{Parser, Subcommand};
use std::path::PathBuf;
use uuid::Uuid;

use apich_vcs::{BundleOptions, ProjectVcs};

#[derive(Parser)]
#[command(
    name = "apich",
    about = "APICH VCS - Containerized & Host Unified Version Control CLI",
    version = "0.1.0"
)]
struct Cli {
    /// Target repository path (defaults to current working directory)
    #[arg(short, long, global = true, default_value = ".")]
    path: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new APICH VCS repository
    Init,

    /// Show working tree status compared to HEAD snapshot
    Status,

    /// Create an immutable snapshot (deduplicated via FastCDC)
    #[command(alias = "commit")]
    Snapshot {
        /// Snapshot commit message
        #[arg(short, long)]
        message: String,
    },

    /// List snapshot history
    Log {
        /// Limit number of snapshots shown
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,
    },

    /// Show snapshot details and tracked files
    Show {
        /// Target snapshot UUID
        snapshot_id: Uuid,
    },

    /// Output contents of a file from a snapshot or current working copy
    Cat {
        /// Relative path to the file
        file: String,
        /// Optional snapshot UUID (defaults to current working copy on disk)
        #[arg(short, long)]
        snapshot: Option<Uuid>,
    },

    /// Show file-level differences between snapshots or working tree
    Diff {
        /// Base snapshot UUID (defaults to HEAD snapshot)
        #[arg(long)]
        from: Option<Uuid>,
        /// Target snapshot UUID (defaults to working copy on disk)
        #[arg(long)]
        to: Option<Uuid>,
    },

    /// Branch management
    Branch {
        #[command(subcommand)]
        command: Option<BranchCommands>,
    },

    /// Switch to an existing branch
    Checkout {
        /// Target branch name
        branch: String,
    },

    /// Reconcile and merge another branch (weave-free 3-way merge)
    Merge {
        /// Branch name to merge into current branch
        branch: String,
    },

    /// Revert working copy and branch HEAD to a snapshot
    Revert {
        /// Target snapshot UUID
        snapshot_id: Uuid,
    },

    /// Undo last operation in OpLog
    Undo,

    /// Redo last undone operation in OpLog
    Redo,

    /// Show full reversible operation audit log
    Oplog,

    /// Mark an explicit milestone
    Milestone {
        /// Milestone name (e.g. v1.0, arxiv-draft)
        name: String,
        /// Milestone description
        #[arg(short, long, default_value = "")]
        desc: String,
    },

    /// List all milestones
    Milestones,

    /// Run GFS retention garbage collection and sweep unreferenced chunks
    Gc,

    /// Bundle management for offline work and clean exports
    Bundle {
        #[command(subcommand)]
        command: BundleCommands,
    },

    /// Git compatibility bridge operations
    Git {
        #[command(subcommand)]
        command: GitCommands,
    },

    /// Research materials management (external Git repositories cloned into subfolders)
    Material {
        #[command(subcommand)]
        command: MaterialCommands,
    },

    /// Configuration inspection and programmatic customization
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Subcommand)]
enum BranchCommands {
    /// List all branches (default if no subcommand given)
    List,
    /// Create a new branch
    Create { name: String },
    /// Switch to branch
    Switch { name: String },
}

#[derive(Subcommand)]
enum BundleCommands {
    /// Export repository bundle to a file
    Export {
        /// Destination bundle file path (.apich.bundle)
        dest: PathBuf,
        /// Include full snapshot history (defaults to true)
        #[arg(long, default_value = "true")]
        include_history: bool,
    },
    /// Import repository bundle into a directory
    Import {
        /// Bundle file path (.apich.bundle)
        bundle: PathBuf,
        /// Target directory
        target: PathBuf,
    },
    /// Export a clean snapshot archive (without .apich) for IEEE/arXiv submission or zip download
    ExportArchive {
        /// Snapshot UUID
        snapshot_id: Uuid,
        /// Destination archive path (e.g. submission.tar.gz)
        dest: PathBuf,
    },
}

#[derive(Subcommand)]
enum GitCommands {
    /// Initialize git compatibility bridge in this repository
    Init,
    /// Export current or specified state into a standard Git commit
    Export {
        /// Git branch name
        #[arg(short, long, default_value = "main")]
        branch: String,
        /// Commit message
        #[arg(short, long)]
        message: String,
        /// Author name
        #[arg(long, default_value = "APICH User")]
        author_name: String,
        /// Author email
        #[arg(long, default_value = "user@apich.local")]
        author_email: String,
    },
    /// Add a git remote
    RemoteAdd {
        /// Remote name (e.g. origin)
        name: String,
        /// Remote URL
        url: String,
    },
    /// List all configured git remotes
    Remotes,
    /// Push exported git commits to remote repository
    Push {
        /// Remote name (e.g. origin)
        remote: String,
        /// Branch name (e.g. main)
        branch: String,
    },
    /// Pull changes from remote repository
    Pull {
        /// Remote name (e.g. origin)
        remote: String,
        /// Branch name (e.g. main)
        branch: String,
    },
}

#[derive(Subcommand)]
enum MaterialCommands {
    /// Clone an external Git repository into a subfolder as research material
    Clone {
        /// Git clone URL
        url: String,
        /// Relative destination path inside workspace (e.g. materials/repo)
        path: String,
    },
    /// List all registered research materials
    List,
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Show current configuration
    Show,
    /// Add an ignore pattern (supports globs and !negation)
    IgnoreAdd { pattern: String },
    /// Remove an ignore pattern
    IgnoreRemove { pattern: String },
    /// Add an LFS pattern (supports globs like models/*.onnx)
    LfsAdd { pattern: String },
    /// Set LFS size threshold in bytes
    LfsThreshold { bytes: u64 },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let repo_path = &cli.path;

    match cli.command {
        Commands::Init => {
            ProjectVcs::open_or_init(repo_path)?;
            let canonical = repo_path.canonicalize().unwrap_or_else(|_| repo_path.clone());
            println!("Initialized empty APICH VCS repository in {}", canonical.display());
        }
        Commands::Status => {
            let vcs = ProjectVcs::open_or_init(repo_path)?;
            let status = vcs.status()?;
            println!("On branch {}", status.branch);
            if let Some(ref head) = status.head_snapshot {
                println!("HEAD snapshot: {} \"{}\"", head.id, head.message);
            } else {
                println!("No snapshots yet (empty history)");
            }

            if status.is_clean() {
                println!("Working tree clean (no modifications)");
            } else {
                println!("\nChanges to be committed (FastCDC continuous tracking):");
                for f in &status.added {
                    println!("  new file:   {}", f);
                }
                for f in &status.modified {
                    println!("  modified:   {}", f);
                }
                for f in &status.removed {
                    println!("  deleted:    {}", f);
                }
            }
            println!("\nTotal tracked files: {}", status.total_files);
        }
        Commands::Snapshot { message } => {
            let vcs = ProjectVcs::open_or_init(repo_path)?;
            if !vcs.has_changes()? {
                println!("Nothing to snapshot: working tree clean (no changes detected).");
                return Ok(());
            }
            let snap = vcs.snapshot(&message)?;
            let branch = vcs.current_branch()?.unwrap_or_else(|| "main".to_string());
            println!("[{} {}] {}", branch, snap.id, snap.message);
        }
        Commands::Log { limit } => {
            let vcs = ProjectVcs::open_or_init(repo_path)?;
            let mut snaps = vcs.list_snapshots()?;
            snaps.sort_by_key(|a| std::cmp::Reverse(a.created_at));
            if snaps.is_empty() {
                println!("No snapshots in repository.");
            } else {
                for s in snaps.into_iter().take(limit) {
                    let milestone_tag = if s.is_milestone {
                        format!(" [Milestone: {}]", s.milestone_name.as_deref().unwrap_or("unnamed"))
                    } else {
                        String::new()
                    };
                    println!("snapshot {}{}", s.id, milestone_tag);
                    println!("Date:    {}", s.created_at.to_rfc3339());
                    println!("Tree:    {}", s.tree_hash);
                    println!("Author:  {}", s.author);
                    println!("\n    {}\n", s.message);
                }
            }
        }
        Commands::Show { snapshot_id } => {
            let vcs = ProjectVcs::open_or_init(repo_path)?;
            let snap = vcs.get_snapshot(snapshot_id)?;
            println!("Snapshot:    {}", snap.id);
            println!("Change-ID:   {}", snap.change_id);
            println!("Date:        {}", snap.created_at.to_rfc3339());
            println!("Author:      {}", snap.author);
            println!("Tree-Hash:   {}", snap.tree_hash);
            if snap.is_milestone {
                println!("Milestone:   {}", snap.milestone_name.as_deref().unwrap_or("unnamed"));
            }
            if let Some(ref oid) = snap.git_commit_oid {
                println!("Git-Commit:  {}", oid);
            }
            println!("\n    {}\n", snap.message);

            let tree = vcs.get_tree(&snap.tree_hash)?;
            println!("Tracked files ({} total):", tree.entries.len());
            for (path, entry) in &tree.entries {
                println!("  {:>8} bytes  {}", entry.size, path);
            }
        }
        Commands::Cat { file, snapshot } => {
            let vcs = ProjectVcs::open_or_init(repo_path)?;
            let content = vcs.read_file_content(snapshot, &file)?;
            use std::io::Write;
            std::io::stdout().write_all(&content)?;
        }
        Commands::Diff { from, to } => {
            let vcs = ProjectVcs::open_or_init(repo_path)?;
            let (added, modified, removed) = match (from, to) {
                (Some(from_id), Some(to_id)) => vcs.diff_snapshots(from_id, to_id)?,
                _ => vcs.diff_working()?,
            };

            if added.is_empty() && modified.is_empty() && removed.is_empty() {
                println!("No differences found.");
            } else {
                for f in added {
                    println!("+ {}", f);
                }
                for f in modified {
                    println!("M {}", f);
                }
                for f in removed {
                    println!("- {}", f);
                }
            }
        }
        Commands::Branch { command } => {
            let vcs = ProjectVcs::open_or_init(repo_path)?;
            match command.unwrap_or(BranchCommands::List) {
                BranchCommands::List => {
                    let current = vcs.current_branch()?.unwrap_or_else(|| "main".to_string());
                    let branches = vcs.list_branches()?;
                    for b in branches {
                        if b == current {
                            println!("* {}", b);
                        } else {
                            println!("  {}", b);
                        }
                    }
                }
                BranchCommands::Create { name } => {
                    vcs.branch_create(&name)?;
                    println!("Created branch '{}'", name);
                }
                BranchCommands::Switch { name } => {
                    vcs.branch_switch(&name)?;
                    println!("Switched to branch '{}'", name);
                }
            }
        }
        Commands::Checkout { branch } => {
            let vcs = ProjectVcs::open_or_init(repo_path)?;
            vcs.branch_switch(&branch)?;
            println!("Switched to branch '{}'", branch);
        }
        Commands::Merge { branch } => {
            let vcs = ProjectVcs::open_or_init(repo_path)?;
            let result = vcs.merge(&branch)?;
            if !result.conflicts.is_empty() {
                eprintln!("Merge completed with {} conflicts:", result.conflicts.len());
                for c in &result.conflicts {
                    eprintln!("  CONFLICT in {}", c.path);
                }
                println!("Inline conflict markers were placed in the affected files.");
            } else {
                println!("Merged branch '{}' cleanly (weave-free).", branch);
            }
        }
        Commands::Undo => {
            let vcs = ProjectVcs::open_or_init(repo_path)?;
            if let Some(target_id) = vcs.undo()? {
                println!("Successfully undid last operation; HEAD is now at snapshot {}", target_id);
            } else {
                println!("Nothing to undo (OpLog is empty or at the beginning).");
            }
        }
        Commands::Redo => {
            let vcs = ProjectVcs::open_or_init(repo_path)?;
            if let Some(target_id) = vcs.redo()? {
                println!("Successfully redid operation; HEAD is now at snapshot {}", target_id);
            } else {
                println!("Nothing to redo (no undone operations to restore).");
            }
        }
        Commands::Oplog => {
            let vcs = ProjectVcs::open_or_init(repo_path)?;
            let ops = vcs.oplog_list()?;
            if ops.is_empty() {
                println!("Operation log is empty.");
            } else {
                for (i, op) in ops.iter().enumerate() {
                    let before = op.snapshot_before.map(|u| u.to_string()).unwrap_or_else(|| "none".to_string());
                    let after = op.snapshot_after.map(|u| u.to_string()).unwrap_or_else(|| "none".to_string());
                    println!(
                        "[{:3}] {:?} ({} -> {}): {}",
                        i + 1,
                        op.action,
                        &before[..before.len().min(8)],
                        &after[..after.len().min(8)],
                        op.description
                    );
                }
            }
        }
        Commands::Revert { snapshot_id } => {
            let vcs = ProjectVcs::open_or_init(repo_path)?;
            vcs.revert_to(snapshot_id)?;
            println!("Working copy and HEAD reverted to snapshot {}", snapshot_id);
        }
        Commands::Milestone { name, desc } => {
            let vcs = ProjectVcs::open_or_init(repo_path)?;
            let snap = vcs.create_milestone(&name, &desc)?;
            println!("Created milestone '{}' at snapshot {}", name, snap.id);
        }
        Commands::Milestones => {
            let vcs = ProjectVcs::open_or_init(repo_path)?;
            let ms = vcs.list_milestones()?;
            if ms.is_empty() {
                println!("No milestones tagged.");
            } else {
                for m in ms {
                    println!(
                        "* {} (snapshot {}) - {}",
                        m.milestone_name.as_deref().unwrap_or("unnamed"),
                        m.id,
                        m.message
                    );
                }
            }
        }
        Commands::Gc => {
            let vcs = ProjectVcs::open_or_init(repo_path)?;
            let stats = vcs.run_gc(None)?;
            println!(
                "GC finished: pruned {} unreferenced chunks, freed {} bytes.",
                stats.pruned_chunks, stats.reclaimed_bytes
            );
        }
        Commands::Bundle { command } => match command {
            BundleCommands::Export {
                dest,
                include_history: _,
            } => {
                let vcs = ProjectVcs::open_or_init(repo_path)?;
                let opts = BundleOptions::default();
                vcs.export_bundle_to_file(&dest, opts)?;
                println!("Exported project bundle to {}", dest.display());
            }
            BundleCommands::Import { bundle, target } => {
                ProjectVcs::import_bundle_from_file(&bundle, &target)?;
                println!(
                    "Imported project bundle from {} into {}",
                    bundle.display(),
                    target.display()
                );
            }
            BundleCommands::ExportArchive {
                snapshot_id,
                dest,
            } => {
                let vcs = ProjectVcs::open_or_init(repo_path)?;
                vcs.export_snapshot_archive_to_file(snapshot_id, &dest)?;
                println!(
                    "Exported clean submission archive for snapshot {} to {}",
                    snapshot_id,
                    dest.display()
                );
            }
        },
        Commands::Git { command } => match command {
            GitCommands::Init => {
                let vcs = ProjectVcs::open_or_init(repo_path)?;
                vcs.git_init()?;
                println!("Initialized Git bridge in {}", repo_path.display());
            }
            GitCommands::Export {
                branch,
                message,
                author_name,
                author_email,
            } => {
                let vcs = ProjectVcs::open_or_init(repo_path)?;
                let oid = vcs.git_export_commit(&branch, &message, &author_name, &author_email)?;
                println!("Exported snapshot to Git commit: {} on branch '{}'", oid, branch);
            }
            GitCommands::RemoteAdd { name, url } => {
                let vcs = ProjectVcs::open_or_init(repo_path)?;
                vcs.git_setup_remote(&name, &url)?;
                println!("Added Git remote '{}' -> {}", name, url);
            }
            GitCommands::Remotes => {
                let vcs = ProjectVcs::open_or_init(repo_path)?;
                let remotes = vcs.git_remotes()?;
                if remotes.is_empty() {
                    println!("No Git remotes configured.");
                } else {
                    for (name, url) in remotes {
                        println!("{} -> {}", name, url);
                    }
                }
            }
            GitCommands::Push { remote, branch } => {
                let vcs = ProjectVcs::open_or_init(repo_path)?;
                vcs.git_push(&remote, &branch)?;
                println!("Pushed to remote '{}' branch '{}'", remote, branch);
            }
            GitCommands::Pull { remote, branch } => {
                let vcs = ProjectVcs::open_or_init(repo_path)?;
                vcs.git_pull(&remote, &branch)?;
                println!("Pulled from remote '{}' branch '{}'", remote, branch);
            }
        },
        Commands::Material { command } => match command {
            MaterialCommands::Clone { url, path } => {
                let vcs = ProjectVcs::open_or_init(repo_path)?;
                let record = vcs.clone_material(&url, &path)?;
                println!("Cloned material '{}' at {} (commit {})", record.name, record.rel_path, record.commit_oid);
            }
            MaterialCommands::List => {
                let vcs = ProjectVcs::open_or_init(repo_path)?;
                let materials = vcs.list_materials()?;
                if materials.is_empty() {
                    println!("No research materials registered.");
                } else {
                    for m in materials {
                        println!("* {} ({}) at {} [commit {}]", m.name, m.url, m.rel_path, m.commit_oid);
                    }
                }
            }
        },
        Commands::Config { command } => match command {
            ConfigCommands::Show => {
                let vcs = ProjectVcs::open_or_init(repo_path)?;
                let toml_str = toml::to_string_pretty(vcs.config())
                    .map_err(|e| apich_vcs::VcsError::Internal(e.to_string()))?;
                println!("{}", toml_str);
            }
            ConfigCommands::IgnoreAdd { pattern } => {
                let mut vcs = ProjectVcs::open_or_init(repo_path)?;
                vcs.add_ignore_rule(&pattern)?;
                vcs.save_config()?;
                println!("Added ignore pattern '{}' and updated config", pattern);
            }
            ConfigCommands::IgnoreRemove { pattern } => {
                let mut vcs = ProjectVcs::open_or_init(repo_path)?;
                vcs.remove_ignore_rule(&pattern)?;
                vcs.save_config()?;
                println!("Removed ignore pattern '{}' and updated config", pattern);
            }
            ConfigCommands::LfsAdd { pattern } => {
                let mut vcs = ProjectVcs::open_or_init(repo_path)?;
                vcs.add_lfs_pattern(&pattern);
                vcs.save_config()?;
                println!("Added LFS pattern '{}' and updated config", pattern);
            }
            ConfigCommands::LfsThreshold { bytes } => {
                let mut vcs = ProjectVcs::open_or_init(repo_path)?;
                vcs.set_lfs_size_threshold(bytes);
                vcs.save_config()?;
                println!("Set LFS size threshold to {} bytes and updated config", bytes);
            }
        },
    }

    Ok(())
}
