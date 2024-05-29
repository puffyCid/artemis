use leptos::{component, view, IntoView};

#[component]
/// Menu page
pub(crate) fn Menu() -> impl IntoView {
    view! {
      <div class="col-span-full">
        <div class="navbar bg-neutral text-neutral-content">
          <div class="navbar-start"></div>
          <div class="navbar-center hidden lg:flex">
            <ul class="menu menu-horizontal px-1">
              <li>
                <a href="/ui/v1/home">Home</a>
              </li>
              <li>
                <a href="/ui/v1/endpoints">Endpoints</a>
              </li>
              <li>
                <a>Collections</a>
              </li>
              <li>
                <a>Files</a>
              </li>
              <li>
                <a href="/ui/v1/about">About</a>
              </li>
            </ul>
          </div>
          <div class="navbar-end">
            <div tabindex="0" class="avatar">
              <div class="w24 rounded-full">
                <img src="https://gravatar.com/avatar/76e90b779ff39910179f1c39b80c4025716c8030e054a257dc7dde83ea1fc691"/>
              </div>
            </div>
          </div>
        </div>
      </div>
    }
}
