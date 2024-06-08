use leptos::{component, view, IntoView, Transition};

#[component]
/// Rust page
pub(crate) fn RustInfo() -> impl IntoView {
    view! {
      <div class="stat shadow">
        <div class="stat-title text-zinc-600">Artemis Version</div>
        <div class="stat-value">
          <Transition fallback=move || {
              view! { <p>"Loading..."</p> }
          }>{move || env!("CARGO_PKG_VERSION").to_string()}</Transition>
        </div>
      </div>
    }
}
