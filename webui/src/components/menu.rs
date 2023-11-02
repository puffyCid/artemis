use leptos::{component, view, IntoView};

#[component]
pub(crate) fn Menu() -> impl IntoView {
    view! {
        <div class="col-span-full">
            <div class="navbar bg-neutral text-neutral-content">
                <div class="navbar-start">
                    icon
                </div>
                <div class="navbar-center hidden lg:flex">
                    <ul class="menu menu-horizontal px-1">
                        <li><a>Endpoints</a></li>
                        <li><a>Collections</a></li>
                        <li><a>Files</a></li>
                    </ul>
                </div>
                <div class="navbar-end">
                    <label tabindex="0" class="btn btn-primary btn-circle avatar">
                        Cat
                    </label>
                </div>
            </div>
        </div>
    }
}
