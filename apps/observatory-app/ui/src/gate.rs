//! The Gate group (FR-B3): the required checks, reviews, ruleset and attestations of the PR, the
//! `handed to CD` reading and the two clocks, drawn from `GateGroup::view`. The Merge button is
//! shown only when the view offers it; it opens a confirmation that names the PR and the target,
//! and only its Confirm sends the request (FR-G5).

use leptos::prelude::*;
use leptos::task::spawn_local;
use observatory_core::view::{GateBadge, GateGroupView, MergeOffer, MergePressed};
use serde_json::json;

use crate::controls::ControlCtx;
use crate::shell;

fn badge_chip(badge: GateBadge) -> &'static str {
    match badge {
        GateBadge::CiGreen => "run",
        GateBadge::CiRed | GateBadge::Blocked => "bad",
        GateBadge::ChecksRunning => "pause",
        GateBadge::HumanMerged | GateBadge::PolicyMerged | GateBadge::Pending => "done",
    }
}

fn conclusion_chip(conclusion: &str) -> &'static str {
    match conclusion {
        "passed" => "run",
        "failed" | "required_missing" => "bad",
        "queued" | "running" => "pause",
        _ => "done",
    }
}

/// The state of the Merge button of a board: whether its confirmation is open, whether a request
/// is out, and the text of the last answer. The board provides it once, because the lanes are
/// drawn again with every event batch and a group's own signals would be lost with them.
#[derive(Clone, Copy)]
pub struct MergeState {
    asking: RwSignal<bool>,
    pending: RwSignal<bool>,
    answer: RwSignal<Option<String>>,
}

impl MergeState {
    pub fn new() -> Self {
        Self {
            asking: RwSignal::new(false),
            pending: RwSignal::new(false),
            answer: RwSignal::new(None),
        }
    }

    /// Sends the one request of a confirmed press; the answer is the text to show.
    fn confirm(self, ctx: Option<ControlCtx>, offer: MergeOffer) {
        self.asking.set(false);
        let Some(ctx) = ctx else {
            return;
        };
        self.pending.set(true);
        spawn_local(async move {
            let args = json!({
                "mission_id": ctx.mission,
                "pr_number": offer.pr_number,
                "target": offer.target,
                "confirm": true,
            });
            let text = match shell::invoke::<_, MergePressed>("merge", &args).await {
                Ok(pressed) => pressed.message,
                Err(message) => format!(
                    "Merge of PR #{} into {} not sent: {message}",
                    offer.pr_number, offer.target
                ),
            };
            self.answer.set(Some(text));
            self.pending.set(false);
        });
    }
}

fn merge_control(offer: MergeOffer, state: MergeState, ctx: Option<ControlCtx>) -> impl IntoView {
    let confirmation = offer.confirmation();
    let button = offer.clone();
    view! {
        <div class="merge-control">
            <button
                type="button"
                class="merge"
                data-testid="merge"
                aria-label=format!("merge-pr-{}", offer.pr_number)
                disabled=move || state.pending.get()
                on:click=move |_| state.asking.set(true)
            >
                {button.label}
            </button>
            <Show when=move || state.asking.get()>
                <div class="card" role="alertdialog" aria-label="merge-confirmation" data-testid="merge-confirmation">
                    <p data-testid="merge-confirmation-text">{confirmation.text.clone()}</p>
                    <button
                        type="button"
                        class="merge"
                        data-testid="merge-confirm"
                        aria-label=format!("confirm-merge-pr-{}", offer.pr_number)
                        on:click={
                            let (ctx, offer) = (ctx.clone(), offer.clone());
                            move |_| state.confirm(ctx.clone(), offer.clone())
                        }
                    >
                        {confirmation.confirm.clone()}
                    </button>
                    <button
                        type="button"
                        data-testid="merge-cancel"
                        aria-label="cancel-merge"
                        on:click=move |_| state.asking.set(false)
                    >
                        {confirmation.cancel.clone()}
                    </button>
                </div>
            </Show>
        </div>
    }
}

/// The group of the Gate lane and of the Details; nothing until a PR exists.
#[component]
pub fn GateGroupBox(view: Signal<GateGroupView>) -> impl IntoView {
    let state = use_context::<MergeState>().unwrap_or_else(MergeState::new);
    let ctx = use_context::<ControlCtx>();
    move || {
        let group = view.get();
        let pr = group.pr_number?;
        let checks = group
            .checks
            .iter()
            .map(|check| {
                view! {
                    <li data-testid="gate-check" aria-label=format!("gate-check-{}", check.name)>
                        <span class="mono">{check.name.clone()}</span>
                        <span
                            class=format!("chip {}", conclusion_chip(&check.conclusion))
                            data-testid="gate-check-conclusion"
                        >
                            {check.conclusion.clone()}
                        </span>
                    </li>
                }
            })
            .collect_view();
        let attestations = group.attestations.join(", ");
        let merge = group
            .merge
            .clone()
            .map(|offer| merge_control(offer, state, ctx.clone()));
        let answered = group.merge.is_some().then_some(move || {
            state.answer.get().map(|text| {
                view! { <p class="result" role="status" data-testid="merge-result">{text}</p> }
            })
        });
        let handed = group.handed_to_cd.clone().map(|text| {
            view! { <p class="mono" data-testid="gate-handed-to-cd">{text}</p> }
        });
        let clocks = group
            .clocks
            .iter()
            .map(|clock| {
                view! {
                    <li data-testid="gate-clock" aria-label="gate-clock">
                        {clock.label.clone()} ": " <span class="mono">{clock.note.clone()}</span>
                    </li>
                }
            })
            .collect_view();
        Some(view! {
            <section class="gate-group" aria-label="gate-group" data-testid="gate-group">
                <header>
                    <span
                        class=format!("chip {}", badge_chip(group.badge))
                        aria-label="gate-badge"
                        data-testid="gate-badge"
                    >
                        {group.badge_text.clone()}
                    </span>
                    <span class="mono" data-testid="gate-pr">{format!("PR #{pr}")}</span>
                </header>
                <ul class="gate-checks" aria-label="gate-checks">{checks}</ul>
                <p class="mono" data-testid="gate-reviews">{group.reviews_text.clone()}</p>
                <p class="mono" data-testid="gate-ruleset">{group.ruleset_text.clone()}</p>
                <p class="mono" data-testid="gate-attestations" title=attestations>
                    {group.attestations_text.clone()}
                </p>
                {handed}
                {merge}
                {answered}
                <ul class="gate-clocks" aria-label="gate-clocks">{clocks}</ul>
            </section>
        })
    }
}
