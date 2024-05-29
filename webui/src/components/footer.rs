use leptos::{component, view, IntoView};

#[component]
/// Foooter page
pub(crate) fn Footer() -> impl IntoView {
    view! {
      <div class="col-span-full fixed bottom-0 min-w-full">
        <section class=""></section>
        <footer class="footer items-center p-4 bg-neutral text-neutral-content">
          <aside class="items-center grid-flow-col">
            <p>MIT - Copyright (c) 2023 puffyCid</p>
          </aside>
          <nav class="grid-flow-col gap-4 md:place-self-center md:justify-self-end">
            <a
              href="https://puffycid.github.io/artemis-api"
              target="_blank"
              rel="noopener noreferrer"
            >
              Docs
            </a>
            <a href="https://github.com/puffycid/artemis" target="_blank" rel="noopener noreferrer">
              GitHub
            </a>
          </nav>
        </footer>
      </div>
    }
}
