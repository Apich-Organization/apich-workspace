use crate::app::components::PageShell;
use crate::ui::i18n::I18n;
use apich_islands::OrgSignupFieldsIsland;
use leptos::prelude::*;

#[component]
pub fn RegisterPage(
    error: Option<String>,
    registration_mode: String,
    i18n: I18n,
    current_path: String,
) -> impl IntoView {
    let mode_banner = match registration_mode.as_str() {
        "invite_only" => view! {
            <div class="alert alert-info" style="font-size:0.825rem; margin-bottom:1rem; padding:0.6rem 0.85rem; border-radius:var(--radius-sm);">
                "🔑 " {i18n.registration_mode_invite()}
            </div>
        }.into_any(),
        "admin_only" => view! {
            <div class="alert alert-warning" style="font-size:0.825rem; margin-bottom:1rem; padding:0.6rem 0.85rem; border-radius:var(--radius-sm);">
                "🚫 " {i18n.registration_mode_closed()}
            </div>
        }.into_any(),
        _ => view! {
            <div class="alert alert-info" style="font-size:0.825rem; margin-bottom:1rem; padding:0.6rem 0.85rem; border-radius:var(--radius-sm);">
                "✉️ " {i18n.registration_mode_open()}
            </div>
        }.into_any(),
    };

    let invite_required = registration_mode == "invite_only";
    let invite_field = if registration_mode != "admin_only" {
        Some(view! {
            <div class="form-group">
                <label for="invite_token">
                    {i18n.invite_code_label()}
                    {invite_required.then(|| view! { <span style="color:#dc2626;">" *"</span> })}
                </label>
                <input
                    type="text"
                    id="invite_token"
                    name="invite_token"
                    required=invite_required
                    placeholder=if invite_required { "inv_...".to_string() } else { i18n.invite_code_optional().to_string() }
                    class="form-control"
                />
            </div>
        })
    } else {
        None
    };

    let registration_closed = registration_mode == "admin_only";

    view! {
        <PageShell title=i18n.submit_register().to_string() i18n=i18n>
            <div class="auth-page">
                <div class="auth-card">
                    <div style="display:flex; justify-content:flex-end; margin-bottom:1rem;">
                        <a
                            href=format!("/set-lang?lang={}&return_to={}", i18n.lang.toggle_code(), urlencoding::encode(&current_path))
                            class="lang-toggle"
                        >
                            "🌐 " {i18n.lang.toggle_label()}
                        </a>
                    </div>
                    <div class="auth-header">
                        <div class="auth-logo">"APICH"</div>
                        <h1 class="auth-title">{i18n.register_title()}</h1>
                        <p class="auth-subtitle">{i18n.register_subtitle()}</p>
                    </div>
                    {error.map(|e| view! { <div class="alert alert-danger">{e}</div> })}
                    {mode_banner}
                    <form method="post" action="/register" class="auth-form">
                        <div class="form-group">
                            <label for="username">{i18n.username()}</label>
                            <input type="text" id="username" name="username" required=true placeholder="quantum_dev" class="form-control" autofocus=true />
                        </div>
                        <div class="form-group">
                            <label for="display_name">{i18n.display_name()}</label>
                            <input type="text" id="display_name" name="display_name" required=true placeholder="Dr. Alice Smith" class="form-control" />
                        </div>
                        <div class="form-group">
                            <label for="email">{i18n.email()}</label>
                            <input type="email" id="email" name="email" required=true placeholder="researcher@lab.org" class="form-control" />
                        </div>
                        <div class="form-group">
                            <label for="password">{i18n.password_label()}</label>
                            <input type="password" id="password" name="password" required=true placeholder="••••••••••••" class="form-control" />
                        </div>
                        {invite_field}

                        <OrgSignupFieldsIsland
                            create_org_label=i18n.create_org_prompt().to_string()
                            org_name_label=i18n.org_name().to_string()
                            org_slug_label=i18n.org_slug().to_string()
                        />

                        <button type="submit" class="btn btn-primary btn-block btn-lg" style="margin-top:0.5rem;" disabled=registration_closed>
                            {i18n.submit_register()}
                        </button>
                    </form>
                    <div class="auth-footer">
                        <p><a href="/login">{i18n.have_account_prompt()}</a></p>
                    </div>
                </div>
            </div>
        </PageShell>
    }
}
