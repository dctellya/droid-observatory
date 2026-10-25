use leptos::prelude::*;
use leptos::task::spawn_local;
use observatory_core::config::{Config, UiConfig};
use observatory_core::model::MissionId;
use observatory_core::view::{
    AUTONOMY_LEVELS, ControlBar, ControlSent, HostedState, HostedStatus, LaunchError, LaunchField,
    LaunchForm, Launched, LiveOutput,
};
use serde_json::json;

use crate::controls::{ControlCtx, Controls};
use crate::shell;

/// What the launch screen, the agent panel and the root share about the hosted run: the run the
/// shell started, its status and the latest lines of its output.
#[derive(Clone, Copy)]
pub struct HostedCtx {
    pub launched: RwSignal<Option<Launched>>,
    pub status: RwSignal<Option<HostedStatus>>,
    pub output: RwSignal<LiveOutput, LocalStorage>,
    /// What the controls wrote, newest last.
    pub sent: RwSignal<Vec<ControlSent>>,
}

impl HostedCtx {
    pub fn new() -> Self {
        Self {
            launched: RwSignal::new(None),
            status: RwSignal::new(None),
            output: RwSignal::new_local(LiveOutput::new(UiConfig::default().live_output_lines)),
            sent: RwSignal::new(Vec::new()),
        }
    }
}

fn unavailable() -> impl IntoView {
    view! {
        <p aria-label="launch-unavailable" role="status">
            "Launch not available in browser host: open the Tauri window (make app-dev) to run a mission."
        </p>
    }
}

/// FR-L3: the form that starts a hosted mission, and the hosted run once it has started.
#[component]
pub fn LaunchScreen(on_close: Callback<()>) -> impl IntoView {
    let tauri = shell::is_tauri();
    let hosted = expect_context::<HostedCtx>();
    let form = RwSignal::new(LaunchForm::defaults(&Config::default().mission));
    let errors = RwSignal::new(Vec::<LaunchError>::new());
    let failure = RwSignal::new(None::<String>);
    let starting = RwSignal::new(false);

    if tauri {
        spawn_local(async move {
            if let Ok(config) = shell::invoke::<_, Config>("app_config", &json!({})).await {
                let untouched = LaunchForm::defaults(&Config::default().mission);
                if form.get_untracked() == untouched {
                    form.set(LaunchForm::defaults(&config.mission));
                }
            }
        });
    }

    let launch = move |_| {
        failure.set(None);
        let config = match form.get_untracked().validate() {
            Ok(config) => config,
            Err(found) => {
                errors.set(found);
                return;
            }
        };
        errors.set(Vec::new());
        if !tauri {
            return;
        }
        starting.set(true);
        spawn_local(async move {
            match shell::invoke::<_, Launched>("host_launch", &json!({ "config": config })).await {
                Ok(launched) => {
                    hosted
                        .status
                        .set(Some(HostedStatus::starting(&launched.launch_id)));
                    hosted.sent.set(Vec::new());
                    hosted.launched.set(Some(launched));
                }
                Err(message) => failure.set(Some(message)),
            }
            starting.set(false);
        });
    };

    view! {
        <section class="launch" aria-label="launch-screen">
            <h1>"Launch a mission"</h1>
            <button type="button" aria-label="launch-back" data-testid="launch-back" on:click=move |_| on_close.run(())>
                "Back to the library"
            </button>
            {(!tauri).then(unavailable)}
            <form
                class="launch-form"
                on:submit=move |ev| {
                    ev.prevent_default();
                    launch(());
                }
            >
                {LaunchField::ALL
                    .into_iter()
                    .map(|field| field_view(field, form, errors))
                    .collect_view()}
                <button type="submit" aria-label="launch" data-testid="launch" disabled=move || starting.get()>
                    "Launch"
                </button>
            </form>
            <Show when=move || failure.get().is_some()>
                <p role="alert" class="error" aria-label="launch-failure">
                    {move || failure.get()}
                </p>
            </Show>
            {move || hosted.launched.get().map(|launched| hosted_view(hosted, launched))}
        </section>
    }
}

/// `launch-field-mission-branch-name`: the test id of a field, from its label.
fn field_testid(field: LaunchField) -> String {
    let slug: String = field
        .label()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let words: Vec<&str> = slug.split('-').filter(|word| !word.is_empty()).collect();
    format!("launch-field-{}", words.join("-"))
}

