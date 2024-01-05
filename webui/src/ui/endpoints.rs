use crate::components::{enrollment::Enrollment, footer::Footer, menu::Menu};
use leptos::{component, view, IntoView};
use leptos_meta::Stylesheet;

#[component]
pub(crate) fn Endpoints() -> impl IntoView {
    view! {
        <Stylesheet id="leptos" href="/pkg/tailwind.css"/>
          <div>
            <Menu />
            <Enrollment />
          </div>
        <Footer />
    }
}
