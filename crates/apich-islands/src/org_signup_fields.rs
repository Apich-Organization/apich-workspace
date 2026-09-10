//! Real Rust replacement for the registration form's small "create an organization" toggle +
//! name-to-slug autofill (`register.rs`'s `ORG_TOGGLE_SCRIPT`). Renders as real fields inside
//! the surrounding server-rendered `<form method="post" action="/register">` -- islands render
//! inline in the DOM tree at their call site, so these still submit natively with the rest of
//! the form.

use leptos::prelude::*;

#[island]
pub fn OrgSignupFieldsIsland(
    #[prop(into)] create_org_label: String,
    #[prop(into)] org_name_label: String,
    #[prop(into)] org_slug_label: String,
) -> impl IntoView {
    let checked = RwSignal::new(false);
    let org_name = RwSignal::new(String::new());
    let org_slug = RwSignal::new(String::new());

    view! {
        <div style="margin-top:0.75rem; margin-bottom:0.75rem;">
            <label style="display:flex; align-items:center; gap:0.5rem; font-size:0.875rem; cursor:pointer; font-weight:600; color:var(--text-main);">
                <input
                    type="checkbox"
                    name="create_org"
                    value="1"
                    prop:checked=move || checked.get()
                    on:change=move |ev| checked.set(event_target_checked(&ev))
                />
                {create_org_label}
            </label>
            <div class="org-expand-box" style:display=move || if checked.get() { "block" } else { "none" }>
                <div class="form-group">
                    <label>{org_name_label}</label>
                    <input
                        type="text"
                        name="org_name"
                        placeholder="Quantum Dynamics Laboratory"
                        class="form-control"
                        prop:value=move || org_name.get()
                        on:input=move |ev| {
                            let v = event_target_value(&ev);
                            org_slug.set(slugify(&v));
                            org_name.set(v);
                        }
                    />
                </div>
                <div class="form-group">
                    <label>{org_slug_label}</label>
                    <input
                        type="text"
                        name="org_slug"
                        placeholder="quantum-dynamics-lab"
                        class="form-control"
                        prop:value=move || org_slug.get()
                        on:input=move |ev| org_slug.set(event_target_value(&ev))
                    />
                </div>
            </div>
        </div>
    }
}

fn slugify(s: &str) -> String {
    let mut out = String::new();
    let mut last_was_dash = false;
    for c in s.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_was_dash = false;
        } else if !last_was_dash && !out.is_empty() {
            out.push('-');
            last_was_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}
