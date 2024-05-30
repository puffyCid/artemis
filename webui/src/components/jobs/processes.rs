use crate::web::{server::request_server, time::unixepoch_to_rfc};
use common::{server::jobs::ProcessJob, system::Processes};
use leptos::{
    component, create_node_ref, create_resource, create_signal, html, logging::error, view,
    IntoView, NodeRef, ReadSignal, Resource, Show, SignalGet, SignalSet, SignalUpdate, Transition,
    WriteSignal,
};
use reqwest::Method;

#[derive(Debug, Clone, PartialEq)]
struct EndpointProcesses {
    procs: ProcessJob,
    count: i32,
    filter: String,
    offset: i32,
}

#[component]
/// Display process listing info
pub(crate) fn EndpointProcesses(procs: Option<ProcessJob>) -> impl IntoView {
    if procs.is_none() {
        return view! { <div>"No processes"</div> };
    }
    let proc_data = procs.unwrap();
    let headers = vec!["Path", "Name", "PID", "PPID", "Start Time"];

    let endpoint_procs = EndpointProcesses {
        procs: proc_data,
        count: 50,
        filter: String::new(),
        offset: 0,
    };

    let (proc_get, proc_set) = create_signal(endpoint_procs);
    let info = create_resource(move || proc_get.get(), list_processes);
    let (asc_ord, set_ord) = create_signal(true);

    view! {
      <div class="col-span-full m-2 mb-14">
        <SearchProcesses proc_set proc_get info/>
        <table class="table table-zebra border">
          // Table Header
          <thead>
            <tr>
              {headers
                  .into_iter()
                  .map(|entry| {
                      view! {
                        <Show
                          when=move || {
                              entry == "Path" || entry == "Name" || entry == "Start Time"
                          }
                          // Non-sortable columns
                          fallback=move || {
                              view! {
                                <th>
                                  <p class="flex items-center justify-between gap-2 leading-none">
                                    {entry}
                                  </p>
                                </th>
                              }
                          }
                        >

                          // Columns that are sortable
                          <th
                            class="cursor-pointer"
                            on:click=move |_| {
                                sort_table(asc_ord.get(), &set_ord, &proc_set, entry);
                            }
                          >

                            <p class="flex items-center justify-between gap-2 leading-none">
                              {entry}
                              <svg
                                xmlns="http://www.w3.org/2000/svg"
                                fill="none"
                                viewBox="0 0 24 24"
                                stroke-width="2"
                                stroke="currentColor"
                                aria-hidden="true"
                                class="w-4 h-4"
                              >
                                <path
                                  stroke-linecap="round"
                                  stroke-linejoin="round"
                                  d="M8.25 15L12 18.75 15.75 15m-7.5-6L12 5.25 15.75 9"
                                ></path>
                              </svg>
                            </p>
                          </th>
                        </Show>
                      }
                  })
                  .collect::<Vec<_>>()}
            </tr>
          </thead>
          // Table Rows
          <tbody>
            <Transition fallback=move || {
                view! {
                  <tr>
                    <th>Loading...</th>
                  </tr>
                }
            }>
              {info
                  .get()
                  .map(|res| {
                      res.into_iter()
                          .map(|entry| {
                              view! {
                                <tr>
                                  <td>{entry.full_path}</td>
                                  <td>{entry.name}</td>
                                  <td>{entry.pid}</td>
                                  <td>{entry.ppid}</td>
                                  <td>{unixepoch_to_rfc(entry.start_time as i64)}</td>
                                </tr>
                              }
                          })
                          .collect::<Vec<_>>()
                  })}

            </Transition>
          </tbody>
        </table>
      </div>
    }
}

