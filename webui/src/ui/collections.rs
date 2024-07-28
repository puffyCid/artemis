use crate::components::{collect::CollectScripts, footer::Footer, menu::Menu};
use leptos::{component, view, IntoView};
use leptos_meta::Stylesheet;

#[component]
pub(crate) fn Collections() -> impl IntoView {
    view! {
      <Stylesheet id="leptos" href="/pkg/tailwind.css" />
      <div class="grid">
        <Menu />
        <CollectScripts />
        <Footer />
      </div>
    }
}
