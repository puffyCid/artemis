use super::enrollment::InfoValue;
use leptos::{component, view, IntoView, SignalGet, SignalUpdate};

#[component]
/// Host navigation
pub(crate) fn Navigate(values: InfoValue) -> impl IntoView {
    view! {
      <div class="btm-nav">
        // Info
        <button
          class:active=move || values.info.get()
          on:click=move |_| {
              values.set_info.update(|value| *value = true);
              values.set_proc.update(|value| *value = false);
          }
        >

          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="16"
            height="16"
            fill="currentColor"
            class="bi bi-info-circle"
            viewBox="0 0 16 16"
          >
            <path d="M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14m0 1A8 8 0 1 0 8 0a8 8 0 0 0 0 16"></path>
            <path d="m8.93 6.588-2.29.287-.082.38.45.083c.294.07.352.176.288.469l-.738 3.468c-.194.897.105 1.319.808 1.319.545 0 1.178-.252 1.465-.598l.088-.416c-.2.176-.492.246-.686.246-.275 0-.375-.193-.304-.533zM9 4.5a1 1 0 1 1-2 0 1 1 0 0 1 2 0"></path>
          </svg>
        </button>
        // Processes
        <button
          class:active=move || values.proc.get()
          on:click=move |_| {
              values.set_info.update(|value| *value = false);
              values.set_proc.update(|value| *value = true);
          }
        >

          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="16"
            height="16"
            fill="currentColor"
            class="bi bi-list-nested"
            viewBox="0 0 16 16"
          >
            <path
              fill-rule="evenodd"
              d="M4.5 11.5A.5.5 0 0 1 5 11h10a.5.5 0 0 1 0 1H5a.5.5 0 0 1-.5-.5m-2-4A.5.5 0 0 1 3 7h10a.5.5 0 0 1 0 1H3a.5.5 0 0 1-.5-.5m-2-4A.5.5 0 0 1 1 3h10a.5.5 0 0 1 0 1H1a.5.5 0 0 1-.5-.5"
            ></path>
          </svg>
        </button>
      </div>
    }
}
