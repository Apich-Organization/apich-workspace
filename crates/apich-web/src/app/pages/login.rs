use leptos::prelude::*;

#[component]
pub fn LoginPage() -> impl IntoView {
    view! {
        <div class="auth-page">
            <div class="auth-card">
                <div class="auth-header">
                    <div class="auth-logo">"APICH"</div>
                    <h1 class="auth-title">"Sign in to your Research Workspace"</h1>
                    <p class="auth-subtitle">"Unified computational workspace, version control, and multi-tenant sandboxes"</p>
                </div>

                <div class="passkey-section">
                    <button type="button" id="btn-passkey-login" class="btn btn-primary btn-block btn-lg">
                        <svg class="icon" viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M12 2a5 5 0 0 0-5 5v3H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8a2 2 0 0 0-2-2h-1V7a5 5 0 0 0-5-5zm-3 5a3 3 0 1 1 6 0v3H9V7z"/>
                        </svg>
                        " Sign In with Passkey / Biometrics"
                    </button>
                    <p class="hint-text">"Hardware security keys, Touch ID, Windows Hello, and FIDO2 supported"</p>
                </div>

                <div class="divider">
                    <span>"OR CONTINUE WITH PASSWORD"</span>
                </div>

                <form method="post" action="/api/auth/login" class="auth-form">
                    <div class="form-group">
                        <label for="username_or_email">"Email or Username"</label>
                        <input
                            type="text"
                            id="username_or_email"
                            name="username_or_email"
                            required
                            placeholder="researcher@institution.org"
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
                    <button type="submit" class="btn btn-secondary btn-block">"Sign In"</button>
                </form>

                <div class="auth-footer">
                    <p>"Need an account? "<a href="/register">"Register with invitation code"</a></p>
                </div>
            </div>
        </div>
    }
}
