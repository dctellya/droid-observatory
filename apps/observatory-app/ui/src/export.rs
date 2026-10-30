use leptos::prelude::*;
use leptos::task::spawn_local;
use observatory_core::config::Config;
use observatory_core::metrics::{
    ExportFile, ExportFormat, Notes, PriceTable, ReportSetup, report_at_last_event,
};
use observatory_core::model::{CycleTemplate, Event, MissionId};
use serde_json::{Value, json};
use wasm_bindgen::{JsCast, JsValue};

use crate::ctx::PriceTableText;
use crate::shell;

/// Hands `file` to the browser as a download.
fn download(file: &ExportFile) -> Result<(), JsValue> {
    let parts = js_sys::Array::of1(&JsValue::from_str(&file.text));
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(file.mime);
    let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &options)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("no document"))?;
    let anchor: web_sys::HtmlAnchorElement = document.create_element("a")?.dyn_into()?;
    anchor.set_href(&url);
    anchor.set_download(&file.file_name);
    let body = document
        .body()
        .ok_or_else(|| JsValue::from_str("no body"))?;
    body.append_child(&anchor)?;
    anchor.click();
    body.remove_child(&anchor)?;
    web_sys::Url::revoke_object_url(&url)
}

/// The Panel's export control: CSV and JSON of the metrics rows, the text of
/// `observatory-cli metrics --csv|--json` for the mission. The browser host downloads it; the
/// window writes it at the path in the field through the shell's `export` command.
#[component]
pub fn ExportControl(
    events: ArcRwSignal<Vec<Event>>,
    name: String,
    mission: MissionId,
) -> impl IntoView {
    let template = expect_context::<RwSignal<CycleTemplate>>();
    let price_text = expect_context::<PriceTableText>();
    let tauri = shell::is_tauri();
    let status = RwSignal::new(String::new());
    let path = RwSignal::new(String::new());
    if tauri {
        let mission = mission.clone();
        spawn_local(async move {
            let args = json!({ "mission_id": mission, "format": ExportFormat::Csv });
            if let Ok(offered) = shell::invoke::<_, String>("export_default_path", &args).await {
                path.set(offered);
            }
        });
    }
    let run = Callback::new(move |format: ExportFormat| {
        if tauri {
            let target = format.with_extension(&path.get_untracked());
            path.set(target.clone());
            let args = json!({ "mission_id": mission, "format": format, "path": target });
            spawn_local(async move {
                match shell::invoke::<_, Value>("export", &args).await {
                    Ok(done) => status.set(format!(
                        "Exported {} rows to {}",
                        done["rows"],
                        done["path"].as_str().unwrap_or_default()
                    )),
                    Err(message) => status.set(format!("Export failed: {message}")),
                }
            });
            return;
        }
        let config = Config::default();
        let file = price_text
            .0
            .with(|text| PriceTable::parse(text).ok())
            .and_then(|prices| {
                template.with(|template| {
                    events.with(|events| {
                        report_at_last_event(
                            events,
                            &ReportSetup {
                                template,
                                prices: &prices,
                                notes: &Notes::default(),
                                budget: config.metrics.budget,
                                launch_budget_usd: None,
                                apportionment: config.metrics.overhead_apportionment,
                            },
                        )
                    })
                })
            })
            .map(|report| report.export(format, &name));
        status.set(match file {
            None => "Nothing to export: no event is held.".to_owned(),
            Some(file) => match download(&file) {
                Ok(()) => format!("Downloaded {} ({} rows)", file.file_name, file.rows),
                Err(err) => format!("Export failed: {err:?}"),
            },
        });
    });
    view! {
        <div class="export" role="group" aria-label="export" data-testid="export">
            {tauri
                .then(|| {
                    view! {
                        <label>
                            "Save to "
                            <input
                                type="text"
                                class="mono export-path"
                                aria-label="export-path"
                                prop:value=move || path.get()
                                on:input=move |ev| path.set(event_target_value(&ev))
                            />
                        </label>
                    }
                })}
            {ExportFormat::ALL
                .into_iter()
                .map(|format| {
                    view! {
                        <button type="button" aria-label=format.aria_label() on:click=move |_| run.run(format)>
                            {format.label()}
                        </button>
                    }
                })
                .collect_view()}
            <span class="mono" role="status" aria-label="export-status" data-testid="export-status">
                {move || status.get()}
            </span>
        </div>
    }
}
