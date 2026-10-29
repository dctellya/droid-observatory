//! The Gate group (FR-B3): the required checks, reviews, ruleset and attestations of the PR, the
//! `handed to CD` reading and the two clocks, drawn from `GateGroup::view`. The Merge button is
//! shown only when the view offers it.

use leptos::prelude::*;
use observatory_core::view::{GateBadge, GateGroupView};

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

/// The group of the Gate lane and of the Details; nothing until a PR exists.
#[component]
pub fn GateGroupBox(view: Signal<GateGroupView>) -> impl IntoView {
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
        let merge = group.merge.as_ref().map(|offer| {
            view! {
                <button
                    type="button"
                    class="merge"
                    data-testid="merge"
                    aria-label=format!("merge-pr-{}", offer.pr_number)
                >
                    {offer.label.clone()}
                </button>
            }
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
                <ul class="gate-clocks" aria-label="gate-clocks">{clocks}</ul>
            </section>
        })
    }
}
