use leptos::prelude::*;
use observatory_core::view::{FeatureTimeline, PanelMetrics, WasteBar, WasteChart};

const NO_METRICS: &str = "No metrics yet";

const WASTE_SLOT_W: f64 = 80.0;
const WASTE_BAR_FRACTION: f64 = 0.6;
const WASTE_PLOT_H: f64 = 160.0;
const WASTE_LABEL_H: f64 = 24.0;
const WASTE_LABEL_BASELINE: f64 = 16.0;

const TIMELINE_LABEL_W: f64 = 140.0;
const TIMELINE_PLOT_W: f64 = 900.0;
const TIMELINE_ROW_H: f64 = 22.0;
const TIMELINE_BAND_PAD: f64 = 6.0;
const TIMELINE_OUTLINE_PAD: f64 = 2.0;
const TIMELINE_LABEL_BASELINE: f64 = 15.0;
const TIMELINE_MIN_BAND_W: f64 = 0.5;

const SPARK_W: f64 = 240.0;
const SPARK_H: f64 = 48.0;

const TIMELINE_LEGEND: [(&str, &str); 6] = [
    ("active", "Active"),
    ("waiting-on-human", "Waiting on human"),
    ("waiting-on-validation", "Waiting on validation"),
    ("rework", "Rework"),
    ("retry", "Retry"),
    ("idle", "Idle"),
];

fn no_metrics() -> AnyView {
    view! { <p class="muted">{NO_METRICS}</p> }.into_any()
}

fn waste_bar(index: usize, bar: &WasteBar) -> AnyView {
    let slot_x = index as f64 * WASTE_SLOT_W;
    let bar_w = WASTE_SLOT_W * WASTE_BAR_FRACTION;
    let x = slot_x + (WASTE_SLOT_W - bar_w) / 2.0;
    let segments = bar
        .segments
        .iter()
        .map(|seg| {
            let y = WASTE_PLOT_H * (1.0 - (seg.y + seg.height));
            let height = WASTE_PLOT_H * seg.height;
            let class = format!("seg-{}", seg.class);
            let tip = format!("{}: {} s", seg.label, seg.seconds);
            view! {
                <rect
                    class=class
                    data-group=seg.label
                    data-seconds=seg.seconds.to_string()
                    x=x.to_string()
                    y=y.to_string()
                    width=bar_w.to_string()
                    height=height.to_string()
                >
                    <title>{tip}</title>
                </rect>
            }
        })
        .collect_view();
    let label_x = slot_x + WASTE_SLOT_W / 2.0;
    let label_y = WASTE_PLOT_H + WASTE_LABEL_BASELINE;
    view! {
        <g aria-label="waste-bar" data-bar=bar.label.clone()>
            {segments}
            <text x=label_x.to_string() y=label_y.to_string() text-anchor="middle">
                {bar.label.clone()}
            </text>
        </g>
    }
    .into_any()
}

fn waste_view(waste: &WasteChart) -> AnyView {
    let width = waste.bars.len().max(1) as f64 * WASTE_SLOT_W;
    let height = WASTE_PLOT_H + WASTE_LABEL_H;
    let view_box = format!("0 0 {width} {height}");
    let bars = waste
        .bars
        .iter()
        .enumerate()
        .map(|(index, bar)| waste_bar(index, bar))
        .collect_view();
    let legend = waste
        .legend
        .iter()
        .map(|label| {
            let class = waste
                .groups
                .iter()
                .find(|group| group.label == *label)
                .map_or_else(String::new, |group| group.class.to_owned());
            let swatch = format!("swatch seg-{class}");
            view! {
                <li aria-label="waste-legend-item">
                    <span class=swatch></span>
                    {*label}
                </li>
            }
        })
        .collect_view();
    let rows = waste
        .groups
        .iter()
        .map(|group| {
            view! {
                <tr aria-label="waste-group">
                    <td>{group.label}</td>
                    <td class="n">{group.seconds}</td>
                    <td class="n">{group.tokens.input}</td>
                    <td class="n">{group.tokens.output}</td>
                    <td class="n">{group.tokens.cache_read}</td>
                </tr>
            }
        })
        .collect_view();
    view! {
        <div class="chart-block">
            <svg
                class="chart"
                viewBox=view_box
                width=width.to_string()
                height=height.to_string()
                role="img"
                aria-label="panel-chart-waste"
                data-testid="panel-chart-waste"
            >
                <title>"Waste by group: the run and each milestone"</title>
                {bars}
            </svg>
            <ul class="chart-legend" aria-label="waste-legend" data-testid="waste-legend">
                {legend}
            </ul>
            <div class="lib"><div class="scroll">
            <table aria-label="waste-groups">
                <thead>
                    <tr>
                        <th>"Group"</th>
                        <th>"Seconds"</th>
                        <th colspan="3">"Tokens burnt in waste"</th>
                    </tr>
                    <tr>
                        <th></th>
                        <th></th>
                        <th>"Input"</th>
                        <th>"Output"</th>
                        <th>"Cache read"</th>
                    </tr>
                </thead>
                <tbody>{rows}</tbody>
            </table>
            </div></div>
        </div>
    }
    .into_any()
}

