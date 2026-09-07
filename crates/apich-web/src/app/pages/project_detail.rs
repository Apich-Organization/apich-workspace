use crate::app::components::{Navbar, StatusBadge};
use leptos::prelude::*;

#[component]
pub fn ProjectDetailPage() -> impl IntoView {
    view! {
        <div class="project-detail-page">
            <Navbar user_name="Dr. Researcher".to_string() is_admin=true />

            <div class="main-content">
                <div class="breadcrumb">
                    <a href="/">"Projects"</a>
                    <span>" / "</span>
                    <span class="active">"Rotated Surface Code Simulator"</span>
                </div>

                <div class="page-header">
                    <div>
                        <div class="title-with-badge">
                            <h1 class="page-title">"Rotated Surface Code Simulator"</h1>
                            <StatusBadge status="running" is_running=true />
                        </div>
                        <p class="page-subtitle">"Dedicated computational sandbox mounted at /workspace • FastCDC Version Control Active"</p>
                    </div>
                    <div class="header-actions">
                        <form method="post" action="/api/projects/proj-id/sandbox/stop" class="inline-form">
                            <button type="submit" class="btn btn-secondary">"Pause Engine"</button>
                        </form>
                        <button class="btn btn-primary" id="btn-create-snapshot">"Create Snapshot"</button>
                    </div>
                </div>

                <div class="detail-grid">
                    // Left Column: Version History & Timeline
                    <div class="detail-main">
                        <div class="section-card">
                            <h2 class="section-title">"Timeline & Version Snapshots"</h2>
                            <p class="text-muted">"Continuous zero-overhead snapshots with content-defined deduplication"</p>

                            <div class="timeline-list">
                                <div class="timeline-item">
                                    <div class="timeline-dot active"></div>
                                    <div class="timeline-content">
                                        <div class="timeline-header">
                                            <span class="timeline-msg">"Added syndrome extraction decoder with MWPM heuristic"</span>
                                            <span class="timeline-time">"12 minutes ago"</span>
                                        </div>
                                        <div class="timeline-meta">
                                            <span>"Snapshot: #18"</span>
                                            <span>"•"</span>
                                            <span>"Files: 4 modified, 1 added"</span>
                                        </div>
                                    </div>
                                </div>

                                <div class="timeline-item">
                                    <div class="timeline-dot"></div>
                                    <div class="timeline-content">
                                        <div class="timeline-header">
                                            <span class="timeline-msg">"Configured benchmark parameters for distance d=5 code"</span>
                                            <span class="timeline-time">"2 hours ago"</span>
                                        </div>
                                        <div class="timeline-meta">
                                            <span>"Snapshot: #17"</span>
                                            <span>"•"</span>
                                            <span>"Files: 2 modified"</span>
                                        </div>
                                    </div>
                                </div>

                                <div class="timeline-item">
                                    <div class="timeline-dot"></div>
                                    <div class="timeline-content">
                                        <div class="timeline-header">
                                            <span class="timeline-msg">"Initial workspace setup with baseline lattice generator"</span>
                                            <span class="timeline-time">"Yesterday"</span>
                                        </div>
                                        <div class="timeline-meta">
                                            <span>"Snapshot: #16"</span>
                                            <span>"•"</span>
                                            <span>"Files: 8 added"</span>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>

                    // Right Column: Sandbox Environment & Team
                    <div class="detail-sidebar">
                        <div class="section-card">
                            <h3 class="card-subtitle">"Sandbox Container Status"</h3>
                            <div class="info-list">
                                <div class="info-row">
                                    <span class="info-label">"Container"</span>
                                    <span class="info-val">"apich-proj-qec-sim-user"</span>
                                </div>
                                <div class="info-row">
                                    <span class="info-label">"Isolation"</span>
                                    <span class="info-val">"Rootless OCI Sandbox"</span>
                                </div>
                                <div class="info-row">
                                    <span class="info-label">"Memory Allocated"</span>
                                    <span class="info-val">"2.0 GB / 2.0 Cores"</span>
                                </div>
                                <div class="info-row">
                                    <span class="info-label">"Active Mount"</span>
                                    <span class="info-val">"/workspace"</span>
                                </div>
                            </div>
                        </div>

                        <div class="section-card">
                            <h3 class="card-subtitle">"Project Team & Access"</h3>
                            <div class="collaborator-list">
                                <div class="collaborator-item">
                                    <div class="collab-avatar">"AL"</div>
                                    <div class="collab-info">
                                        <span class="collab-name">"Dr. Researcher"</span>
                                        <span class="collab-role">"Project Owner"</span>
                                    </div>
                                </div>
                                <div class="collaborator-item">
                                    <div class="collab-avatar">"CW"</div>
                                    <div class="collab-info">
                                        <span class="collab-name">"Carol Theory Lead"</span>
                                        <span class="collab-role">"Team Admin"</span>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
