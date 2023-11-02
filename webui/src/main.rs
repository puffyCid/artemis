mod components;
use crate::components::{
    footer::Footer,
    menu::Menu,
    stats::endpoints::{EndpointOS, Stats},
};
use leptos::{component, mount_to_body, view, IntoView};
use leptos_meta::Stylesheet;

/// Entry point to the WebAssembly (WASM) binary
fn main() {
    mount_to_body(|| view! {<App />})
}

#[component]
/// Setup the WebAssembly Application
fn App() -> impl IntoView {
    view! {
    <Stylesheet id="leptos" href="/pkg/tailwind.css"/>
    <div class="grid grid-cols-4 grid-rows-4">
        <Menu />
        <div class="col-span-1"><Stats os=EndpointOS::All /></div>
        <div class="col-span-1"><Stats os=EndpointOS::Linux /></div>
        <div class="col-span-1"><Stats os=EndpointOS::MacOS /></div>
        <div class="col-span-1"><Stats os=EndpointOS::Windows /></div>
    </div>
    <Footer />
    }
}
