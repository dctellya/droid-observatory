use leptos::prelude::*;
use observatory_core::model::CycleTemplate;
use observatory_core::view::BoardView;

use crate::perf;

/// Time, tokens per role and model drift against the declared models. Cost and waste arrive
/// with M5.
#[component]
pub fn Panel(board: Memo<BoardView>) -> impl IntoView {
    // Runs once the Panel's rows are in the DOM; the measure it closes was opened by the request
    // that produced this board, so live batches add none.
    Effect::new(move |_| {
        board.track();
        perf::panel_rendered();
    });
    view! {
        <section class="panel" data-testid="panel" aria-label=move || board.with(|b| b.panel.aria_label.clone())>
            <h2>"Panel"</h2>
            <p aria-label="panel-elapsed" data-testid="panel-elapsed">
                "Elapsed: "
                {move || {
                    board
                        .with(|b| b.panel.elapsed_ms)
                        .map_or_else(|| "n/a".to_owned(), |ms| format!("{} s", ms / 1000))
                }}
            </p>
            <div class="lib"><div class="scroll">
            <table aria-label="panel-roles" data-testid="panel-roles">
                <thead>
                    <tr>
                        <th>"Role"</th>
                        <th>"Sessions"</th>
                        <th>"Input"</th>
                        <th>"Output"</th>
                        <th>"Cache read"</th>
                        <th>"Cache write"</th>
                        <th>"Turns"</th>
                        <th>"Turn input"</th>
                        <th>"Turn output"</th>
                        <th>"Turn cache read"</th>
                        <th>"Declared model"</th>
                        <th>"Observed models"</th>
                        <th>"Drift"</th>
                    </tr>
                </thead>
                <tbody>
                    {move || {
                        board
                            .with(|b| {
                                b.panel
                                    .roles
                                    .iter()
                                    .map(|row| {
                                        let observed = row
                                            .observed
                                            .iter()
                                            .map(|o| format!("{} ({})", o.model, o.events))
                                            .collect::<Vec<_>>()
                                            .join(", ");
                                        view! {
                                            <tr aria-label=format!("panel-role-{}", row.role.as_str())>
                                                <td>{row.role.as_str()}</td>
                                                <td class="n">{row.sessions}</td>
                                                <td class="n">{row.tokens.input}</td>
                                                <td class="n">{row.tokens.output}</td>
                                                <td class="n">{row.tokens.cache_read}</td>
                                                <td class="n">{row.tokens.cache_write}</td>
                                                <td class="n" aria-label=format!("panel-turns-{}", row.role.as_str())>{row.turns}</td>
                                                <td class="n">{row.turn_input}</td>
                                                <td class="n">{row.turn_output}</td>
                                                <td class="n">{row.turn_cache_read}</td>
                                                <td class="mono" aria-label="declared-model">{row.declared.0.clone()}</td>
                                                <td class="mono" aria-label="observed-models">{observed}</td>
                                                <td aria-label="model-drift">
                                                    {if row.drifted { "drift" } else { "as declared" }}
                                                </td>
                                            </tr>
                                        }
                                    })
                                    .collect_view()
                            })
                    }}
                </tbody>
            </table>
            </div></div>
        </section>
    }
}

/// The overlay filter list: events a person caused, each also marked on its phase lane.
#[component]
pub fn OverlayList(board: Memo<BoardView>) -> impl IntoView {
    let template = expect_context::<RwSignal<CycleTemplate>>();
    view! {
        <section class="overlay-list" aria-label=move || board.with(|b| b.overlay_human.aria_label.clone())>
            <h2>
                "Human overlay markers "
                <span class="count" aria-label="overlay-human-count">
                    {move || board.with(|b| b.overlay_human.markers.len())}
                </span>
            </h2>
            <ul>
                {move || {
                    board
                        .with(|b| {
                            b.overlay_human
                                .markers
                                .iter()
                                .map(|marker| {
                                    view! {
                                        <li aria-label="overlay-human-marker">
                                            {marker.at.to_rfc3339_millis()} " "
                                            {marker.kind.clone()} " on " {template.with(|template| template.name_of(&marker.lane))}
                                        </li>
                                    }
                                })
                                .collect_view()
                        })
                }}
            </ul>
        </section>
    }
}
