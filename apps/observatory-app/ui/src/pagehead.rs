//! The head every page starts with (UI kit rule 4): the eyebrow breadcrumb, the title and a
//! muted subtitle with the page's facts on the right.

use leptos::prelude::*;

#[component]
pub fn PageHead(
    #[prop(into)] eyebrow: Signal<String>,
    #[prop(into)] subtitle: Signal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="pagehead" data-testid="page-head">
            <div>
                <div class="eyebrow" aria-label="eyebrow" data-testid="eyebrow">{move || eyebrow.get()}</div>
                {children()}
            </div>
            <span class="mono mut small" aria-label="page-subtitle" data-testid="page-subtitle">
                {move || subtitle.get()}
            </span>
        </div>
    }
}
