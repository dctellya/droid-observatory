//! The controls of a mission (FR-H1 to FR-H4) and the stall banner (FR-H5), built from the kit's
//! components: KPI tiles, the `.lib` table, chips, the pill switch and cards. Which control is
//! enabled, what an attached mission shows and what a flag says come from `core::view`.

use std::collections::HashMap;

use leptos::prelude::*;
use leptos::task::spawn_local;
use observatory_core::control::HumanAction;
use observatory_core::model::{EventId, FeatureId, MissionId};
use observatory_core::view::{
    ActionRecorded, ControlBar, HostedStatus, QuestionRow, RunState, StallBanner, StallFlagView,
    terminal_text,
};
use serde_json::json;

use crate::launch::HostedCtx;
use crate::shell;

/// The stall flags of every mission the shell has reported, by mission id.
#[derive(Clone, Copy)]
pub struct StallCtx(pub RwSignal<HashMap<String, Vec<StallFlagView>>>);

/// How the controls of one open mission reach the shell.
#[derive(Clone)]
pub struct ControlCtx {
    pub mission: MissionId,
    /// Set for a hosted mission: the control is written to its process.
    pub launch_id: Option<String>,
    pub hosted: HostedCtx,
    pub result: RwSignal<Option<String>>,
    /// The text an attached mission shows for the person to carry to the terminal.
    pub paste: RwSignal<Option<String>>,
    pub stalled: Signal<Vec<StallFlagView>>,
    /// Whether the mission can be controlled at all: a hosted run, or an attached one still live.
    pub enabled: Signal<bool>,
}

impl ControlCtx {
    pub fn act(&self, action: HumanAction, text: Option<String>) {
        let ctx = self.clone();
        if ctx.launch_id.is_none() {
            let shown = terminal_text(&action, text.as_deref().unwrap_or_default());
            ctx.paste.set(Some(shown));
        }
        spawn_local(async move {
            let args = json!({
                "mission_id": ctx.mission,
                "launch_id": ctx.launch_id,
                "action": action,
                "text": text,
            });
            let outcome = match shell::invoke::<_, ActionRecorded>("human_action", &args).await {
                Ok(done) => format!("action {} event {}", done.action_id, done.event_id),
                Err(message) => format!("refused: {message}"),
            };
            ctx.result.set(Some(outcome));
            if let Some(launch_id) = &ctx.launch_id {
                let args = json!({ "launch_id": launch_id });
                if let Ok(now) = shell::invoke::<_, HostedStatus>("host_status", &args).await {
                    ctx.hosted.status.set(Some(now));
                }
            }
        });
    }
}

/// The control bar: state, the pause and resume switch, the message field, the questions and
/// permission requests that wait, and for an attached mission the text to carry to the terminal.
#[component]
pub fn Controls(ctx: ControlCtx, bar: Signal<ControlBar>) -> impl IntoView {
    let message = RwSignal::new(String::new());
    let (pause_ctx, resume_ctx, say_ctx) = (ctx.clone(), ctx.clone(), ctx.clone());
    let paste = ctx.paste;
    let result = ctx.result;
    let questions_ctx = ctx.clone();
    let permissions_ctx = ctx;
    view! {
        <section class="controls" aria-label="controls" data-testid="control-bar">
            <div class="kpis">
                <div class="kpi">
                    <div class="v">
                        <span
                            class=move || format!("chip {}", bar.with(|b| b.state.chip()))
                            data-testid="control-state"
                        >
                            {move || bar.with(|b| b.state.label())}
                        </span>
                    </div>
                    <div class="k">"mission"</div>
                </div>
                <div class="kpi" class:bad=move || bar.with(|b| !b.questions.is_empty())>
                    <div class="v" data-testid="control-question-count">
                        {move || bar.with(|b| b.questions.len())}
                    </div>
                    <div class="k">"questions waiting"</div>
                </div>
                <div class="kpi" class:bad=move || bar.with(|b| !b.permissions.is_empty())>
                    <div class="v" data-testid="control-permission-count">
                        {move || bar.with(|b| b.permissions.len())}
                    </div>
                    <div class="k">"permission requests"</div>
                </div>
            </div>
            <div class="bar">
                <div class="switch" role="group" aria-label="pause-resume">
                    <button
                        type="button"
                        aria-label="host-pause"
                        data-testid="control-pause"
                        class:on=move || bar.with(|b| b.state == RunState::Paused)
                        disabled=move || bar.with(|b| !b.pause_enabled)
                        on:click=move |_| pause_ctx.act(HumanAction::Pause, None)
                    >
                        "Pause"
                    </button>
                    <button
                        type="button"
                        aria-label="host-resume"
                        data-testid="control-resume"
                        disabled=move || bar.with(|b| !b.resume_enabled)
                        on:click=move |_| resume_ctx.act(HumanAction::Resume, None)
                    >
                        "Resume"
                    </button>
                </div>
                <input
                    type="text"
                    aria-label="host-message-text"
                    data-testid="control-message-text"
                    prop:value=move || message.get()
                    on:input=move |ev| message.set(event_target_value(&ev))
                />
                <button
                    type="button"
                    aria-label="host-message"
                    data-testid="control-message"
                    disabled=move || bar.with(|b| !b.message_enabled)
                    on:click=move |_| {
                        let text = message.get_untracked();
                        if !text.trim().is_empty() {
                            say_ctx.act(HumanAction::message(&text), Some(text));
                            message.set(String::new());
                        }
                    }
                >
                    "Message"
                </button>
            </div>
            {move || {
                bar.with(|b| b.terminal_instruction)
                    .map(|instruction| {
                        view! {
                            <div class="paste">
                                <p class="mut" data-testid="terminal-instruction">{instruction}</p>
                                <pre class="raw" data-testid="terminal-text">
                                    {move || paste.get().unwrap_or_default()}
                                </pre>
                            </div>
                        }
                    })
            }}
            <ul class="sheet" aria-label="host-questions" data-testid="host-questions">
                {move || {
                    bar.with(|b| b.questions.clone())
                        .into_iter()
                        .map(|question| question_sheet(questions_ctx.clone(), question))
                        .collect_view()
                }}
            </ul>
            <PermissionList ctx=permissions_ctx bar=bar />
            <p class="result" data-testid="control-result">{move || result.get()}</p>
        </section>
    }
}

