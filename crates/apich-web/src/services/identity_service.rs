use crate::auth::hash_password;
use crate::auth::verify_password;
use crate::error::WebError;
use crate::error::WebResult;
use apich_db::CreateOrganizationDto;
use apich_db::CreateTeamDto;
use apich_db::Database;
use apich_db::IdentityPermissionResolver;
use apich_db::OrgMemberWithUser;
use apich_db::Organization;
use apich_db::Team;
use apich_db::TeamMemberWithUser;
use apich_db::TeamTreeNode;
use apich_db::UpdateOrganizationDto;
use apich_db::UpdateTeamDto;
use apich_db::UpdateUserProfileDto;
use apich_db::User;

use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct IdentityService {
    db: Arc<Database>,
}

impl IdentityService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    // --- Organization Operations ---

    pub async fn create_organization(
        &self,
        creator_id: Uuid,
        dto: CreateOrganizationDto,
    ) -> WebResult<Organization> {
        let repo = self.db.repository();
        if repo.get_organization_by_slug(&dto.slug).await?.is_some() {
            return Err(WebError::Conflict(format!(
                "Organization slug '{}' already exists",
                dto.slug
            )));
        }

        let org = repo.create_organization(creator_id, dto).await?;
        Ok(org)
    }

    pub async fn get_organization(
        &self,
        caller_id: Uuid,
        org_id: Uuid,
    ) -> WebResult<Organization> {
        let repo = self.db.repository();
        let org = repo
            .get_organization_by_id(org_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Organization not found".to_string()))?;

        // Check view access (org admin, member, or platform admin)
        let can_manage =
            IdentityPermissionResolver::can_manage_org(self.db.pool(), caller_id, org_id).await?;
        if !can_manage {
            // Check if member
            let members = repo.list_org_members(org_id).await?;
            if !members.iter().any(|m| m.user_id == caller_id) {
                return Err(WebError::Forbidden(
                    "Access denied to organization".to_string(),
                ));
            }
        }

        Ok(org)
    }

    pub async fn list_user_organizations(
        &self,
        user_id: Uuid,
    ) -> WebResult<Vec<Organization>> {
        let is_admin =
            IdentityPermissionResolver::is_platform_admin(self.db.pool(), user_id).await?;
        let repo = self.db.repository();
        if is_admin {
            // Platform admin sees all orgs
            let all =
                sqlx::query_as::<_, Organization>("SELECT * FROM organizations ORDER BY name ASC")
                    .fetch_all(self.db.pool())
                    .await?;
            Ok(all)
        } else {
            let orgs = repo.list_organizations_for_user(user_id).await?;
            Ok(orgs)
        }
    }

    pub async fn add_org_member(
        &self,
        caller_id: Uuid,
        org_id: Uuid,
        target_user_id: Uuid,
        role: &str,
    ) -> WebResult<()> {
        let can_manage =
            IdentityPermissionResolver::can_manage_org(self.db.pool(), caller_id, org_id).await?;
        if !can_manage {
            return Err(WebError::Forbidden(
                "You must be an Organization Owner or Platform Admin to manage members".to_string(),
            ));
        }

        let repo = self.db.repository();
        repo.add_org_member(org_id, target_user_id, role).await?;
        Ok(())
    }

    pub async fn remove_org_member(
        &self,
        caller_id: Uuid,
        org_id: Uuid,
        target_user_id: Uuid,
    ) -> WebResult<()> {
        let can_manage =
            IdentityPermissionResolver::can_manage_org(self.db.pool(), caller_id, org_id).await?;
        if !can_manage {
            return Err(WebError::Forbidden(
                "You must be an Organization Owner or Platform Admin to remove members".to_string(),
            ));
        }

        let repo = self.db.repository();
        repo.remove_org_member(org_id, target_user_id).await?;
        Ok(())
    }

    pub async fn list_org_members(
        &self,
        caller_id: Uuid,
        org_id: Uuid,
    ) -> WebResult<Vec<OrgMemberWithUser>> {
        let _ = self.get_organization(caller_id, org_id).await?;
        let repo = self.db.repository();
        let members = repo.list_org_members(org_id).await?;
        Ok(members)
    }

    pub async fn update_organization(
        &self,
        caller_id: Uuid,
        org_id: Uuid,
        dto: UpdateOrganizationDto,
    ) -> WebResult<Organization> {
        let can_manage =
            IdentityPermissionResolver::can_manage_org(self.db.pool(), caller_id, org_id).await?;
        if !can_manage {
            return Err(WebError::Forbidden(
                "You must be an Organization Owner or Platform Admin to edit this organization"
                    .to_string(),
            ));
        }

        let repo = self.db.repository();
        let org = repo.update_organization(org_id, dto).await?;
        Ok(org)
    }

    pub async fn delete_organization(
        &self,
        caller_id: Uuid,
        org_id: Uuid,
    ) -> WebResult<()> {
        let is_platform_admin =
            IdentityPermissionResolver::is_platform_admin(self.db.pool(), caller_id).await?;
        let repo = self.db.repository();
        let members = repo.list_org_members(org_id).await?;
        let is_owner = members
            .iter()
            .any(|m| m.user_id == caller_id && m.role == "owner");

        if !is_platform_admin && !is_owner {
            return Err(WebError::Forbidden(
                "Only the Organization Owner or Platform Admin can delete an organization"
                    .to_string(),
            ));
        }

        repo.delete_organization(org_id).await?;
        Ok(())
    }

    // --- Team Operations ---


    pub async fn create_team(
        &self,
        creator_id: Uuid,
        dto: CreateTeamDto,
    ) -> WebResult<Team> {
        // Can caller create a team in this org?
        // If parent_team_id is specified: caller must be admin of parent team OR org owner
        // If no parent_team_id: caller must be org owner/admin
        let can_manage = if let Some(parent_id) = dto.parent_team_id {
            IdentityPermissionResolver::can_manage_team(self.db.pool(), creator_id, parent_id)
                .await?
        } else {
            IdentityPermissionResolver::can_manage_org(self.db.pool(), creator_id, dto.org_id)
                .await?
        };

        if !can_manage {
            return Err(WebError::Forbidden(
                "Insufficient privileges to create a team at this tree node".to_string(),
            ));
        }

        let repo = self.db.repository();
        let team = repo.create_team(creator_id, dto).await?;
        Ok(team)
    }

    pub async fn get_team_tree_for_org(
        &self,
        caller_id: Uuid,
        org_id: Uuid,
    ) -> WebResult<Vec<TeamTreeNode>> {
        let _ = self.get_organization(caller_id, org_id).await?;
        let repo = self.db.repository();
        let tree = repo.get_team_tree_for_org(org_id).await?;
        Ok(tree)
    }

    pub async fn list_teams_by_org(
        &self,
        caller_id: Uuid,
        org_id: Uuid,
    ) -> WebResult<Vec<Team>> {
        let _ = self.get_organization(caller_id, org_id).await?;
        let repo = self.db.repository();
        let teams = repo.list_teams_by_org(org_id).await?;
        Ok(teams)
    }

    pub async fn add_team_member(
        &self,
        caller_id: Uuid,
        team_id: Uuid,
        target_user_id: Uuid,
        role: &str,
    ) -> WebResult<()> {
        let can_manage =
            IdentityPermissionResolver::can_manage_team(self.db.pool(), caller_id, team_id).await?;
        if !can_manage {
            return Err(WebError::Forbidden(
                "You do not have administrative rights to manage members in this team".to_string(),
            ));
        }

        let repo = self.db.repository();
        repo.add_team_member(team_id, target_user_id, role).await?;
        Ok(())
    }

    pub async fn remove_team_member(
        &self,
        caller_id: Uuid,
        team_id: Uuid,
        target_user_id: Uuid,
    ) -> WebResult<()> {
        let can_manage =
            IdentityPermissionResolver::can_manage_team(self.db.pool(), caller_id, team_id).await?;
        if !can_manage {
            return Err(WebError::Forbidden(
                "You do not have administrative rights to remove members from this team"
                    .to_string(),
            ));
        }

        let repo = self.db.repository();
        repo.remove_team_member(team_id, target_user_id).await?;
        Ok(())
    }

    pub async fn list_team_members(
        &self,
        caller_id: Uuid,
        team_id: Uuid,
    ) -> WebResult<Vec<TeamMemberWithUser>> {
        let repo = self.db.repository();
        let team = repo
            .get_team_by_id(team_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Team not found".to_string()))?;

        let can_view =
            IdentityPermissionResolver::can_manage_team(self.db.pool(), caller_id, team_id).await?
                || repo
                    .list_team_members(team_id)
                    .await?
                    .iter()
                    .any(|m| m.user_id == caller_id);

        if !can_view {
            let _ = self.get_organization(caller_id, team.org_id).await?;
        }

        let members = repo.list_team_members(team_id).await?;
        Ok(members)
    }

    pub async fn update_team(
        &self,
        caller_id: Uuid,
        team_id: Uuid,
        dto: UpdateTeamDto,
    ) -> WebResult<Team> {
        let repo = self.db.repository();
        let team = repo
            .get_team_by_id(team_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Team not found".to_string()))?;

        // Caller must be able to manage this team or its organization
        let can_manage =
            IdentityPermissionResolver::can_manage_team(self.db.pool(), caller_id, team_id).await?
                || IdentityPermissionResolver::can_manage_org(
                    self.db.pool(),
                    caller_id,
                    team.org_id,
                )
                .await?;

        if !can_manage {
            return Err(WebError::Forbidden(
                "You do not have administrative rights to modify this team".to_string(),
            ));
        }

        // Prevent cyclic self-parenting
        if let Some(Some(parent_id)) = dto.parent_team_id {
            if parent_id == team_id {
                return Err(WebError::BadRequest(
                    "A team cannot be its own parent team".to_string(),
                ));
            }
        }

        let updated = repo.update_team(team_id, dto).await?;
        Ok(updated)
    }

    pub async fn delete_team(
        &self,
        caller_id: Uuid,
        team_id: Uuid,
    ) -> WebResult<()> {
        let repo = self.db.repository();
        let team = repo
            .get_team_by_id(team_id)
            .await?
            .ok_or_else(|| WebError::NotFound("Team not found".to_string()))?;

        let can_manage =
            IdentityPermissionResolver::can_manage_team(self.db.pool(), caller_id, team_id).await?
                || IdentityPermissionResolver::can_manage_org(
                    self.db.pool(),
                    caller_id,
                    team.org_id,
                )
                .await?;

        if !can_manage {
            return Err(WebError::Forbidden(
                "You do not have administrative rights to delete this team".to_string(),
            ));
        }

        repo.delete_team(team_id).await?;
        Ok(())
    }

    // --- User Profile & Personal Security Operations ---

    pub async fn update_user_profile(
        &self,
        user_id: Uuid,
        dto: UpdateUserProfileDto,
    ) -> WebResult<User> {
        let repo = self.db.repository();
        let updated = repo.update_user_profile(user_id, dto).await?;
        Ok(updated)
    }

    pub async fn change_user_password(
        &self,
        user_id: Uuid,
        current_password: &str,
        new_password: &str,
    ) -> WebResult<()> {
        if new_password.len() < 8 {
            return Err(WebError::BadRequest(
                "New password must be at least 8 characters long".to_string(),
            ));
        }

        let repo = self.db.repository();
        let user = repo
            .get_user_by_id(user_id)
            .await?
            .ok_or_else(|| WebError::NotFound("User not found".to_string()))?;

        let valid = verify_password(current_password, &user.password_hash)?;
        if !valid {
            return Err(WebError::BadRequest(
                "Current password entered is incorrect".to_string(),
            ));
        }


        let new_hash = hash_password(new_password)?;
        repo.update_user_password(user_id, &new_hash).await?;
        Ok(())
    }

    /// Check if user has administrative roles in any organization or team
    pub async fn is_org_or_team_admin(
        &self,
        user_id: Uuid,
    ) -> WebResult<bool> {
        let result: (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM org_members WHERE user_id = $1 AND role IN ('owner', 'admin')
                UNION ALL
                SELECT 1 FROM team_members WHERE user_id = $1 AND role = 'admin'
            )
            "#,
        )
        .bind(user_id)
        .fetch_one(self.db.pool())
        .await
        .map_err(apich_db::DbError::from)?;

        Ok(result.0)
    }
}
