use leptos::{component, view, IntoView};

#[component]
/// Scripts page
pub(crate) fn Scripts() -> impl IntoView {
    view! {
      <div class="stat shadow">
        <div class="stat-title text-zinc-600">Server Scripts</div>
        <div class="stat-value">0</div>
      </div>
    }
}
