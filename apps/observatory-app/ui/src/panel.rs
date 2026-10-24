use leptos::prelude::*;
use observatory_core::view::BoardView;

/// Time, tokens per role and model drift against the declared models. Cost and waste arrive
/// with M5.
#[component]
pub fn Panel(board: Memo<BoardView>) -> impl IntoView {
    view! {
        <section class="panel" aria-label=move || board.with(|b| b.panel.aria_label.clone())>
            <h2>"Panel"</h2>
            <p aria-label="panel-elapsed">
                "Elapsed: "
                {move || {
                    board
                        .with(|b| b.panel.elapsed_ms)
                        .map_or_else(|| "n/a".to_owned(), |ms| format!("{} s", ms / 1000))
                }}
            </p>
            <table aria-label="panel-roles">
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
                                                <td>{row.sessions}</td>
                                                <td>{row.tokens.input}</td>
                                                <td>{row.tokens.output}</td>
                                                <td>{row.tokens.cache_read}</td>
                                                <td>{row.tokens.cache_write}</td>
                                                <td aria-label=format!("panel-turns-{}", row.role.as_str())>{row.turns}</td>
                                                <td>{row.turn_input}</td>
                                                <td>{row.turn_output}</td>
                                                <td>{row.turn_cache_read}</td>
                                                <td aria-label="declared-model">{row.declared.0.clone()}</td>
                                                <td aria-label="observed-models">{observed}</td>
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
        </section>
    }
}

/// The overlay filter list: events a person caused, each also marked on its phase lane.
#[component]
pub fn OverlayList(board: Memo<BoardView>) -> impl IntoView {
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
                                            {marker.kind.clone()} " on " {marker.lane.name()}
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
