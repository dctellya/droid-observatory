//! `TauriBackend` (architecture §7): commands and events of the shell through
//! `window.__TAURI__` (`app.withGlobalTauri`). Payloads travel as JSON text, so the types are the
//! core serde types and no binding crate is needed. In the browser host there is no
//! `__TAURI__`: `is_tauri` is false and the UI shows attach as not available.

use js_sys::{Function, Promise, Reflect};
use serde::Serialize;
use serde::de::DeserializeOwned;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use wasm_bindgen_futures::JsFuture;

fn tauri() -> Option<JsValue> {
    let window = web_sys::window()?;
    Reflect::get(&window, &"__TAURI__".into())
        .ok()
        .filter(|value| !value.is_undefined() && !value.is_null())
}

/// True inside the Tauri window.
pub fn is_tauri() -> bool {
    tauri().is_some()
}

fn member(of: &JsValue, name: &str) -> Result<JsValue, String> {
    Reflect::get(of, &name.into())
        .ok()
        .filter(|value| !value.is_undefined())
        .ok_or_else(|| format!("window.__TAURI__ has no {name}"))
}

fn from_js<T: DeserializeOwned>(value: &JsValue) -> Result<T, String> {
    let text = if value.is_undefined() {
        "null".to_owned()
    } else {
        js_sys::JSON::stringify(value)
            .map_err(|err| format!("{err:?}"))?
            .as_string()
            .unwrap_or_else(|| "null".to_owned())
    };
    serde_json::from_str(&text).map_err(|err| err.to_string())
}

fn reason(error: JsValue) -> String {
    error
        .as_string()
        .or_else(|| {
            Reflect::get(&error, &"message".into())
                .ok()
                .and_then(|message| message.as_string())
        })
        .unwrap_or_else(|| format!("{error:?}"))
}

/// Calls the shell command `command` with `args` (snake_case keys, as the command declares).
pub async fn invoke<A: Serialize, T: DeserializeOwned>(
    command: &str,
    args: &A,
) -> Result<T, String> {
    let tauri = tauri().ok_or("not available in the browser host")?;
    let core = member(&tauri, "core")?;
    let invoke: Function = member(&core, "invoke")?.into();
    let text = serde_json::to_string(args).map_err(|err| err.to_string())?;
    let args = js_sys::JSON::parse(&text).map_err(reason)?;
    let promise: Promise = invoke
        .call2(&core, &command.into(), &args)
        .map_err(reason)?
        .into();
    let value = JsFuture::from(promise).await.map_err(reason)?;
    from_js(&value)
}

/// Calls `handler` with the payload of every `event` the shell emits, for the life of the page.
pub fn listen<T: DeserializeOwned + 'static>(event: &'static str, handler: impl Fn(T) + 'static) {
    let register = || -> Result<(), String> {
        let tauri = tauri().ok_or("not available in the browser host")?;
        let events = member(&tauri, "event")?;
        let listen: Function = member(&events, "listen")?.into();
        let callback = Closure::<dyn Fn(JsValue)>::new(move |envelope: JsValue| {
            let payload = Reflect::get(&envelope, &"payload".into()).unwrap_or(JsValue::NULL);
            match from_js::<T>(&payload) {
                Ok(value) => handler(value),
                Err(err) => web_sys::console::warn_1(&format!("{event}: {err}").into()),
            }
        });
        listen
            .call2(&events, &event.into(), callback.as_ref().unchecked_ref())
            .map_err(reason)?;
        callback.forget();
        Ok(())
    };
    if let Err(err) = register() {
        web_sys::console::warn_1(&format!("cannot listen to {event}: {err}").into());
    }
}
