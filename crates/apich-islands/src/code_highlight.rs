//! Shared syntax-highlighting overlay for every plain `<textarea>` code editor in this crate
//! (`document_editor.rs`'s script/Typst/LaTeX source panel, `note_editor.rs`'s markdown body).
//!
//! A `<textarea>` can only ever show flat monospace text -- there is no way to color individual
//! characters inside one, in any browser. Real highlighting needs a *second*, purely decorative
//! element showing colored text, with the real (interactive) textarea placed on top of it with
//! its own text made invisible (`color: transparent`) but its caret kept visible
//! (`caret-color: ...`) -- the classic "highlighted textarea" technique (the same one `CodeMirror`
//! 5 and every textarea-based highlighter uses). This keeps every existing textarea-based
//! behavior in this codebase (Ctrl+S save, `#code-editor-input`-targeted cursor insert, jump-to-
//! line selection, the script-runner's Ctrl+Enter) working completely unchanged -- `#code-editor-
//! input` is still a real, focusable, selectable `<textarea>`; only its visible glyphs move to
//! the overlay behind it.
//!
//! Highlighting itself is Prism.js (loaded from the CDN allowlist, autoloading only the specific
//! language grammar a given file actually needs) -- hand-rolling tokenizers for six languages in
//! Rust/WASM would be its own large project for a purely cosmetic feature. Typst isn't one of
//! Prism's bundled languages, so its (small, hand-written) grammar is registered directly in
//! `CODE_HIGHLIGHT_JS` instead of being autoloaded.
//!
//! Deliberately NOT loading Prism's own theme CSS: `Prism.highlightElement()` (needed so the
//! autoloader plugin's lazy-fetch-then-retry can hook in -- the lower-level `Prism.highlight()`
//! string API never triggers it) mutates BOTH the `<code>` element *and its `<pre>` parent*,
//! stamping `language-<lang>` onto each. Prism's theme then targets `pre[class*="language-"]` /
//! `code[class*="language-"]` with layout-affecting rules (padding, margin, font-size,
//! line-height, white-space) at higher CSS specificity than this file's own `.code-highlight-
//! overlay` class -- confirmed live via `getComputedStyle`: the overlay silently gained Prism's
//! `padding:1em;margin:.5em 0` etc. instead of the padding/font this technique requires to stay
//! pixel-for-pixel aligned with the real (invisible) textarea underneath it. The user-visible
//! result was exactly what you'd expect from a misaligned click-through overlay: clicking one
//! character visually landed the cursor on a different one in the real textarea, and a
//! reverse-search jump's selection highlight appeared shifted from the colored text it was
//! supposedly selecting. Styling every `.token.*` color by hand instead (see `styles.rs`) removes
//! Prism's base layout rules from the page entirely, so there's nothing left to fight.
//!
//! Deliberately staying an eager inline `<script>` rather than becoming a Leptos `on:mount`/
//! `Effect` island: every other plain-JS-to-island conversion this session (`ai_drawer.rs`,
//! `jump_to_line.rs`'s Typst/markdown/note callers) was safe specifically because a user can only
//! reach a click target *after* the page has already rendered it, by which point hydration has
//! reliably finished. Initial syntax coloring is different -- it's what the user sees at first
//! paint, with no interaction required first, and first paint by definition happens before
//! hydration completes. Gating it on an `Effect` would trade today's near-instant highlight for a
//! guaranteed flash of unstyled textarea content on every page load: the same hydration-race bug
//! class fixed earlier this session, reintroduced in the opposite direction. Combined with Prism
//! itself being an external CDN library with no Rust/WASM equivalent (see above), this stays
//! eager plain JS -- the same category as the LaTeX PDF viewer's PDF.js orchestration in
//! `document_editor.rs` (`LATEX_PDF_VIEWER_JS`), for the same reason.

/// Maps a file extension to the Prism.js language id needed for
/// `Prism.languages[id]`/the autoloader's `components/prism-<id>.min.js`. Falls back to `""`
/// (no highlighting, but the overlay still mirrors the plain escaped text) for anything with no
/// well-known grammar, rather than guessing wrong.
pub(crate) fn prism_lang_for_ext(ext: &str) -> &'static str {
    match ext {
        | "py" => "python",
        | "r" => "r",
        | "rs" => "rust",
        | "tex" | "latex" => "latex",
        | "sh" | "bash" => "bash",
        | "js" => "javascript",
        | "ts" => "typescript",
        | "md" | "markdown" => "markdown",
        | "typ" => "typst",
        | "json" => "json",
        | "toml" => "toml",
        | "yaml" | "yml" => "yaml",
        | _ => "",
    }
}