#[component]
pub fn WasteBarsChart(metrics: Memo<Option<PanelMetrics>>) -> impl IntoView {
    view! {
        {move || {
            metrics.with(|m| m.as_ref().map_or_else(no_metrics, |m| waste_view(&m.waste)))
        }}
    }
}

fn timeline_row(index: usize, row: &FeatureTimeline) -> AnyView {
    let top = index as f64 * TIMELINE_ROW_H;
    let bands = row
        .bands
        .iter()
        .map(|band| {
            let x = TIMELINE_LABEL_W + band.x * TIMELINE_PLOT_W;
            let width = (band.width * TIMELINE_PLOT_W).max(TIMELINE_MIN_BAND_W);
            let tip = format!(
                "{} {} to {}",
                band.state,
                band.from.to_rfc3339_millis(),
                band.to.to_rfc3339_millis()
            );
            view! {
                <rect
                    class=format!("state-{}", band.class)
                    data-state=band.state
                    x=x.to_string()
                    y=(top + TIMELINE_BAND_PAD).to_string()
                    width=width.to_string()
                    height=(TIMELINE_ROW_H - 2.0 * TIMELINE_BAND_PAD).to_string()
                >
                    <title>{tip}</title>
                </rect>
            }
        })
        .collect_view();
    let rework = row
        .rework
        .iter()
        .map(|window| {
            let x = TIMELINE_LABEL_W + window.x * TIMELINE_PLOT_W;
            let width = (window.width * TIMELINE_PLOT_W).max(TIMELINE_MIN_BAND_W);
            let class = if window.unresolved {
                "rework-window unresolved"
            } else {
                "rework-window"
            };
            let tip = format!(
                "rework window {} to {}{}",
                window.from.to_rfc3339_millis(),
                window.to.to_rfc3339_millis(),
                if window.unresolved {
                    " (unresolved)"
                } else {
                    ""
                }
            );
            view! {
                <rect
                    class=class
                    x=x.to_string()
                    y=(top + TIMELINE_OUTLINE_PAD).to_string()
                    width=width.to_string()
                    height=(TIMELINE_ROW_H - 2.0 * TIMELINE_OUTLINE_PAD).to_string()
                >
                    <title>{tip}</title>
                </rect>
            }
        })
        .collect_view();
    view! {
        <g aria-label="panel-timeline-row" data-feature=row.feature.0.clone()>
            <text class="mono" x="0" y=(top + TIMELINE_LABEL_BASELINE).to_string()>
                {row.feature.0.clone()}
            </text>
            {bands}
            {rework}
        </g>
    }
    .into_any()
}

fn timeline_view(metrics: &PanelMetrics) -> AnyView {
    let rows = &metrics.timeline.rows;
    let width = TIMELINE_LABEL_W + TIMELINE_PLOT_W;
    let height = rows.len() as f64 * TIMELINE_ROW_H;
    let view_box = format!("0 0 {width} {height}");
    let body = rows
        .iter()
        .enumerate()
        .map(|(index, row)| timeline_row(index, row))
        .collect_view();
    let legend = TIMELINE_LEGEND
        .iter()
        .map(|(class, label)| {
            let swatch = format!("swatch state-{class}");
            view! {
                <li aria-label="timeline-legend-item">
                    <span class=swatch></span>
                    {*label}
                </li>
            }
        })
        .collect_view();
    view! {
        <div class="chart-block">
            <svg
                class="chart"
                viewBox=view_box
                width=width.to_string()
                height=height.to_string()
                role="img"
                aria-label="panel-chart-timeline"
                data-testid="panel-chart-timeline"
            >
                <title>"Time states and rework windows per feature"</title>
                {body}
            </svg>
            <ul class="chart-legend" aria-label="timeline-legend" data-testid="timeline-legend">
                {legend}
                <li aria-label="timeline-legend-note">"Rework windows are drawn as outlines"</li>
            </ul>
        </div>
    }
    .into_any()
}

#[component]
pub fn TimelineChart(metrics: Memo<Option<PanelMetrics>>) -> impl IntoView {
    view! {
        {move || metrics.with(|m| m.as_ref().map_or_else(no_metrics, timeline_view))}
    }
}

fn sparkline_view(metrics: &PanelMetrics) -> AnyView {
    let spark = &metrics.sparkline;
    let d = spark.path(SPARK_W, SPARK_H);
    let view_box = format!("0 0 {SPARK_W} {SPARK_H}");
    let total = format!(
        "Cumulative cost: {:.2} {} (price table {})",
        spark.total, spark.currency, spark.price_table
    );
    view! {
        <div class="chart-block">
            <svg
                class="chart"
                viewBox=view_box
                width=SPARK_W.to_string()
                height=SPARK_H.to_string()
                role="img"
                aria-label="panel-chart-sparkline"
                data-testid="panel-chart-sparkline"
            >
                <title>"Cumulative cost over the run"</title>
                <path aria-label="sparkline-path" d=d class="spark"></path>
            </svg>
            <p aria-label="sparkline-total">{total}</p>
        </div>
    }
    .into_any()
}

#[component]
pub fn CostSparkline(metrics: Memo<Option<PanelMetrics>>) -> impl IntoView {
    view! {
        {move || metrics.with(|m| m.as_ref().map_or_else(no_metrics, sparkline_view))}
    }
}
