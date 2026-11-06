//! Most między UI w WASM a komendami Tauri (host w Rust).
//!
//! Tauri v2 wystawia `window.__TAURI_INTERNALS__.invoke(cmd, args)` — z WASM
//! wchodzimy przez `js_sys` i czekamy na Promise przez `wasm_bindgen_futures`.
//! Argumenty i wyniki idą przez `serde-wasm-bindgen` (bez ręcznego JSON-a).

use js_sys::{Function, Promise, Reflect, JSON};
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

/// Wywołuje komendę Tauri. `args` musi być mapą z kluczami camelCase
/// (takich oczekuje makro `#[tauri::command]` domyślnie).
pub async fn invoke<A, R>(cmd: &str, args: &A) -> Result<R, String>
where
    A: Serialize,
    R: DeserializeOwned,
{
    let value = invoke_raw(cmd, args).await?;
    serde_wasm_bindgen::from_value(value).map_err(|e| format!("odpowiedź {cmd}: {e}"))
}

/// Woła komendę i zwraca surowy `JsValue` — dla wyników niebędących strukturą.
pub async fn invoke_raw<A: Serialize>(cmd: &str, args: &A) -> Result<JsValue, String> {
    let window = web_sys::window().ok_or_else(|| "brak window".to_string())?;

    let internals = Reflect::get(&window, &JsValue::from_str("__TAURI_INTERNALS__"))
        .map_err(|_| "brak __TAURI_INTERNALS__ (aplikacja nie działa w Tauri)".to_string())?;

    let invoke_fn = Reflect::get(&internals, &JsValue::from_str("invoke"))
        .map_err(|_| "brak invoke".to_string())?
        .dyn_into::<Function>()
        .map_err(|_| "invoke nie jest funkcją".to_string())?;

    let js_args = serde_wasm_bindgen::to_value(args).map_err(|e| format!("argumenty: {e}"))?;

    let promise = invoke_fn
        .call2(&internals, &JsValue::from_str(cmd), &js_args)
        .map_err(|e| format!("invoke {cmd}: {}", js_err(&e)))?;

    let result = JsFuture::from(Promise::from(promise))
        .await
        .map_err(|e| error_message(&e))?;

    Ok(result)
}

/// Czyta `message` z odrzuconego `Error` albo z obiektu błędu komendy
/// (`{kind, message}`) — Tauri v2 odrzuca obiema postaciami.
pub fn error_message(value: &JsValue) -> String {
    if let Some(msg) = value.as_string() {
        if !msg.is_empty() {
            return msg;
        }
    }
    if let Ok(msg) = Reflect::get(value, &JsValue::from_str("message")) {
        if let Some(s) = msg.as_string() {
            if !s.is_empty() {
                return s;
            }
        }
    }
    match JSON::stringify(value) {
        Ok(json) => json.as_string().unwrap_or_else(|| "nieznany błąd".into()),
        Err(_) => "nieznany błąd".to_string(),
    }
}

fn js_err(value: &JsValue) -> String {
    error_message(value)
}

/// Czeka `ms` milisekund — do debounce filtrów.
pub async fn sleep(ms: u32) {
    gloo_timers::future::TimeoutFuture::new(ms).await;
}

/// Nieużywane na razie — zachowane dla przyszłych zdarzeń (np. eventy progress).
#[allow(dead_code)]
pub fn closure_once(f: impl FnOnce(JsValue) + 'static) -> Closure<dyn FnMut(JsValue)> {
    let mut f = Some(f);
    Closure::new(move |v: JsValue| {
        if let Some(f) = f.take() {
            f(v);
        }
    })
}
