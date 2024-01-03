use crate::components::{
    footer::Footer,
    menu::Menu,
    rust::RustInfo,
    stats::{resources::Resources, scripts::Scripts},
};
use leptos::{component, view, IntoView};
use leptos_meta::Stylesheet;

#[component]
pub(crate) fn About() -> impl IntoView {
    view! {
        <Stylesheet id="leptos" href="/pkg/tailwind.css"/>
        <div class="grid grid-cols-3 grid-rows-4">
            <Menu />
            <Resources />
            <Scripts />
            <RustInfo />
        </div>
        <Footer />
    }
}