fn field_view(
    field: LaunchField,
    form: RwSignal<LaunchForm>,
    errors: RwSignal<Vec<LaunchError>>,
) -> impl IntoView {
    let label = field.label();
    let message = move || {
        errors.with(|errors| {
            errors
                .iter()
                .find(|error| error.field == field)
                .map(|error| error.message.clone())
        })
    };
    let control = if field == LaunchField::Autonomy {
        view! {
            <select
                aria-label=label
                data-testid=field_testid(field)
                prop:value=move || form.with(|form| form.value(field).to_owned())
                on:change=move |ev| form.update(|form| form.set(field, event_target_value(&ev)))
            >
                {AUTONOMY_LEVELS
                    .into_iter()
                    .map(|level| view! { <option value=level>{level}</option> })
                    .collect_view()}
            </select>
        }
        .into_any()
    } else {
        view! {
            <input
                type="text"
                aria-label=label
                data-testid=field_testid(field)
                prop:value=move || form.with(|form| form.value(field).to_owned())
                on:input=move |ev| form.update(|form| form.set(field, event_target_value(&ev)))
            />
        }
        .into_any()
    };
    view! {
        <div class="field">
            <label>
                <span class="field-label">{label}</span>
                " "
                {control}
            </label>
            <Show when=move || message().is_some()>
                <p role="alert" class="error" aria-label="launch-error">
                    {message}
                </p>
            </Show>
        </div>
    }
}

fn state_text(state: &HostedState) -> String {
    match state {
        HostedState::Starting => "starting".to_owned(),
        HostedState::Running => "running".to_owned(),
        HostedState::Exited { code } => match code {
            Some(code) => format!("exited ({code})"),
            None => "exited (signal)".to_owned(),
        },
    }
}

/// The hosted run: what was spawned, pause/resume/message, the questions that wait, what was
/// written and acknowledged, and its output.
fn hosted_view(hosted: HostedCtx, launched: Launched) -> impl IntoView {
    let status = move || hosted.status.get();
    let control = ControlCtx {
        mission: MissionId(launched.session_id.clone()),
        launch_id: Some(launched.launch_id.clone()),
        hosted,
        result: RwSignal::new(None),
        paste: RwSignal::new(None),
        stalled: Signal::derive(Vec::new),
        enabled: Signal::derive(|| true),
    };
    let bar = Signal::derive(move || {
        ControlBar::hosted(
            &hosted
                .status
                .get()
                .unwrap_or_else(|| HostedStatus::starting("")),
            None,
        )
    });
    view! {
        <section class="hosted" aria-label="hosted-run" data-testid="hosted-run">
            <h2>"Hosted mission"</h2>
            <p>
                "State: "
                <strong aria-label="hosted-state" data-testid="hosted-state">
                    {move || status().map(|s| state_text(&s.state))}
                </strong>
                ", pid " <span aria-label="hosted-pid">{launched.pid}</span> ", session "
                <code aria-label="hosted-session">{launched.session_id.clone()}</code>
            </p>
            <p>
                <code aria-label="hosted-command">{launched.command.clone()}</code>
            </p>
            <p>
                "Spool " <code aria-label="hosted-spool">{launched.spool.clone()}</code>
            </p>
            <Controls ctx=control bar=bar />
            <h3>"Acknowledged by Droid"</h3>
            <ul aria-label="host-acks" data-testid="host-acks">
                {move || {
                    status()
                        .map(|s| s.acks)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|ack| {
                            view! {
                                <li aria-label="host-ack-row">
                                    <code>{ack.request_id}</code>
                                    " "
                                    {ack.method}
                                    " "
                                    {if ack.accepted { "accepted".to_owned() } else {
                                        format!("rejected: {}", ack.message.unwrap_or_default())
                                    }}
                                </li>
                            }
                        })
                        .collect_view()
                }}
            </ul>
            <LiveOutputView />
        </section>
    }
}

/// FR-A3: the latest masked lines of the hosted stream and the count of every line the spool
/// holds.
#[component]
pub fn LiveOutputView() -> impl IntoView {
    let hosted = expect_context::<HostedCtx>();
    view! {
        <section class="live-output" aria-label="live-output">
            <h3>
                "Live output " <span class="count" aria-label="live-output-total">
                    {move || hosted.output.with(LiveOutput::total)}
                </span> " lines"
            </h3>
            <pre class="mono" aria-label="live-output-lines">
                {move || {
                    hosted
                        .output
                        .with(|output| {
                            output
                                .lines()
                                .iter()
                                .map(|line| line.text.clone())
                                .collect::<Vec<_>>()
                                .join("\n")
                        })
                }}
            </pre>
        </section>
    }
}
