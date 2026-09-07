use crate::app::components::Navbar;
use leptos::prelude::*;

#[component]
pub fn OrgTeamsPage() -> impl IntoView {
    view! {
        <div class="org-teams-page">
            <Navbar user_name="Dr. Researcher".to_string() is_admin=true />

            <div class="main-content">
                <div class="page-header">
                    <div>
                        <h1 class="page-title">"Organization & Team Directory"</h1>
                        <p class="page-subtitle">"Hierarchical identity tree: Administrators at any level manage all nested sub-teams and collaborators below their node"</p>
                    </div>
                    <div class="header-actions">
                        <button class="btn btn-secondary" id="btn-invite-member">"Invite Collaborator"</button>
                        <button class="btn btn-primary" id="btn-create-subteam">"Create Sub-Team"</button>
                    </div>
                </div>

                <div class="tree-container-card">
                    <div class="tree-header">
                        <span class="tree-root-badge">"ORGANIZATION ROOT"</span>
                        <h2 class="tree-org-name">"Quantum Research Institute"</h2>
                        <span class="tree-org-slug">"slug: quantum-institute • 14 Members"</span>
                    </div>

                    <div class="tree-body">
                        // Node Level 1
                        <div class="tree-node level-1">
                            <div class="tree-node-content">
                                <div class="node-icon">"📁"</div>
                                <div class="node-info">
                                    <span class="node-title">"Quantum Theory Division"</span>
                                    <span class="node-meta">"Team Lead: Carol Theory • 6 Members"</span>
                                </div>
                                <div class="node-badge admin-badge">"Admin Node"</div>
                            </div>

                            // Node Level 2 (Nested)
                            <div class="tree-children">
                                <div class="tree-node level-2">
                                    <div class="tree-node-content">
                                        <div class="node-icon">"📂"</div>
                                        <div class="node-info">
                                            <span class="node-title">"Fault-Tolerant Architecture"</span>
                                            <span class="node-meta">"Sub-team of Quantum Theory • 4 Members"</span>
                                        </div>
                                    </div>

                                    // Node Level 3 (Nested)
                                    <div class="tree-children">
                                        <div class="tree-node level-3">
                                            <div class="tree-node-content">
                                                <div class="node-icon">"📄"</div>
                                                <div class="node-info">
                                                    <span class="node-title">"Surface Code Protocols"</span>
                                                    <span class="node-meta">"Focus group • 2 Projects"</span>
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>

                        // Node Level 1 (Sibling)
                        <div class="tree-node level-1">
                            <div class="tree-node-content">
                                <div class="node-icon">"📁"</div>
                                <div class="node-info">
                                    <span class="node-title">"Experimental Hardware Lab"</span>
                                    <span class="node-meta">"Cryogenics & Microwave Control • 8 Members"</span>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