/// The AskUser sheet of one waiting question.
fn question_sheet(ctx: ControlCtx, question: QuestionRow) -> impl IntoView {
    let answer = RwSignal::new(String::new());
    let event_id = question.event_id.clone();
    let prompt = if question.prompts.is_empty() {
        "question".to_owned()
    } else {
        question.prompts.join(" / ")
    };
    view! {
        <li class="card" aria-label="host-question" data-testid="ask-user-sheet">
            <code class="mono small">{question.event_id.clone()}</code>
            <span class="prompt">{prompt}</span>
            <span>
                <input
                    type="text"
                    aria-label="host-answer-text"
                    data-testid="host-answer-text"
                    prop:value=move || answer.get()
                    on:input=move |ev| answer.set(event_target_value(&ev))
                />
                <button
                    type="button"
                    aria-label="host-answer"
                    data-testid="host-answer"
                    on:click=move |_| {
                        let text = answer.get_untracked();
                        if !text.trim().is_empty() {
                            ctx.act(
                                HumanAction::answer(EventId(event_id.clone()), &text),
                                Some(text),
                            );
                            answer.set(String::new());
                        }
                    }
                >
                    "Answer"
                </button>
            </span>
        </li>
    }
}

/// The permission requests that wait, one row each with Approve and Deny.
#[component]
fn PermissionList(ctx: ControlCtx, bar: Signal<ControlBar>) -> impl IntoView {
    view! {
        <div class="lib">
            <div class="scroll">
                <table aria-label="host-permissions" data-testid="host-permissions">
                    <tbody>
                        {move || {
                            bar.with(|b| b.permissions.clone())
                                .into_iter()
                                .map(|permission| {
                                    let tools = if permission.tools.is_empty() {
                                        "permission".to_owned()
                                    } else {
                                        permission.tools.join(", ")
                                    };
                                    let (approve_ctx, deny_ctx) = (ctx.clone(), ctx.clone());
                                    let (approve_event, deny_event) = (
                                        EventId(permission.event_id.clone()),
                                        EventId(permission.event_id.clone()),
                                    );
                                    view! {
                                        <tr
                                            class="row"
                                            aria-label="host-permission"
                                            data-testid="host-permission"
                                            data-request-id=permission.request_id.clone()
                                        >
                                            <td class="name">
                                                {tools} <small>{permission.request_id.clone()}</small>
                                            </td>
                                            <td>
                                                <button
                                                    type="button"
                                                    aria-label="host-approve"
                                                    data-testid="host-approve"
                                                    disabled=!permission.can_approve
                                                    on:click=move |_| {
                                                        approve_ctx
                                                            .act(
                                                                HumanAction::Approve {
                                                                    request_event: approve_event.clone(),
                                                                },
                                                                None,
                                                            );
                                                    }
                                                >
                                                    "Approve"
                                                </button>
                                                <button
                                                    type="button"
                                                    aria-label="host-deny"
                                                    data-testid="host-deny"
                                                    disabled=!permission.can_deny
                                                    on:click=move |_| {
                                                        deny_ctx
                                                            .act(
                                                                HumanAction::Deny {
                                                                    request_event: deny_event.clone(),
                                                                },
                                                                None,
                                                            );
                                                    }
                                                >
                                                    "Deny"
                                                </button>
                                            </td>
                                        </tr>
                                    }
                                })
                                .collect_view()
                        }}
                    </tbody>
                </table>
            </div>
        </div>
    }
}

