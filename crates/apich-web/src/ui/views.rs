use crate::app::styles::EMBEDDED_CSS;
use crate::ui::i18n::I18n;

/// Render standard HTML5 page shell with embedded styling and language tags. Used only by the
/// 404 fallback now -- every real page is a Leptos component in `app::pages::*`, rendered
/// directly by its handler in `ui::handlers` via `app::components::render_document`.
pub fn render_page(title: &str, body_html: &str, i18n: &I18n) -> String {
    let lang_attr = if i18n.is_zh() { "zh-CN" } else { "en" };
    format!(
        r#"<!DOCTYPE html>
<html lang="{}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - APICH {}</title>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.8/dist/katex.min.css">
    <script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.8/dist/katex.min.js"></script>
    <script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.8/dist/contrib/auto-render.min.js" onload="try{{renderMathInElement(document.body,{{delimiters:[{{left:'$$',right:'$$',display:true}},{{left:'$',right:'$',display:false}}]}})}}catch(e){{}}"></script>
    <style>{}</style>
</head>
<body>
    <div id="app">{}</div>
</body>
</html>"#,
        lang_attr,
        html_escape(title),
        i18n.brand_title(),
        EMBEDDED_CSS,
        body_html
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
