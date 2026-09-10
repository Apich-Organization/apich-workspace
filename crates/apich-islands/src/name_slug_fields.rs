//! Real Rust replacement for the several near-identical "type a name, auto-fill a URL slug"
//! form field pairs (`dashboard.rs`'s new-project modal, `org_teams.rs`'s new-org modal both
//! used to embed the same `oninput="...replace(/[^a-z0-9]+/g, '-')..."` one-liner).

use leptos::prelude::*;

#[island]
pub fn NameSlugFieldsIsland(
    #[prop(into)] name_field: String,
    #[prop(into)] slug_field: String,
    #[prop(into)] name_label: String,
    #[prop(into)] slug_label: String,
    #[prop(into)] name_placeholder: String,
    #[prop(into)] slug_placeholder: String,
) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let slug = RwSignal::new(String::new());

    view! {
        <div class="form-group">
            <label>{name_label}</label>
            <input
                type="text"
                name=name_field
                required=true
                placeholder=name_placeholder
                class="form-control"
                prop:value=move || name.get()
                on:input=move |ev| {
                    let v = event_target_value(&ev);
                    slug.set(slugify(&v));
                    name.set(v);
                }
            />
        </div>
        <div class="form-group">
            <label>{slug_label}</label>
            <input
                type="text"
                name=slug_field
                required=true
                placeholder=slug_placeholder
                class="form-control"
                prop:value=move || slug.get()
                on:input=move |ev| slug.set(event_target_value(&ev))
            />
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
