use crate::app::components::ActiveNav;
use crate::app::components::AppShell;
use crate::ui::i18n::I18n;
use apich_db::Organization;
use apich_db::TeamWithOrg;
use apich_db::User;
use apich_db::UserRole;
use apich_db::UserWithStorageSummary;
use leptos::prelude::*;

/// Format byte counts into human-readable strings (MB, GB, etc.).
fn format_storage_bytes(bytes: i64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;

    let b = bytes as f64;
    if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

fn render_org_options(orgs: &[Organization]) -> Vec<impl IntoView> {
    if orgs.is_empty() {
        return vec![view! { <option value="" disabled=true selected=true>"No organizations available"</option> }.into_any()];
    }
    orgs.iter()
        .map(|org| {
            view! { <option value=org.id.to_string()>{format!("{} (@{})", org.name, org.slug)}</option> }.into_any()
        })
        .collect()
}

fn render_team_options(teams: &[TeamWithOrg]) -> Vec<impl IntoView> {
    if teams.is_empty() {
        return vec![view! { <option value="" disabled=true selected=true>"No teams available"</option> }.into_any()];
    }
    teams.iter()
        .map(|t| {
            view! {
                <option value=t.id.to_string()>
                    {format!("{} • {} (@{})", t.org_name, t.name, t.slug)}
                </option>
            }.into_any()
        })
        .collect()
}

#[component]
pub fn AdminUsersPage(
    user: User,
    is_org_or_team_admin: bool,
    users: Vec<UserWithStorageSummary>,
    all_orgs: Vec<Organization>,
    all_teams: Vec<TeamWithOrg>,
    notice: Option<String>,
    error: Option<String>,
    i18n: I18n,
    current_path: String,
) -> impl IntoView {
    let alert = notice.map_or_else(
        || {
            error.map(|e| {
                view! { <div class="alert alert-danger" style="margin-bottom:1.5rem;">{e}</div> }
                    .into_any()
            })
        },
        |n| {
            Some(
                view! { <div class="alert alert-success" style="margin-bottom:1.5rem;">{n}</div> }
                    .into_any(),
            )
        },
    );

    let current_admin_id = user.id;
    let all_orgs_arc = std::sync::Arc::new(all_orgs.clone());
    let all_teams_arc = std::sync::Arc::new(all_teams.clone());

    let user_count = users.len();

    let user_rows = users
        .into_iter()
        .map(|item| {
            let u = item.user;
            let orgs_for_row = all_orgs_arc.clone();
            let teams_for_row = all_teams_arc.clone();
            let used = item.used_storage_bytes;
            let quota = u.storage_quota_bytes;
            let is_self = u.id == current_admin_id;

            let pct = if quota > 0 {
                ((used as f64 / quota as f64) * 100.0).clamp(0.0, 100.0)
            } else {
                0.0
            };

            let bar_color = if pct >= 90.0 {
                "#ef4444" // red / danger
            } else if pct >= 75.0 {
                "#f59e0b" // amber / warning
            } else {
                "var(--primary, #3b82f6)"
            };

            let initial = u
                .display_name
                .chars()
                .next()
                .unwrap_or('U')
                .to_uppercase()
                .to_string();

            let quota_mb = (quota / (1024 * 1024)).max(1);

            let org_tags = if item.orgs.is_empty() {
                view! { <span class="text-muted" style="font-size:0.75rem;">"—"</span> }.into_any()
            } else {
                view! {
                    <div style="display:flex; flex-wrap:wrap; gap:0.25rem;">
                        {item.orgs.iter().map(|o| {
                            view! {
                                <span class="badge" style="font-size:0.75rem; background:var(--bg-muted); border:1px solid var(--border-subtle); padding:0.15rem 0.4rem; border-radius:4px;">
                                    {format!("🏛️ {} ({})", o.org_name, o.role)}
                                </span>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            };

            let team_tags = if item.teams.is_empty() {
                view! { <span class="text-muted" style="font-size:0.75rem;">"—"</span> }.into_any()
            } else {
                view! {
                    <div style="display:flex; flex-wrap:wrap; gap:0.25rem; margin-top:0.25rem;">
                        {item.teams.iter().map(|t| {
                            view! {
                                <span class="badge" style="font-size:0.75rem; background:rgba(59, 130, 246, 0.08); color:var(--primary, #3b82f6); border:1px solid rgba(59, 130, 246, 0.2); padding:0.15rem 0.4rem; border-radius:4px;">
                                    {format!("👥 {} ({})", t.team_name, t.role)}
                                </span>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            };

            let current_org_affiliations = item.orgs.clone();
            let current_team_affiliations = item.teams.clone();

            view! {
                <tr style="border-bottom:1px solid var(--border-subtle);">
                    <td style="padding:0.85rem 1rem; vertical-align:middle;">
                        <div style="display:flex; align-items:center; gap:0.75rem;">
                            <div style="width:36px; height:36px; border-radius:50%; background:var(--primary, #3b82f6); color:#fff; display:flex; align-items:center; justify-content:center; font-weight:600; font-size:0.95rem; flex-shrink:0;">
                                {initial}
                            </div>
                            <div>
                                <div style="font-weight:600; color:var(--text-main); font-size:0.92rem;">
                                    {u.display_name.clone()}
                                    {if is_self {
                                        view! { <span class="badge" style="margin-left:0.4rem; font-size:0.7rem; background:rgba(16, 185, 129, 0.15); color:#10b981; padding:0.1rem 0.4rem; border-radius:4px;">"You"</span> }.into_any()
                                    } else {
                                        view! { <span></span> }.into_any()
                                    }}
                                </div>
                                <div style="font-size:0.8rem; color:var(--text-muted);">
                                    {format!("@{} • {}", u.username, u.email)}
                                </div>
                            </div>
                        </div>
                    </td>
                    <td style="padding:0.85rem 1rem; vertical-align:middle;">
                        <div style="display:flex; flex-direction:column; gap:0.25rem;">
                            {if u.is_platform_admin {
                                view! { <span class="role-badge role-badge-admin" style="width:fit-content; font-size:0.72rem;">"Platform Admin"</span> }.into_any()
                            } else {
                                match u.role {
                                    | UserRole::Admin => view! { <span class="role-badge role-badge-admin" style="width:fit-content; font-size:0.72rem;">"Admin"</span> }.into_any(),
                                    | UserRole::Member => view! { <span class="role-badge role-badge-member" style="width:fit-content; font-size:0.72rem;">"Researcher"</span> }.into_any(),
                                    | UserRole::Guest => view! { <span class="role-badge role-badge-viewer" style="width:fit-content; font-size:0.72rem;">"Guest"</span> }.into_any(),
                                }
                            }}
                        </div>
                    </td>
                    <td style="padding:0.85rem 1rem; vertical-align:middle;">
                        {if u.is_active {
                            view! {
                                <span class="badge" style="background:rgba(16, 185, 129, 0.15); color:#10b981; padding:0.2rem 0.5rem; border-radius:4px; font-size:0.78rem; font-weight:500;">
                                    "● Active"
                                </span>
                            }.into_any()
                        } else {
                            view! {
                                <span class="badge" style="background:rgba(239, 68, 68, 0.15); color:#ef4444; padding:0.2rem 0.5rem; border-radius:4px; font-size:0.78rem; font-weight:500;">
                                    "🔒 Locked"
                                </span>
                            }.into_any()
                        }}
                    </td>
                    <td style="padding:0.85rem 1rem; vertical-align:middle; min-width:170px;">
                        <div style="display:flex; justify-content:space-between; font-size:0.78rem; margin-bottom:0.25rem;">
                            <span style="font-weight:600; color:var(--text-main);">{format_storage_bytes(used)}</span>
                            <span class="text-muted">{format!("limit: {}", format_storage_bytes(quota))}</span>
                        </div>
                        <div style="background:var(--bg-muted); height:6px; border-radius:3px; overflow:hidden; border:1px solid var(--border-subtle);">
                            <div style=format!("width:{pct:.1}%; height:100%; background:{bar_color}; border-radius:3px; transition:width 0.3s ease;")></div>
                        </div>
                        <div style="font-size:0.72rem; color:var(--text-muted); margin-top:0.2rem; text-align:right;">
                            {format!("{pct:.1}% used")}
                        </div>
                    </td>
                    <td style="padding:0.85rem 1rem; vertical-align:middle; max-width:240px;">
                        {org_tags}
                        {team_tags}
                    </td>
                    <td style="padding:0.85rem 1rem; vertical-align:middle; text-align:right; white-space:nowrap;">
                        <div style="display:inline-flex; gap:0.35rem; align-items:center;">
                            // Quota & Role Modal
                            <apich_islands::ModalIsland trigger_label="⚙️ Quota".to_string() trigger_class="btn btn-secondary btn-sm".to_string() title=format!("Storage Quota & Role: @{}", u.username)>
                                <form method="post" action="/admin/users/update-quota" style="text-align:left;">
                                    <input type="hidden" name="user_id" value=u.id.to_string() />
                                    <div class="form-group">
                                        <label>{i18n.user_storage_quota_mb()}</label>
                                        <input type="number" name="storage_quota_mb" value=quota_mb min="1" step="1" required=true class="form-control" />
                                        <small class="text-muted" style="font-size:0.8rem; margin-top:0.25rem; display:block;">
                                            "Default is 100 MB. Common tiers: 100 MB, 500 MB, 1024 MB (1 GB), 10240 MB (10 GB)."
                                        </small>
                                    </div>
                                    <div class="form-group">
                                        <label>"Platform Role"</label>
                                        <select name="role" class="form-control">
                                            <option value="member" selected={u.role == UserRole::Member}>"Researcher (Standard Member)"</option>
                                            <option value="admin" selected={u.role == UserRole::Admin}>"Administrator"</option>
                                            <option value="guest" selected={u.role == UserRole::Guest}>"Guest (Read-only)"</option>
                                        </select>
                                    </div>
                                    <div class="form-group" style="margin-bottom:1.25rem;">
                                        <label style="display:flex; align-items:center; gap:0.5rem; cursor:pointer;">
                                            <input type="checkbox" name="is_platform_admin" value="true" checked=u.is_platform_admin style="width:16px; height:16px;" />
                                            <span>"Global Platform Administrator"</span>
                                        </label>
                                    </div>
                                    <div style="display:flex; justify-content:flex-end; gap:0.5rem; margin-top:1rem;">
                                        <button type="submit" class="btn btn-primary">{i18n.save()}</button>
                                    </div>
                                </form>
                            </apich_islands::ModalIsland>

                            // Affiliations Modal
                            <apich_islands::ModalIsland
                                trigger_label="🏛️ Affiliations".to_string()
                                trigger_class="btn btn-secondary btn-sm".to_string()
                                title=format!("Organizations & Teams: @{}", u.username)
                                card_class="modal-wide".to_string()
                            >
                                <div class="modal-wide" style="text-align:left; display:flex; flex-direction:column; gap:1.25rem;">
                                    <p class="text-muted" style="margin:0; font-size:0.875rem;">
                                        "Manage organization memberships and project team assignments for this user."
                                    </p>

                                    <div style="display:grid; grid-template-columns:repeat(auto-fit, minmax(330px, 1fr)); gap:1.25rem; align-items:start;">
                                        // Organizations Section
                                        <div style="background:var(--bg-muted); border:1px solid var(--border-subtle); border-radius:10px; padding:1.1rem; display:flex; flex-direction:column; gap:1rem;">
                                            <div style="display:flex; justify-content:space-between; align-items:center; border-bottom:1px solid var(--border-subtle); padding-bottom:0.6rem;">
                                                <div style="display:flex; align-items:center; gap:0.5rem;">
                                                    <span style="font-size:1.1rem;">"🏛️"</span>
                                                    <strong style="font-size:0.95rem; color:var(--text-main);">"Organizations"</strong>
                                                </div>
                                                <span class="badge" style="background:var(--bg-surface); border:1px solid var(--border-subtle); font-size:0.75rem; font-weight:600; padding:0.2rem 0.5rem; border-radius:12px;">
                                                    {format!("{} joined", current_org_affiliations.len())}
                                                </span>
                                            </div>

                                            <div style="display:flex; flex-direction:column; gap:0.5rem; max-height:220px; overflow-y:auto; padding-right:0.25rem;">
                                                {if current_org_affiliations.is_empty() {
                                                    view! {
                                                        <div style="padding:1.25rem 0.75rem; text-align:center; background:var(--bg-surface); border:1px dashed var(--border-subtle); border-radius:8px;">
                                                            <p class="text-muted" style="margin:0; font-size:0.85rem;">"No organization affiliations yet."</p>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <div style="display:flex; flex-direction:column; gap:0.4rem;">
                                                            {current_org_affiliations.iter().map(|org_mem| {
                                                                let role_cls = match org_mem.role.as_str() {
                                                                    "owner" => "role-badge role-badge-owner",
                                                                    "admin" => "role-badge role-badge-admin",
                                                                    _ => "role-badge role-badge-member",
                                                                };
                                                                view! {
                                                                    <div style="display:flex; justify-content:space-between; align-items:center; gap:0.75rem; background:var(--bg-surface); padding:0.55rem 0.75rem; border-radius:8px; border:1px solid var(--border-subtle);">
                                                                        <div style="display:flex; align-items:center; gap:0.5rem; min-width:0; overflow:hidden;">
                                                                            <span style="font-weight:600; font-size:0.875rem; color:var(--text-main); white-space:nowrap; overflow:hidden; text-overflow:ellipsis;" title=org_mem.org_name.clone()>
                                                                                {org_mem.org_name.clone()}
                                                                            </span>
                                                                            <span class=role_cls style="font-size:0.7rem; padding:0.15rem 0.45rem; flex-shrink:0;">
                                                                                {org_mem.role.clone()}
                                                                            </span>
                                                                        </div>
                                                                        <form method="post" action="/admin/users/remove-org" style="margin:0; flex-shrink:0;">
                                                                            <input type="hidden" name="user_id" value=u.id.to_string() />
                                                                            <input type="hidden" name="org_id" value=org_mem.org_id.to_string() />
                                                                            <button type="submit" class="btn btn-danger btn-sm" style="padding:0.2rem 0.5rem; font-size:0.75rem;" title="Remove organization membership">
                                                                                "Remove"
                                                                            </button>
                                                                        </form>
                                                                    </div>
                                                                }
                                                            }).collect::<Vec<_>>()}
                                                        </div>
                                                    }.into_any()
                                                }}
                                            </div>

                                            <form method="post" action="/admin/users/assign-org" style="margin:0; background:var(--bg-surface); border:1px solid var(--border-subtle); border-radius:8px; padding:0.9rem;">
                                                <input type="hidden" name="user_id" value=u.id.to_string() />
                                                <div style="font-weight:600; font-size:0.825rem; color:var(--text-sub); margin-bottom:0.75rem; text-transform:uppercase; letter-spacing:0.03em;">
                                                    "+ Add to Organization"
                                                </div>
                                                <div style="margin-bottom:0.75rem;">
                                                    <label style="font-size:0.8rem; font-weight:600; color:var(--text-sub); display:block; margin-bottom:0.35rem;">
                                                        "Select Organization"
                                                    </label>
                                                    <select name="org_id" class="form-control" required=true style="width:100%; box-sizing:border-box;">
                                                        {render_org_options(&orgs_for_row)}
                                                    </select>
                                                </div>
                                                <div style="display:flex; gap:0.6rem; align-items:flex-end;">
                                                    <div style="flex:1; min-width:0;">
                                                        <label style="font-size:0.8rem; font-weight:600; color:var(--text-sub); display:block; margin-bottom:0.35rem;">
                                                            "Assigned Role"
                                                        </label>
                                                        <select name="role" class="form-control" style="width:100%; box-sizing:border-box;">
                                                            <option value="member">"Member"</option>
                                                            <option value="admin">"Admin"</option>
                                                            <option value="owner">"Owner"</option>
                                                        </select>
                                                    </div>
                                                    <button type="submit" class="btn btn-primary" style="height:38px; padding:0 1.1rem; display:inline-flex; align-items:center; gap:0.35rem; flex-shrink:0; white-space:nowrap;">
                                                        "➕ Add"
                                                    </button>
                                                </div>
                                            </form>
                                        </div>

                                        // Teams Section
                                        <div style="background:var(--bg-muted); border:1px solid var(--border-subtle); border-radius:10px; padding:1.1rem; display:flex; flex-direction:column; gap:1rem;">
                                            <div style="display:flex; justify-content:space-between; align-items:center; border-bottom:1px solid var(--border-subtle); padding-bottom:0.6rem;">
                                                <div style="display:flex; align-items:center; gap:0.5rem;">
                                                    <span style="font-size:1.1rem;">"👥"</span>
                                                    <strong style="font-size:0.95rem; color:var(--text-main);">"Teams"</strong>
                                                </div>
                                                <span class="badge" style="background:var(--bg-surface); border:1px solid var(--border-subtle); font-size:0.75rem; font-weight:600; padding:0.2rem 0.5rem; border-radius:12px;">
                                                    {format!("{} joined", current_team_affiliations.len())}
                                                </span>
                                            </div>

                                            <div style="display:flex; flex-direction:column; gap:0.5rem; max-height:220px; overflow-y:auto; padding-right:0.25rem;">
                                                {if current_team_affiliations.is_empty() {
                                                    view! {
                                                        <div style="padding:1.25rem 0.75rem; text-align:center; background:var(--bg-surface); border:1px dashed var(--border-subtle); border-radius:8px;">
                                                            <p class="text-muted" style="margin:0; font-size:0.85rem;">"No team affiliations yet."</p>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <div style="display:flex; flex-direction:column; gap:0.4rem;">
                                                            {current_team_affiliations.iter().map(|team_mem| {
                                                                let role_cls = match team_mem.role.as_str() {
                                                                    "admin" => "role-badge role-badge-admin",
                                                                    _ => "role-badge role-badge-member",
                                                                };
                                                                view! {
                                                                    <div style="display:flex; justify-content:space-between; align-items:center; gap:0.75rem; background:var(--bg-surface); padding:0.55rem 0.75rem; border-radius:8px; border:1px solid var(--border-subtle);">
                                                                        <div style="display:flex; flex-direction:column; gap:0.15rem; min-width:0; overflow:hidden;">
                                                                            <div style="display:flex; align-items:center; gap:0.5rem;">
                                                                                <span style="font-weight:600; font-size:0.875rem; color:var(--text-main); white-space:nowrap; overflow:hidden; text-overflow:ellipsis;" title=team_mem.team_name.clone()>
                                                                                    {team_mem.team_name.clone()}
                                                                                </span>
                                                                                <span class=role_cls style="font-size:0.7rem; padding:0.15rem 0.45rem; flex-shrink:0;">
                                                                                    {team_mem.role.clone()}
                                                                                </span>
                                                                            </div>
                                                                            <span class="text-muted" style="font-size:0.75rem; white-space:nowrap; overflow:hidden; text-overflow:ellipsis;">
                                                                                {format!("Org: {}", team_mem.org_name)}
                                                                            </span>
                                                                        </div>
                                                                        <form method="post" action="/admin/users/remove-team" style="margin:0; flex-shrink:0;">
                                                                            <input type="hidden" name="user_id" value=u.id.to_string() />
                                                                            <input type="hidden" name="team_id" value=team_mem.team_id.to_string() />
                                                                            <button type="submit" class="btn btn-danger btn-sm" style="padding:0.2rem 0.5rem; font-size:0.75rem;" title="Remove team membership">
                                                                                "Remove"
                                                                            </button>
                                                                        </form>
                                                                    </div>
                                                                }
                                                            }).collect::<Vec<_>>()}
                                                        </div>
                                                    }.into_any()
                                                }}
                                            </div>

                                            <form method="post" action="/admin/users/assign-team" style="margin:0; background:var(--bg-surface); border:1px solid var(--border-subtle); border-radius:8px; padding:0.9rem;">
                                                <input type="hidden" name="user_id" value=u.id.to_string() />
                                                <div style="font-weight:600; font-size:0.825rem; color:var(--text-sub); margin-bottom:0.75rem; text-transform:uppercase; letter-spacing:0.03em;">
                                                    "+ Add to Team"
                                                </div>
                                                <div style="margin-bottom:0.75rem;">
                                                    <label style="font-size:0.8rem; font-weight:600; color:var(--text-sub); display:block; margin-bottom:0.35rem;">
                                                        "Select Team"
                                                    </label>
                                                    <select name="team_id" class="form-control" required=true style="width:100%; box-sizing:border-box;">
                                                        {render_team_options(&teams_for_row)}
                                                    </select>
                                                </div>
                                                <div style="display:flex; gap:0.6rem; align-items:flex-end;">
                                                    <div style="flex:1; min-width:0;">
                                                        <label style="font-size:0.8rem; font-weight:600; color:var(--text-sub); display:block; margin-bottom:0.35rem;">
                                                            "Assigned Role"
                                                        </label>
                                                        <select name="role" class="form-control" style="width:100%; box-sizing:border-box;">
                                                            <option value="member">"Member"</option>
                                                            <option value="admin">"Admin"</option>
                                                        </select>
                                                    </div>
                                                    <button type="submit" class="btn btn-primary" style="height:38px; padding:0 1.1rem; display:inline-flex; align-items:center; gap:0.35rem; flex-shrink:0; white-space:nowrap;">
                                                        "➕ Add"
                                                    </button>
                                                </div>
                                            </form>
                                        </div>
                                    </div>
                                </div>
                            </apich_islands::ModalIsland>

                            // Reset Password Form
                            <form method="post" action="/admin/users/reset-password" style="margin:0; display:inline;" onsubmit="return confirm('Generate and dispatch a temporary password to this user? Existing sessions will be terminated.');">
                                <input type="hidden" name="user_id" value=u.id.to_string() />
                                <button type="submit" class="btn btn-secondary btn-sm" title="Generate temporary password and email user">
                                    "🔑 Reset"
                                </button>
                            </form>

                            // Lock / Unlock Toggle Form
                            {if !is_self {
                                let (label, btn_class, confirm_msg) = if u.is_active {
                                    ("🔒 Lock", "btn btn-secondary btn-sm", "Lock this user account? The user will be immediately logged out.")
                                } else {
                                    ("🔓 Unlock", "btn btn-primary btn-sm", "Unlock this user account?")
                                };
                                view! {
                                    <form method="post" action="/admin/users/toggle-lock" style="margin:0; display:inline;" onsubmit=format!("return confirm('{confirm_msg}');")>
                                        <input type="hidden" name="user_id" value=u.id.to_string() />
                                        <button type="submit" class=btn_class>
                                            {label}
                                        </button>
                                    </form>
                                }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }}

                            // Delete Form
                            {if !is_self {
                                view! {
                                    <form method="post" action="/admin/users/delete" style="margin:0; display:inline;" onsubmit="return confirm('PERMANENTLY delete this user account? All owned workspaces, personal access tokens, and sessions will be destroyed. This cannot be undone.');">
                                        <input type="hidden" name="user_id" value=u.id.to_string() />
                                        <button type="submit" class="btn btn-danger btn-sm" title="Permanently delete user">
                                            "🗑️"
                                        </button>
                                    </form>
                                }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }}
                        </div>
                    </td>
                </tr>
            }
        })
        .collect::<Vec<_>>();

    view! {
        <AppShell
            user=user
            is_org_or_team_admin=is_org_or_team_admin
            active_nav=ActiveNav::UserAdmin
            current_path=current_path
            page_title=i18n.sidebar_user_admin().to_string()
            i18n=i18n
        >
            <div class="page-header" style="margin-bottom:1.5rem;">
                <div>
                    <h1 class="page-title">{i18n.user_admin_title()}</h1>
                    <p class="text-muted" style="font-size:0.85rem; margin-top:0.25rem;">
                        {format!("{} • Total Accounts: {}", i18n.user_admin_subtitle(), user_count)}
                    </p>
                </div>
                <div class="header-actions" style="display:flex; gap:0.5rem; align-items:center;">
                    // Create User Modal
                    <apich_islands::ModalIsland trigger_label=format!("+ {}", i18n.new_user()) trigger_class="btn btn-primary".to_string() title=i18n.create_user_title().to_string()>
                        <form method="post" action="/admin/users/new" style="text-align:left;">
                            <div style="display:grid; grid-template-columns: 1fr 1fr; gap:1rem;">
                                <div class="form-group">
                                    <label>"Username"</label>
                                    <input type="text" name="username" required=true placeholder="jane_doe" class="form-control" />
                                </div>
                                <div class="form-group">
                                    <label>"Email Address"</label>
                                    <input type="email" name="email" required=true placeholder="jane@institution.edu" class="form-control" />
                                </div>
                            </div>
                            <div class="form-group">
                                <label>"Display Name"</label>
                                <input type="text" name="display_name" required=true placeholder="Dr. Jane Doe" class="form-control" />
                            </div>
                            <div class="form-group">
                                <label>"Initial Password (Optional)"</label>
                                <input type="password" name="password" placeholder="Leave empty to auto-generate secure temporary password" class="form-control" />
                            </div>
                            <div style="display:grid; grid-template-columns: 1fr 1fr; gap:1rem;">
                                <div class="form-group">
                                    <label>"Platform Role"</label>
                                    <select name="role" class="form-control">
                                        <option value="member" selected=true>"Researcher (Member)"</option>
                                        <option value="admin">"Administrator"</option>
                                        <option value="guest">"Guest"</option>
                                    </select>
                                </div>
                                <div class="form-group">
                                    <label>{i18n.user_storage_quota_mb()}</label>
                                    <input type="number" name="storage_quota_mb" value="100" min="1" step="1" required=true class="form-control" />
                                    <small class="text-muted" style="font-size:0.75rem; margin-top:0.2rem; display:block;">
                                        "Default is 100 MB. Adjustable anytime."
                                    </small>
                                </div>
                            </div>
                            <div class="form-group">
                                <label style="display:flex; align-items:center; gap:0.5rem; cursor:pointer;">
                                    <input type="checkbox" name="is_platform_admin" value="true" style="width:16px; height:16px;" />
                                    <span>"Grant Global Platform Administrator Rights"</span>
                                </label>
                            </div>
                            <div class="form-group" style="margin-bottom:1.25rem;">
                                <label style="display:flex; align-items:center; gap:0.5rem; cursor:pointer;">
                                    <input type="checkbox" name="send_welcome_email" value="true" checked=true style="width:16px; height:16px;" />
                                    <span>"Send welcome email with temporary credentials via SMTP"</span>
                                </label>
                            </div>
                            <div style="display:flex; justify-content:flex-end; gap:0.5rem; margin-top:1.25rem;">
                                <button type="submit" class="btn btn-primary">{i18n.save()}</button>
                            </div>
                        </form>
                    </apich_islands::ModalIsland>

                    // Create Org Modal
                    <apich_islands::ModalIsland trigger_label=format!("+ {}", i18n.new_organization()) trigger_class="btn btn-secondary".to_string() title=i18n.new_organization().to_string()>
                        <form method="post" action="/admin/orgs/new">
                            <apich_islands::NameSlugFieldsIsland
                                name_field="name".to_string()
                                slug_field="slug".to_string()
                                name_label=i18n.org_name().to_string()
                                slug_label=i18n.org_slug().to_string()
                                name_placeholder=String::new()
                                slug_placeholder=String::new()
                            />
                            <div class="form-group">
                                <label>{i18n.description_label()}</label>
                                <textarea name="description" rows="2" class="form-control"></textarea>
                            </div>
                            <div style="display:flex; justify-content:flex-end; gap:0.5rem; margin-top:1.25rem;">
                                <button type="submit" class="btn btn-primary">{i18n.save()}</button>
                            </div>
                        </form>
                    </apich_islands::ModalIsland>

                    // Create Team Modal
                    <apich_islands::ModalIsland trigger_label="+ Team".to_string() trigger_class="btn btn-secondary".to_string() title="Create New Team".to_string()>
                        <form method="post" action="/admin/teams/new">
                            <div class="form-group">
                                <label>"Parent Organization"</label>
                                <select name="org_id" class="form-control" required=true>
                                    {render_org_options(&all_orgs)}
                                </select>
                            </div>
                            <apich_islands::NameSlugFieldsIsland
                                name_field="name".to_string()
                                slug_field="slug".to_string()
                                name_label="Team Name".to_string()
                                slug_label="Team Slug".to_string()
                                name_placeholder=String::new()
                                slug_placeholder=String::new()
                            />
                            <div class="form-group">
                                <label>{i18n.description_label()}</label>
                                <textarea name="description" rows="2" class="form-control"></textarea>
                            </div>
                            <div style="display:flex; justify-content:flex-end; gap:0.5rem; margin-top:1.25rem;">
                                <button type="submit" class="btn btn-primary">{i18n.save()}</button>
                            </div>
                        </form>
                    </apich_islands::ModalIsland>
                </div>
            </div>

            {alert}

            <div class="section-card" style="padding:0; overflow:hidden; border:1px solid var(--border-subtle); border-radius:8px;">
                <div style="overflow-x:auto;">
                    <table style="width:100%; border-collapse:collapse; text-align:left; font-size:0.875rem;">
                        <thead>
                            <tr style="background:var(--bg-muted); border-bottom:1px solid var(--border-subtle); color:var(--text-muted); font-size:0.75rem; text-transform:uppercase; letter-spacing:0.04em;">
                                <th style="padding:0.75rem 1rem;">"User Account"</th>
                                <th style="padding:0.75rem 1rem;">"Role"</th>
                                <th style="padding:0.75rem 1rem;">{i18n.account_status()}</th>
                                <th style="padding:0.75rem 1rem;">"Storage Space (Used / Quota)"</th>
                                <th style="padding:0.75rem 1rem;">"Affiliations"</th>
                                <th style="padding:0.75rem 1rem; text-align:right;">"Actions"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {user_rows}
                        </tbody>
                    </table>
                </div>
            </div>
        </AppShell>
    }
}