pub(crate) const CODE_HIGHLIGHT_JS: &str = r#"
(function(){
    function escapeHtml(s){
        return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
    }

    function registerTypstGrammar(){
        if (!window.Prism || window.Prism.languages.typst) { return; }
        window.Prism.languages.typst = {
            'comment': [/\/\/.*/, /\/\*[\s\S]*?\*\//],
            'string': {pattern: /"(?:[^"\\]|\\.)*"/, greedy: true},
            'heading': {pattern: /^[ \t]*=+[^\n]*/m, alias: 'title'},
            'function': /#[a-zA-Z_][\w-]*/,
            'keyword': /\b(?:let|set|show|import|include|if|else|for|while|return|break|continue|in|as)\b/,
            'number': /\b\d+(?:\.\d+)?(?:em|pt|cm|mm|in|fr|%)?\b/,
            'punctuation': /[{}()\[\],.:;]/
        };
    }

    var prismLoading = null;
    function ensurePrism(cb){
        if (window.Prism) { registerTypstGrammar(); cb(); return; }
        if (!prismLoading) {
            prismLoading = new Promise(function(resolve){
                // No theme CSS loaded here on purpose -- see this file's module doc comment.
                var core = document.createElement('script');
                core.src = 'https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/prism.min.js';
                core.onload = function(){
                    var auto = document.createElement('script');
                    auto.src = 'https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/plugins/autoloader/prism-autoloader.min.js';
                    auto.onload = function(){
                        window.Prism.plugins.autoloader.languages_path = 'https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/';
                        registerTypstGrammar();
                        resolve();
                    };
                    document.head.appendChild(auto);
                };
                document.head.appendChild(core);
            });
        }
        prismLoading.then(cb);
    }

    function wireOne(wrap){
        var ta = wrap.querySelector('textarea');
        var codeEl = wrap.querySelector('pre code');
        var pre = wrap.querySelector('pre');
        if (!ta || !codeEl || !pre) { return; }
        var lang = wrap.dataset.lang || '';
        if (lang) { codeEl.className = 'language-' + lang; }

        function finalize(){
            // A trailing newline keeps the overlay's scrollHeight matched to the textarea's even
            // when the real content ends without one -- otherwise the last visual line can end
            // up one line short of where the (invisible) textarea actually scrolls to.
            if (codeEl.innerHTML.slice(-1) !== '\n') { codeEl.innerHTML += '\n'; }
            // `highlightElement` also mutates `pre` (adds the same `language-*` class, plus
            // `tabindex="0"`) -- strip both. No theme stylesheet targets `[class*="language-"]`
            // anymore, but keeping this element's class list exactly what this file's own CSS
            // expects removes any chance of a future page addition doing the same thing again.
            pre.className = 'code-highlight-overlay';
            pre.removeAttribute('tabindex');
        }
        function render(){
            var text = ta.value;
            if (window.Prism && lang) {
                // `highlightElement`, not the lower-level `Prism.highlight()`: the autoloader
                // plugin's lazy-fetch-then-retry logic is a hook registered on
                // `highlightElement`/`highlightAllUnder` specifically -- calling the bare
                // `Prism.highlight(text, grammar, lang)` API instead (tried first) never fires
                // that hook, so a language whose grammar hadn't loaded yet just silently stayed
                // unhighlighted forever, confirmed live (0 `.token` elements for python/r/rust
                // even after Prism core had loaded -- only Typst, whose grammar is registered
                // synchronously above with no fetch involved, worked on the first attempt).
                codeEl.textContent = text;
                window.Prism.highlightElement(codeEl, false, finalize);
            } else {
                codeEl.innerHTML = escapeHtml(text);
                finalize();
            }
        }
        function syncScroll(){
            pre.scrollTop = ta.scrollTop;
            pre.scrollLeft = ta.scrollLeft;
        }

        ta.addEventListener('input', render);
        ta.addEventListener('scroll', syncScroll);
        render();
        ensurePrism(render);
        wrap.__apichRender = render;
    }

    // Lets code OUTSIDE this file (e.g. `note_editor.rs`'s task-checkbox toggle, which replaces
    // `ta.value` directly via `fetch()` -- no keystroke involved) refresh the highlight overlay
    // without going through a native `input` event. Dispatching a real `input` event was tried
    // first and reverted: this textarea also has Leptos's own `on:input` handler on it for the
    // live-preview debounce, and a dispatched event doesn't distinguish "the highlight overlay
    // wants a repaint" from "the user typed something" -- it fired *both*, so every checkbox
    // click was silently triggering an extra ~500ms-later full remote preview re-render the
    // original design never did (confirmed live: it was destroying and recreating the very
    // checkbox DOM node the click had just come from).
    window.__apichRefreshHighlight = function(textareaId){
        var ta = document.getElementById(textareaId);
        var wrap = ta && ta.closest('.code-editor-wrap');
        if (wrap && wrap.__apichRender) { wrap.__apichRender(); }
    };

    function initAll(){
        document.querySelectorAll('.code-editor-wrap[data-lang]').forEach(function(w){
            if (w.__apichHighlightWired) { return; }
            w.__apichHighlightWired = true;
            wireOne(w);
        });
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initAll);
    } else {
        initAll();
    }
})();
"#;
