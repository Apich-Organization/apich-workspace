use crate::app::components::ActiveNav;
use crate::app::components::AppShell;
use crate::ui::i18n::I18n;
use apich_db::IdentityPermissionResolver;
use apich_db::OrgMemberWithUser;
use apich_db::Organization;
use apich_db::Repository;
use apich_db::Team;
use apich_db::TeamMemberWithUser;
use apich_db::TeamTreeNode;
use apich_db::User;
use leptos::prelude::*;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct TeamRow {
    pub team: Team,
    pub depth: usize,
    pub can_manage: bool,
    pub members: Vec<TeamMemberWithUser>,
}

#[derive(Clone)]
pub struct OrgCard {
    pub org: Organization,
    pub can_manage: bool,
    pub members: Vec<OrgMemberWithUser>,
    pub teams: Vec<TeamRow>,
}

/// Depth-first flatten of a team tree, annotating each node with whether the current
/// user is allowed to manage it (plan.md: only an admin at that level may create
/// subteams under it or invite members into it).
pub async fn flatten_team_tree(
    pool: &PgPool,
    user_id: Uuid,
    repo: &Repository<'_>,
    nodes: &[TeamTreeNode],
) -> Vec<TeamRow> {
    let mut out = Vec::new();
    let mut stack: Vec<(&TeamTreeNode, usize)> = nodes.iter().map(|n| (n, 0)).rev().collect();
    while let Some((node, depth)) = stack.pop() {
        let can_manage = IdentityPermissionResolver::can_manage_team(pool, user_id, node.team.id)
            .await
            .unwrap_or(false);
        let members = repo
            .list_team_members(node.team.id)
            .await
            .unwrap_or_default();
        out.push(TeamRow {
            team: node.team.clone(),
            depth,
            can_manage,
            members,
        });
        for child in node.children.iter().rev() {
            stack.push((child, depth.saturating_add(1)));
        }
    }
    out
}

#[component]
pub fn OrgTeamsPage(
    user: User,
    is_org_or_team_admin: bool,
    orgs: Vec<OrgCard>,
    all_users: Vec<User>,
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

    let user_options = all_users
        .into_iter()
        .map(|u| {
            let label = format!("{} (@{})", u.display_name, u.username);
            view! { <option value=u.username>{label}</option> }
        })
        .collect::<Vec<_>>();

    let org_sections = orgs
        .into_iter()
        .map(|card| render_org_card(card, &user_options, i18n))
        .collect::<Vec<_>>();

    view! {
        <AppShell
            user=user
            is_org_or_team_admin=is_org_or_team_admin
            active_nav=ActiveNav::OrgAdmin
            current_path=current_path
            page_title=i18n.sidebar_org_admin().to_string()
            i18n=i18n
        >
            <div class="page-header">
                <div>
                    <h1 class="page-title">{i18n.org_admin_title()}</h1>
                </div>
                <div class="header-actions">
                    <apich_islands::ModalIsland trigger_label=format!("+ {}", i18n.new_organization()) trigger_class="btn btn-primary".to_string() title=i18n.new_organization().to_string()>
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
                </div>
            </div>
            {alert}
            {org_sections}
        </AppShell>
    }
}

