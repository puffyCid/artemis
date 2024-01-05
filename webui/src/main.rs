mod components;
mod ui;
mod web;

use crate::ui::{about::About, endpoints::Endpoints, home::Home};
use leptos::{component, mount_to_body, view, IntoView};
use leptos_router::{Route, Router, Routes};

/// Entry point to the WebAssembly (WASM) binary
fn main() {
    mount_to_body(|| view! {<App />})
}

#[component]
/// Setup the WebAssembly Application
fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes>
                <Route path="/ui/v1/about" view=About />
                <Route path="/ui/v1/home" view=Home />
                <Route path="/ui/v1/endpoints" view=Endpoints />
            </Routes>
        </Router>
    }
}