#[component]
/// Search through process data
fn SearchProcesses(
    proc_set: WriteSignal<EndpointProcesses>,
    proc_get: ReadSignal<EndpointProcesses>,
    info: Resource<EndpointProcesses, Vec<Processes>>,
) -> impl IntoView {
    let counts = vec![20, 50, 100];
    let search_form: NodeRef<html::Input> = create_node_ref();

    let search_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let value = search_form.get().unwrap().value();
        let mut status = proc_get.get();
        status.filter = value;
        status.offset = 0;
        proc_set.set(status);
    };

    let previous_disabled = move || proc_get.get().offset <= 0;
    let next_disabled = move || proc_get.get().count > info.get().unwrap_or_default().len() as i32;

    view! {
      <div class="grid grid-cols-4 p-2 gap-2">
        <form on:submit=search_submit>
          <label class="input input-sm input-bordered flex items-center gap-2">
            <input type="text" class="grow" node_ref=search_form placeholder="Search Processes"/>
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 16 16"
              fill="currentColor"
              class="w-4 h-4 opacity-70"
            >
              <path
                fill-rule="evenodd"
                d="M9.965 11.026a5 5 0 1 1 1.06-1.06l2.755 2.754a.75.75 0 1 1-1.06 1.06l-2.755-2.754ZM10.5 7a3.5 3.5 0 1 1-7 0 3.5 3.5 0 0 1 7 0Z"
                clip-rule="evenodd"
              ></path>
            </svg>
          </label>
        </form>
        <div class="dropdown">
          <div tabindex="0" role="button" class="btn btn-sm">
            "Limit: "
            {move || proc_get.get().count}
          </div>
          <ul
            tabindex="0"
            class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-52"
          >
            {counts
                .into_iter()
                .map(|count| {
                    view! {
                      <li>
                        <a on:click=move |_| {
                            proc_set.update(|request| request.count = count)
                        }>{count}</a>
                      </li>
                    }
                })
                .collect::<Vec<_>>()}
          </ul>
        </div>
        <button
          class="join-item btn btn-sm btn-outline"
          disabled=previous_disabled
          on:click=move |_| {
              if proc_get.get().offset > 0 {
                  proc_set.update(|request| request.offset -= request.count)
              }
          }
        >

          Previous
        </button>
        <button
          class="join-item btn btn-sm btn-outline"
          disabled=next_disabled
          on:click=move |_| { proc_set.update(|request| request.offset += request.count) }
        >
          Next
        </button>
      </div>
    }
}

/// Sort the table by columns
fn sort_table(
    order: bool,
    update_order: &WriteSignal<bool>,
    info: &WriteSignal<EndpointProcesses>,
    column: &str,
) {
    if column == "Path" {
        if order {
            info.update(|endpoints| {
                endpoints
                    .procs
                    .data
                    .sort_by(|a, b| a.full_path.to_lowercase().cmp(&b.full_path.to_lowercase()))
            });
            update_order.set(false);
            return;
        }
        info.update(|endpoints| {
            endpoints
                .procs
                .data
                .sort_by(|a, b| b.full_path.to_lowercase().cmp(&a.full_path.to_lowercase()))
        });
        update_order.set(true);
    } else if column == "Name" {
        if order {
            info.update(|endpoints| {
                endpoints
                    .procs
                    .data
                    .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
            update_order.set(false);
            return;
        }
        info.update(|endpoints| {
            endpoints
                .procs
                .data
                .sort_by(|a, b| b.name.to_lowercase().cmp(&a.name.to_lowercase()))
        });
        update_order.set(true);
    } else if column == "Start Time" {
        if order {
            info.update(|endpoints| {
                endpoints
                    .procs
                    .data
                    .sort_by(|a, b| a.start_time.cmp(&b.start_time))
            });
            update_order.set(false);

            return;
        }
        info.update(|endpoints| {
            endpoints
                .procs
                .data
                .sort_by(|a, b| b.start_time.cmp(&a.start_time))
        });
        update_order.set(true);
    }
}

/// Get processes associated with endpoint
pub(crate) async fn endpoint_processes(data: String) -> Option<ProcessJob> {
    let res_result = request_server("endpoints/processes", data, Method::POST).await;
    let response = match res_result {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to send request for process list: {err:?}");
            return None;
        }
    };

    let result_json = response.json().await;
    match result_json {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to get process list: {err:?}");
            None
        }
    }
}

/// List processes for view
async fn list_processes(procs: EndpointProcesses) -> Vec<Processes> {
    let mut data = Vec::new();

    for (key, value) in procs.procs.data.into_iter().enumerate() {
        if procs.offset > key as i32 {
            continue;
        }

        if data.len() as i32 == procs.count {
            break;
        }

        if procs.filter.is_empty() {
            data.push(value);
            continue;
        }

        if !serde_json::to_string(&value)
            .unwrap_or_default()
            .contains(&procs.filter)
        {
            continue;
        }

        data.push(value);
    }

    data
}
