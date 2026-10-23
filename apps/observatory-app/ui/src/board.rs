use leptos::prelude::*;
use observatory_core::view::BoardView;

#[component]
pub fn Board() -> impl IntoView {
    let board = BoardView::empty();
    let lanes = board
        .lanes
        .into_iter()
        .map(|lane| {
            view! {
                <section class="lane" aria-label=lane.aria_label>
                    <h2 class="lane-name">{lane.name}</h2>
                    <div class="cards"></div>
                </section>
            }
        })
        .collect_view();

    view! { <main class="board" aria-label="board">{lanes}</main> }
}