/// Mark complete and mark skipped for each feature of a card.
#[component]
pub fn FeatureMarks(features: Vec<FeatureId>) -> impl IntoView {
    let Some(ctx) = use_context::<ControlCtx>() else {
        return ().into_any();
    };
    let rows = features
        .into_iter()
        .map(|feature| {
            let reason = RwSignal::new(String::new());
            let (done_ctx, skip_ctx) = (ctx.clone(), ctx.clone());
            let (done, skipped) = (feature.clone(), feature.clone());
            view! {
                <li data-testid="feature-mark" data-feature=feature.0.clone()>
                    <span>{feature.0.clone()}</span>
                    <button
                        type="button"
                        aria-label=format!("mark-complete-{}", feature.0)
                        data-testid="mark-complete"
                        on:click=move |_| {
                            done_ctx
                                .act(HumanAction::MarkComplete { feature_id: done.clone() }, None);
                        }
                    >
                        "Complete"
                    </button>
                    <input
                        type="text"
                        aria-label=format!("skip-reason-{}", feature.0)
                        data-testid="skip-reason"
                        placeholder="reason"
                        prop:value=move || reason.get()
                        on:input=move |ev| reason.set(event_target_value(&ev))
                    />
                    <button
                        type="button"
                        aria-label=format!("mark-skipped-{}", feature.0)
                        data-testid="mark-skipped"
                        on:click=move |_| {
                            let text = reason.get_untracked();
                            skip_ctx
                                .act(HumanAction::mark_skipped(skipped.clone(), &text), Some(text));
                            reason.set(String::new());
                        }
                    >
                        "Skip"
                    </button>
                </li>
            }
        })
        .collect_view();
    let enabled = ctx.enabled;
    view! {
        <Show when=move || enabled.get()>
            <ul class="feature-marks" aria-label="feature-marks">{rows.clone()}</ul>
        </Show>
    }
    .into_any()
}

/// The marker a card shows when one of its features carries a stall flag.
pub fn stall_marker(features: Vec<FeatureId>) -> AnyView {
    let Some(ctx) = use_context::<ControlCtx>() else {
        return ().into_any();
    };
    let stalled = move || {
        ctx.stalled
            .with(|flags| flags.iter().any(|flag| features.contains(&flag.feature_id)))
    };
    view! {
        <Show when=stalled>
            <span class="chip bad" data-testid="stall-marker">"stalled"</span>
        </Show>
    }
    .into_any()
}

/// The stall banner: each flagged feature with the idle minutes, the runbook prompt that
/// matches what it last did, and the runbook section.
#[component]
pub fn StallBannerView(flags: Signal<Vec<StallFlagView>>) -> impl IntoView {
    view! {
        <Show when=move || flags.with(|f| !f.is_empty())>
            <section class="stall-banner" role="alert" aria-label="stall-banner" data-testid="stall-banner">
                {move || {
                    StallBanner::of(&flags.get())
                        .rows
                        .into_iter()
                        .map(|row| {
                            view! {
                                <article class="card" data-testid="stall-row">
                                    <span class="mono">{row.feature_id.0.clone()}</span>
                                    <span class="mono mut small">
                                        "idle " {row.idle_minutes} " min"
                                    </span>
                                    <span data-testid="stall-prompt">{row.prompt.clone()}</span>
                                    <a
                                        href=row.link.clone()
                                        data-testid="stall-link"
                                        on:click=|ev| ev.prevent_default()
                                    >
                                        {row.link.clone()}
                                    </a>
                                </article>
                            }
                        })
                        .collect_view()
                }}
            </section>
        </Show>
    }
}

/// The theme switch of the kit: follow the system, or force dark or light with `data-theme`.
#[component]
pub fn ThemeSwitch() -> impl IntoView {
    let set = |theme: Option<&'static str>| {
        let root = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.document_element());
        if let Some(root) = root {
            let _ = match theme {
                Some(theme) => root.set_attribute("data-theme", theme),
                None => root.remove_attribute("data-theme"),
            };
        }
    };
    view! {
        <div class="switch theme-switch" role="group" aria-label="theme">
            <button type="button" data-testid="theme-system" on:click=move |_| set(None)>"System"</button>
            <button type="button" data-testid="theme-dark" on:click=move |_| set(Some("dark"))>"Dark"</button>
            <button type="button" data-testid="theme-light" on:click=move |_| set(Some("light"))>"Light"</button>
        </div>
    }
}
