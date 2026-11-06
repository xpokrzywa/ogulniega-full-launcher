//! Spike: Leptos (CSR) w WebView Tauri — weryfikacja toolchainu przed
//! przepisaniem pełnego UI.

use leptos::prelude::*;

#[component]
fn App() -> impl IntoView {
    view! {
        <main class="spike">
            <h1>"Ogulniega Full Launcher"</h1>
            <p>"UI w czystym Rust (Leptos → WASM)."</p>
        </main>
    }
}

fn main() {
    mount_to_body(|| view! { <App/> });
}
