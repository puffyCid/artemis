use leptos::{component, view, IntoView};

#[component]
pub(crate) fn Scripts() -> impl IntoView {
    view! {
        <div class ="stat shadow">
            <div class="stat-title"> Server Scripts </div>
            <div class="stat-value">0</div>
        </div>
    }
}