fn render_org_card(
    card: OrgCard,
    user_options: &[impl IntoView + Clone + 'static],
    i18n: I18n,
) -> impl IntoView {
    let org = card.org;
    let org_id = org.id;

    let member_rows = card
        .members
        .iter()
        .map(|m| {
            let role_badge = if m.role == "admin" || m.role == "owner" {
                view! { <span class="role-badge role-badge-admin">"Admin"</span> }.into_any()
            } else {
                view! { <span class="role-badge role-badge-viewer">"Member"</span> }.into_any()
            };
            let remove = card.can_manage.then(|| {
                view! {
                    <form method="post" action="/admin/orgs/members/remove" class="inline-form">
                        <input type="hidden" name="org_id" value=org_id.to_string() />
                        <input type="hidden" name="user_id" value=m.user_id.to_string() />
                        <apich_islands::ConfirmSubmitButton
                            label=i18n.remove().to_string()
                            message="Remove member?".to_string()
                            button_class="btn btn-danger btn-sm".to_string()
                            button_style=String::new()
                        />
                    </form>
                }
            });
            view! {
                <div class="collaborator-item" style="justify-content:space-between; margin-bottom:0.5rem;">
                    <div style="display:flex; align-items:center; gap:0.65rem;">
                        <div class="collab-avatar" style="width:28px; height:28px; font-size:0.7rem;">{m.display_name.chars().take(2).collect::<String>()}</div>
                        <div>
                            <span style="font-weight:600; font-size:0.85rem;">{m.display_name.clone()}</span>
                            <span style="font-size:0.75rem; color:var(--text-sub);">{format!(" (@{})", m.username)}</span>
                        </div>
                    </div>
                    <div style="display:flex; align-items:center; gap:0.5rem;">{role_badge}{remove}</div>
                </div>
            }
        })
        .collect::<Vec<_>>();

    let team_rows = card
        .teams
        .iter()
        .map(|row| render_team_row(row, &org_id, user_options, i18n))
        .collect::<Vec<_>>();

    let manage_controls = card.can_manage.then(|| {
        let modal_team_id = format!("modal-team-{org_id}");
        let modal_member_id = format!("modal-org-member-{org_id}");
        view! {
            <button type="button" class="btn btn-secondary btn-sm" onclick=format!("document.getElementById('{}').style.display='flex'", modal_team_id)>
                "+ " {i18n.create_team()}
            </button>
            <button type="button" class="btn btn-secondary btn-sm" onclick=format!("document.getElementById('{}').style.display='flex'", modal_member_id)>
                "+ " {i18n.add_member()}
            </button>
            <form method="post" action="/admin/orgs/delete" class="inline-form">
                <input type="hidden" name="org_id" value=org_id.to_string() />
                <apich_islands::ConfirmSubmitButton
                    label=i18n.delete().to_string()
                    message="Delete this organization and everything under it?".to_string()
                    button_class="btn btn-danger btn-sm".to_string()
                    button_style=String::new()
                />
            </form>
        }
    });

    let create_team_modal = card.can_manage.then(|| {
        view! {
            <div id=format!("modal-team-{}", org_id) class="modal-backdrop">
                <div class="modal-card">
                    <div class="modal-header">
                        <h3 class="modal-title">{i18n.create_team()}</h3>
                        <button type="button" class="modal-close" onclick=format!("document.getElementById('modal-team-{}').style.display='none'", org_id)>"×"</button>
                    </div>
                    <form method="post" action="/admin/teams/new">
                        <input type="hidden" name="org_id" value=org_id.to_string() />
                        <div class="form-group">
                            <label>{i18n.team_name()}</label>
                            <input type="text" name="name" required=true class="form-control" />
                        </div>
                        <div class="form-group">
                            <label>{i18n.team_slug()}</label>
                            <input type="text" name="slug" required=true class="form-control" />
                        </div>
                        <div class="form-group">
                            <label>{i18n.description_label()}</label>
                            <textarea name="description" rows="2" class="form-control"></textarea>
                        </div>
                        <div style="display:flex; justify-content:flex-end; gap:0.5rem; margin-top:1.25rem;">
                            <button type="submit" class="btn btn-primary">{i18n.save()}</button>
                        </div>
                    </form>
                </div>
            </div>
        }
    });

    let add_member_modal = card.can_manage.then(|| {
        view! {
            <div id=format!("modal-org-member-{}", org_id) class="modal-backdrop">
                <div class="modal-card">
                    <div class="modal-header">
                        <h3 class="modal-title">{i18n.add_member()}</h3>
                        <button type="button" class="modal-close" onclick=format!("document.getElementById('modal-org-member-{}').style.display='none'", org_id)>"×"</button>
                    </div>
                    <form method="post" action="/admin/orgs/members/add">
                        <input type="hidden" name="org_id" value=org_id.to_string() />
                        <div class="form-group">
                            <label>"User"</label>
                            <select name="user_id_or_username" required=true class="form-control">{user_options.to_vec()}</select>
                        </div>
                        <div class="form-group">
                            <label>"Role"</label>
                            <select name="role" class="form-control">
                                <option value="member">"Member"</option>
                                <option value="admin">"Admin"</option>
                            </select>
                        </div>
                        <div style="display:flex; justify-content:flex-end; gap:0.5rem; margin-top:1.25rem;">
                            <button type="submit" class="btn btn-primary">{i18n.save()}</button>
                        </div>
                    </form>
                </div>
            </div>
        }
    });

    view! {
        <div class="section-card" style="margin-bottom:2rem;">
            <div style="display:flex; justify-content:space-between; align-items:center; border-bottom:1px solid var(--border-subtle); padding-bottom:1rem; margin-bottom:1.25rem;">
                <div>
                    <h3 style="font-size:1.3rem; font-weight:700; color:var(--text-main);">{org.name.clone()}</h3>
                    <span style="font-size:0.8rem; color:var(--text-sub);">"Slug: "<code>{org.slug}</code></span>
                </div>
                <div style="display:flex; gap:0.5rem; align-items:center;">{manage_controls}</div>
            </div>

            <div style="margin-bottom:1.5rem;">
                <h4 style="font-size:1rem; font-weight:600; margin-bottom:0.75rem; color:var(--text-main);">{i18n.teams_and_groups()}</h4>
                {if team_rows.is_empty() {
                    view! { <p class="text-muted" style="font-size:0.85rem;">{i18n.no_subteams()}</p> }.into_any()
                } else {
                    view! { <div>{team_rows}</div> }.into_any()
                }}
            </div>

            <div>
                <h4 style="font-size:1rem; font-weight:600; margin-bottom:0.75rem; color:var(--text-main);">
                    {format!("{} ({})", i18n.org_members_heading(), member_rows.len())}
                </h4>
                <div style="max-height:260px; overflow-y:auto;">{member_rows}</div>
            </div>
        </div>
        {create_team_modal}
        {add_member_modal}
    }
}

fn render_team_row(
    row: &TeamRow,
    org_id: &Uuid,
    user_options: &[impl IntoView + Clone + 'static],
    i18n: I18n,
) -> impl IntoView {
    let team_id = row.team.id;
    let depth = u32::try_from(row.depth).unwrap_or(0);
    let indent = format!("{}rem", 1.25 * f64::from(depth));
    let modal_sub_id = format!("modal-subteam-{team_id}");
    let modal_member_id = format!("modal-team-member-{team_id}");

    let member_chips = row
        .members
        .iter()
        .map(|m| {
            let remove = row.can_manage.then(|| {
                view! {
                    <form method="post" action="/admin/teams/members/remove" class="inline-form">
                        <input type="hidden" name="team_id" value=team_id.to_string() />
                        <input type="hidden" name="user_id" value=m.user_id.to_string() />
                        <apich_islands::ConfirmSubmitButton
                            label="×".to_string()
                            message="Remove member?".to_string()
                            button_class="btn btn-ghost btn-sm".to_string()
                            button_style="padding:0.1rem 0.4rem;".to_string()
                        />
                    </form>
                }
            });
            view! {
                <span class="team-tag" style="display:inline-flex; align-items:center; gap:0.25rem;">
                    {m.display_name.clone()}{remove}
                </span>
            }
        })
        .collect::<Vec<_>>();

    let manage_controls = row.can_manage.then(|| {
        view! {
            <button type="button" class="btn btn-ghost btn-sm" onclick=format!("document.getElementById('{}').style.display='flex'", modal_sub_id)>
                "+ " {i18n.create_team()}
            </button>
            <button type="button" class="btn btn-ghost btn-sm" onclick=format!("document.getElementById('{}').style.display='flex'", modal_member_id)>
                "+ " {i18n.add_member()}
            </button>
            <form method="post" action="/admin/teams/delete" class="inline-form">
                <input type="hidden" name="team_id" value=team_id.to_string() />
                <apich_islands::ConfirmSubmitButton
                    label=i18n.delete().to_string()
                    message="Delete this team and its subteams?".to_string()
                    button_class="btn btn-ghost btn-sm".to_string()
                    button_style="color:var(--danger);".to_string()
                />
            </form>
        }
    });

    let sub_modal = row.can_manage.then(|| {
        view! {
            <div id=modal_sub_id.clone() class="modal-backdrop">
                <div class="modal-card">
                    <div class="modal-header">
                        <h3 class="modal-title">{i18n.create_team()}</h3>
                        <button type="button" class="modal-close" onclick=format!("document.getElementById('{}').style.display='none'", modal_sub_id)>"×"</button>
                    </div>
                    <form method="post" action="/admin/teams/new">
                        <input type="hidden" name="org_id" value=org_id.to_string() />
                        <input type="hidden" name="parent_team_id" value=team_id.to_string() />
                        <div class="form-group">
                            <label>{i18n.team_name()}</label>
                            <input type="text" name="name" required=true class="form-control" />
                        </div>
                        <div class="form-group">
                            <label>{i18n.team_slug()}</label>
                            <input type="text" name="slug" required=true class="form-control" />
                        </div>
                        <div style="display:flex; justify-content:flex-end; gap:0.5rem; margin-top:1.25rem;">
                            <button type="submit" class="btn btn-primary">{i18n.save()}</button>
                        </div>
                    </form>
                </div>
            </div>
        }
    });

    let member_modal = row.can_manage.then(|| {
        view! {
            <div id=modal_member_id.clone() class="modal-backdrop">
                <div class="modal-card">
                    <div class="modal-header">
                        <h3 class="modal-title">{i18n.add_member()}</h3>
                        <button type="button" class="modal-close" onclick=format!("document.getElementById('{}').style.display='none'", modal_member_id)>"×"</button>
                    </div>
                    <form method="post" action="/admin/teams/members/add">
                        <input type="hidden" name="team_id" value=team_id.to_string() />
                        <div class="form-group">
                            <label>"User"</label>
                            <select name="user_id_or_username" required=true class="form-control">{user_options.to_vec()}</select>
                        </div>
                        <div class="form-group">
                            <label>"Role"</label>
                            <select name="role" class="form-control">
                                <option value="member">"Member"</option>
                                <option value="admin">"Admin"</option>
                            </select>
                        </div>
                        <div style="display:flex; justify-content:flex-end; gap:0.5rem; margin-top:1.25rem;">
                            <button type="submit" class="btn btn-primary">{i18n.save()}</button>
                        </div>
                    </form>
                </div>
            </div>
        }
    });

    view! {
        <div style=format!("padding:0.6rem 0; padding-left:{}; border-bottom:1px solid var(--border-subtle);", indent)>
            <div style="display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:0.5rem;">
                <div>
                    <span style="font-weight:600; font-size:0.9rem;">{row.team.name.clone()}</span>
                    <span style="font-size:0.75rem; color:var(--text-sub); margin-left:0.5rem;">{format!("{} members", row.members.len())}</span>
                </div>
                <div style="display:flex; gap:0.4rem; align-items:center;">{manage_controls}</div>
            </div>
            <div style="margin-top:0.4rem; display:flex; flex-wrap:wrap; gap:0.35rem;">{member_chips}</div>
        </div>
        {sub_modal}
        {member_modal}
    }
}
