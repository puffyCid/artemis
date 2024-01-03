use leptos::{component, view, IntoView, Transition};

#[component]
pub(crate) fn RustInfo() -> impl IntoView {
    view! {
        <div class ="stat shadow">
            <div class="stat-title"> Artemis Version </div>
            <div class="stat-value">
                <Transition fallback=move || view!{<p> "Loading..."</p>}>
                    {move ||
                        env!("CARGO_PKG_VERSION").to_string()
                    }
                </Transition>
            </div>
        </div>
        <div class ="stat shadow">
            <div class="stat-title"> Documentation </div>
            <div class="stat-desc">
                <Transition fallback=move || view!{<p> "Loading..."</p>}>
                    {move ||
                       env!("CARGO_PKG_HOMEPAGE").to_string()
                    }
                </Transition>
            </div>
        </div>
        <div class ="stat shadow">
            <div class="stat-title"> Github Repo </div>
            <div class="stat-desc">
                <Transition fallback=move || view!{<p> "Loading..."</p>}>
                    {move ||
                       env!("CARGO_PKG_REPOSITORY").to_string()
                    }
                </Transition>
            </div>
        </div>
    }
}
