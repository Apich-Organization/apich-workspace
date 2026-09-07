use leptos::prelude::*;

#[component]
pub fn RegisterPage() -> impl IntoView {
    view! {
        <div class="auth-page">
            <div class="auth-card">
                <div class="auth-header">
                    <div class="auth-logo">"APICH"</div>
                    <h1 class="auth-title">"Create your Research Account"</h1>
                    <p class="auth-subtitle">"Join collaborative technical workflows and containerized research suites"</p>
                </div>

                <form method="post" action="/api/auth/register" class="auth-form">
                    <div class="form-group">
                        <label for="display_name">"Full Name / Display Name"</label>
                        <input
                            type="text"
                            id="display_name"
                            name="display_name"
                            required
                            placeholder="Dr. Jane Doe"
                            class="form-control"
                        />
                    </div>
                    <div class="form-group">
                        <label for="username">"Username"</label>
                        <input
                            type="text"
                            id="username"
                            name="username"
                            required
                            placeholder="jdoe"
                            class="form-control"
                        />
                    </div>
                    <div class="form-group">
                        <label for="email">"Institutional Email"</label>
                        <input
                            type="email"
                            id="email"
                            name="email"
                            required
                            placeholder="jane.doe@university.edu"
                            class="form-control"
                        />
                    </div>
                    <div class="form-group">
                        <label for="password">"Password"</label>
                        <input
                            type="password"
                            id="password"
                            name="password"
                            required
                            placeholder="••••••••••••"
                            class="form-control"
                        />
                    </div>
                    <div class="form-group">
                        <label for="invite_token">
                            "Invitation Code "
                            <span class="text-muted">"(Required if self-registration is invite-only)"</span>
                        </label>
                        <input
                            type="text"
                            id="invite_token"
                            name="invite_token"
                            placeholder="Enter 32-character invite token"
                            class="form-control"
                        />
                    </div>

                    <button type="submit" class="btn btn-primary btn-block btn-lg">"Complete Registration"</button>
                </form>

                <div class="auth-footer">
                    <p>"Already have an account? "<a href="/login">"Sign In"</a></p>
                </div>
            </div>
        </div>
    }
}
