use leptos::{component, view, IntoView};

#[component]
pub(crate) fn Footer() -> impl IntoView {
    view! {
        <div class="flex flex-col">
        <section class="flex-grow h-screen"></section>
            <footer class="footer items-center p-4 bg-neutral text-neutral-content">
                <aside class="items-center grid-flow-col">
                    <p>MIT - Copyright (c) 2023 puffyCid</p>
                </aside>
                <nav class="grid-flow-col gap-4 md:place-self-center md:justify-self-end">
                    <a>Docs</a>
                    <a>GitHub</a>
                </nav>
            </footer>
        </div>
    }
}
