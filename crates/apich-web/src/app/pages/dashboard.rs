use crate::app::components::{Navbar, StatCard, StatusBadge};
use leptos::prelude::*;

#[component]
pub fn DashboardPage() -> impl IntoView {
    view! {
        <div class="dashboard-page">
            <Navbar user_name="Dr. Researcher".to_string() is_admin=true />

            <div class="main-content">
                <div class="page-header">
                    <div>
                        <h1 class="page-title">"Research Projects & Sandboxes"</h1>
                        <p class="page-subtitle">"Manage your computational environments, versioned datasets, and collaborative workspaces"</p>
                    </div>
                    <div class="header-actions">
                        <button class="btn btn-primary" id="btn-new-project">
                            <span class="btn-icon">"+"</span>
                            " New Project"
                        </button>
                    </div>
                </div>

                // Metrics Overview
                <div class="stats-grid">
                    <StatCard title="Active Projects" value="4".to_string() subtitle="Across 2 organizations" />
                    <StatCard title="Running Sandboxes" value="2".to_string() subtitle="Dedicated isolated containers" />
                    <StatCard title="Storage Allocated" value="1.4 GB".to_string() subtitle="Of 20 GB quota" />
                    <StatCard title="Recent Snapshots" value="18".to_string() subtitle="Last snapshot 12m ago" />
                </div>

                // Projects List Section
                <div class="section-card">
                    <div class="section-header">
                        <h2 class="section-title">"Active Workspace Projects"</h2>
                        <span class="text-muted">"Each project runs in a secure, resource-bounded environment with automatic timeline versioning"</span>
                    </div>

                    <div class="projects-grid">
                        // Sample Project 1
                        <div class="project-card">
                            <div class="project-card-header">
                                <div>
                                    <h3 class="project-name">
                                        <a href="/projects/proj-qec-sim">"Rotated Surface Code Simulator"</a>
                                    </h3>
                                    <span class="team-tag">"Quantum Theory / Error Correction"</span>
                                </div>
                                <StatusBadge status="running" is_running=true />
                            </div>
                            <p class="project-desc">
                                "High-threshold Monte Carlo simulations for 2D topological surface code fault tolerance."
                            </p>
                            <div class="project-footer">
                                <div class="project-meta">
                                    <span>"Storage: 420 MB"</span>
                                    <span>"•"</span>
                                    <span>"Updated 15 mins ago"</span>
                                </div>
                                <div class="project-actions">
                                    <form method="post" action="/api/projects/sample-id/sandbox/stop">
                                        <button type="submit" class="btn btn-secondary btn-sm">"Stop Sandbox"</button>
                                    </form>
                                    <a href="/projects/sample-id" class="btn btn-primary btn-sm">"Open Workspace"</a>
                                </div>
                            </div>
                        </div>

                        // Sample Project 2
                        <div class="project-card">
                            <div class="project-card-header">
                                <div>
                                    <h3 class="project-name">
                                        <a href="/projects/proj-nlp-paper">"Transformer Attention Benchmarks"</a>
                                    </h3>
                                    <span class="team-tag">"Machine Intelligence Group"</span>
                                </div>
                                <StatusBadge status="stopped" is_running=false />
                            </div>
                            <p class="project-desc">
                                "Empirical comparative study of multi-query and grouped-query attention architectures."
                            </p>
                            <div class="project-footer">
                                <div class="project-meta">
                                    <span>"Storage: 180 MB"</span>
                                    <span>"•"</span>
                                    <span>"Updated 2 days ago"</span>
                                </div>
                                <div class="project-actions">
                                    <form method="post" action="/api/projects/sample-id-2/sandbox/start">
                                        <button type="submit" class="btn btn-primary btn-sm">"Launch Sandbox"</button>
                                    </form>
                                    <a href="/projects/sample-id-2" class="btn btn-ghost btn-sm">"View Details"</a>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>

                // New Project Modal Form
                <div id="modal-new-project" class="modal-dialog hidden">
                    <div class="modal-content">
                        <div class="modal-header">
                            <h3 class="modal-title">"Create New Research Project"</h3>
                            <button class="modal-close" id="btn-close-modal">"&times;"</button>
                        </div>
                        <form method="post" action="/api/projects" class="modal-form">
                            <div class="form-group">
                                <label for="proj_name">"Project Name"</label>
                                <input type="text" id="proj_name" name="name" required placeholder="Quantum Annealing Benchmarks" class="form-control" />
                            </div>
                            <div class="form-group">
                                <label for="proj_slug">"URL Identifier (Slug)"</label>
                                <input type="text" id="proj_slug" name="slug" required placeholder="quantum-annealing-bench" class="form-control" />
                            </div>
                            <div class="form-group">
                                <label for="proj_desc">"Description"</label>
                                <textarea id="proj_desc" name="description" rows="3" placeholder="Brief summary of research objectives and experimental setup" class="form-control"></textarea>
                            </div>
                            <div class="modal-actions">
                                <button type="button" class="btn btn-ghost" id="btn-cancel-modal">"Cancel"</button>
                                <button type="submit" class="btn btn-primary">"Initialize Project & Workspace"</button>
                            </div>
                        </form>
                    </div>
                </div>
            </div>
        </div>
    }
}
