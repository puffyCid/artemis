use crate::components::host::HostDetails;
use crate::components::host_navigation::Navigate;
use crate::components::jobs::processes::{endpoint_processes, HostProcesses};
use crate::web::server::request_server;
use common::server::heartbeat::Heartbeat;
use common::server::webui::{EndpointList, EndpointOS, EndpointRequest};
use common::system::Memory;
use leptos::logging::error;
use leptos::{
    component, create_node_ref, create_resource, create_signal, html, view, IntoView, NodeRef,
    ReadSignal, Resource, Show, SignalGet, SignalSet, SignalUpdate, Transition, WriteSignal,
};
use leptos_router::use_query_map;
use reqwest::Method;

#[component]
/// Render table of endpoints
pub(crate) fn Enrollment() -> impl IntoView {
    let headers = vec!["Platform", "Hostname", "Version", "IP", "Artemis"];
    let request = EndpointRequest {
        offset: 0,
        filter: EndpointOS::All,
        tags: Vec::new(),
        search: String::new(),
        count: 50,
    };
    let (request_get, request_set) = create_signal(request);
    let (asc_ord, set_ord) = create_signal(true);
    let info = create_resource(move || request_get.get(), request_endpoints);

    view! {
      <div class="col-span-full m-2 mb-14">
        <SearchEndpoints request_set request_get info/>
        <table class="table table-zebra border">
          // Table Header
          <thead>
            <tr>
              {headers
                  .into_iter()
                  .map(|entry| {
                      view! {
                        <Show when=move || { entry == "Hostname" }>
                          // Hostname column is sortable
                          <th
                            class="cursor-pointer"
                            on:click=move |_| {
                                sort_table(asc_ord.get(), &set_ord, &info, entry);
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
                        <Show when=move || { entry == "Platform" }>
                          <th class="dropdown">
                            <p
                              tabindex="0"
                              role="button"
                              class="flex items-center justify-between gap-2 leading-none"
                            >
                              {entry}
                            </p>
                            <ul
                              tabindex="0"
                              class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-52"
                            >
                              <li>
                                <a on:click=move |_| {
                                    filter_endpoints("Darwin", request_set, &info)
                                }>macOS</a>
                              </li>
                              <li>
                                <a on:click=move |_| {
                                    filter_endpoints("Windows", request_set, &info)
                                }>Windows</a>
                              </li>
                              <li>
                                <a on:click=move |_| {
                                    filter_endpoints("Linux", request_set, &info)
                                }>Linux</a>
                              </li>
                              <li>
                                <a on:click=move |_| {
                                    filter_endpoints("All", request_set, &info)
                                }>All</a>
                              </li>
                            </ul>
                          </th>
                        </Show>
                        <Show when=move || { entry != "Platform" && entry != "Hostname" }>
                          <th>
                            <p class="flex items-center justify-between gap-2 leading-none">
                              {entry}
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
              {move || {
                  info.get()
                      .map(|res| {
                          res.into_iter()
                              .map(|entry| {
                                  view! {
                                    <tr>
                                      <PlatformIcon platform=entry.os.clone()/>
                                      <td>
                                        <a
                                          class="link link-primary no-underline"
                                          href=format!(
                                              "/ui/v1/endpoints/info?query={}.{}",
                                              entry.os,
                                              entry.id,
                                          )
                                        >

                                          {&entry.hostname}
                                        </a>
                                      </td>
                                      <td>{&entry.version}</td>
                                      <td>{&entry.ip}</td>
                                      <td>{&entry.artemis_version}</td>
                                    </tr>
                                  }
                              })
                              .collect::<Vec<_>>()
                      })
              }}

            </Transition>
          </tbody>
        </table>
      </div>
    }
}

#[component]
/// Return the platform icon for a table row
fn PlatformIcon(platform: String) -> impl IntoView {
    if platform == "Windows" {
        let windows = String::from(
            r#"<svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8" viewBox="0 0 24 24"><path d="M3,12V6.75L9,5.43V11.91L3,12M20,3V11.75L10,11.9V5.21L20,3M3,13L9,13.09V19.9L3,18.75V13M20,13.25V22L10,20.09V13.1L20,13.25Z" /></svg>"#,
        );
        return view! { <td inner_html=windows></td> };
    } else if platform == "Darwin" {
        let macos = String::from(
            r#"<svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8" viewBox="0 0 24 24"><path d="M18.71,19.5C17.88,20.74 17,21.95 15.66,21.97C14.32,22 13.89,21.18 12.37,21.18C10.84,21.18 10.37,21.95 9.1,22C7.79,22.05 6.8,20.68 5.96,19.47C4.25,17 2.94,12.45 4.7,9.39C5.57,7.87 7.13,6.91 8.82,6.88C10.1,6.86 11.32,7.75 12.11,7.75C12.89,7.75 14.37,6.68 15.92,6.84C16.57,6.87 18.39,7.1 19.56,8.82C19.47,8.88 17.39,10.1 17.41,12.63C17.44,15.65 20.06,16.66 20.09,16.67C20.06,16.74 19.67,18.11 18.71,19.5M13,3.5C13.73,2.67 14.94,2.04 15.94,2C16.07,3.17 15.6,4.35 14.9,5.19C14.21,6.04 13.07,6.7 11.95,6.61C11.8,5.46 12.36,4.26 13,3.5Z" /></svg>"#,
        );
        return view! { <td inner_html=macos></td> };
    } else if platform == "Linux" {
        let linux = String::from(
            r#"<svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8" viewBox="0 0 24 24"><path d="M19,16C19,17.72 18.37,19.3 17.34,20.5C17.75,20.89 18,21.41 18,22H6C6,21.41 6.25,20.89 6.66,20.5C5.63,19.3 5,17.72 5,16H3C3,14.75 3.57,13.64 4.46,12.91L4.47,12.89C6,11.81 7,10 7,8V7A5,5 0 0,1 12,2A5,5 0 0,1 17,7V8C17,10 18,11.81 19.53,12.89L19.54,12.91C20.43,13.64 21,14.75 21,16H19M16,16A4,4 0 0,0 12,12A4,4 0 0,0 8,16A4,4 0 0,0 12,20A4,4 0 0,0 16,16M10,9L12,10.5L14,9L12,7.5L10,9M10,5A1,1 0 0,0 9,6A1,1 0 0,0 10,7A1,1 0 0,0 11,6A1,1 0 0,0 10,5M14,5A1,1 0 0,0 13,6A1,1 0 0,0 14,7A1,1 0 0,0 15,6A1,1 0 0,0 14,5Z" /></svg>"#,
        );
        return view! { <td inner_html=linux></td> };
    }
    let all = String::from(
        r#"<svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8" viewBox="0 0 24 24"><path d="M4,6H20V16H4M20,18A2,2 0 0,0 22,16V6C22,4.89 21.1,4 20,4H4C2.89,4 2,4.89 2,6V16A2,2 0 0,0 4,18H0V20H24V18H20Z" /></svg>"#,
    );
    view! { <td inner_html=all></td> }
}

#[component]
/// Search for specific endpoints
fn SearchEndpoints(
    request_set: WriteSignal<EndpointRequest>,
    request_get: ReadSignal<EndpointRequest>,
    info: Resource<EndpointRequest, Vec<EndpointList>>,
) -> impl IntoView {
    let counts = [20, 50, 100];
    let search_form: NodeRef<html::Input> = create_node_ref();

    let search_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let value = search_form.get().unwrap().value();
        let mut status = request_get.get();
        status.search = value;
        status.offset = 0;
        request_set.set(status);
    };

    let previous_disabled = move || request_get.get().offset <= 0;
    let next_disabled =
        move || request_get.get().count > info.get().unwrap_or_default().len() as i32;

    view! {
      <div class="grid grid-cols-4 p-2 gap-2">
        <form on:submit=search_submit>
          <label class="input input-sm input-bordered flex items-center gap-2">
            <input type="text" class="grow" node_ref=search_form placeholder="Search Endpoints"/>
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
            {move || request_get.get().count}
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
                            request_set.update(|request| request.count = count)
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
              if request_get.get().offset > 0 {
                  request_set.update(|request| request.offset -= request.count)
              }
          }
        >

          Previous
        </button>
        <button
          class="join-item btn btn-sm btn-outline"
          disabled=next_disabled
          on:click=move |_| { request_set.update(|request| request.offset += request.count) }
        >
          Next
        </button>
      </div>
    }
}

/// Get endpoints based on platform filter
fn filter_endpoints(
    platform: &str,
    request: WriteSignal<EndpointRequest>,
    info: &Resource<EndpointRequest, Vec<EndpointList>>,
) {
    let filter = if platform == "Darwin" || platform == "Macos" {
        EndpointOS::Darwin
    } else if platform == "Windows" {
        EndpointOS::Windows
    } else if platform == "Linux" {
        EndpointOS::Linux
    } else {
        EndpointOS::All
    };

    request.update(|body| body.filter = filter);
    info.refetch();
}

pub(crate) struct InfoValue {
    pub(crate) info: ReadSignal<bool>,
    pub(crate) proc: ReadSignal<bool>,
    pub(crate) set_info: WriteSignal<bool>,
    pub(crate) set_proc: WriteSignal<bool>,
}

#[component]
/// Get the details from queried endpoint
pub(crate) fn GetInfo() -> impl IntoView {
    let query = use_query_map();
    // search stored as ?q=
    let search = move || query.get().get("query").cloned().unwrap_or_default();
    let info_results = create_resource(search, endpoint_info);
    let proc_results = create_resource(search, endpoint_processes);

    let (info, set_info) = create_signal(true);
    let (proc, set_proc) = create_signal(false);

    let values = InfoValue {
        info,
        proc,
        set_info,
        set_proc,
    };
    view! {
      <Show when=move || { info.get() }>
        <Transition fallback=move || {
            view! { <p>"Loading..."</p> }
        }>
          {move || {
              info_results
                  .get()
                  .map(|res| {
                      view! { <HostDetails beat=res/> }
                  })
          }}

        </Transition>
      </Show>
      <Show when=move || {
          proc.get()
      }>
        {move || {
            proc_results
                .get()
                .map(|res| {
                    view! { <HostProcesses procs=res/> }
                })
        }}

      </Show>
      <Navigate values/>
    }
}

/// Sort the table by column. Right now only Hostname is supported
fn sort_table(
    order: bool,
    update_order: &WriteSignal<bool>,
    info: &Resource<EndpointRequest, Vec<EndpointList>>,
    column: &str,
) {
    if column == "Hostname" {
        if order {
            info.update(|endpoints| {
                endpoints
                    .as_mut()
                    .unwrap()
                    .sort_by(|a, b| a.hostname.to_lowercase().cmp(&b.hostname.to_lowercase()))
            });
            update_order.set(false);
            return;
        }
        info.update(|endpoints| {
            endpoints
                .as_mut()
                .unwrap()
                .sort_by(|a, b| b.hostname.to_lowercase().cmp(&a.hostname.to_lowercase()))
        });
        update_order.set(true);
    }
}

/// Make a request for endpoint list
async fn request_endpoints(body: EndpointRequest) -> Vec<EndpointList> {
    let list = Vec::new();
    let res_result = request_server(
        "endpoint/list",
        serde_json::to_string(&body).unwrap_or_default(),
        Method::POST,
    )
    .await;
    let response = match res_result {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to send request for endpoints: {err:?}");
            return list;
        }
    };

    let result_json = response.json().await;
    match result_json {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to get endpoint list: {err:?}");
            list
        }
    }
}

/// Get endpoint host info
async fn endpoint_info(data: String) -> Heartbeat {
    let beat = Heartbeat {
        endpoint_id: String::new(),
        heartbeat: false,
        jobs_running: 0,
        hostname: String::new(),
        timestamp: 0,
        cpu: Vec::new(),
        disks: Vec::new(),
        memory: Memory {
            available_memory: 0,
            free_memory: 0,
            free_swap: 0,
            total_memory: 0,
            total_swap: 0,
            used_memory: 0,
            used_swap: 0,
        },
        boot_time: 0,
        ip: String::from("127.0.0.1"),
        os_version: String::new(),
        uptime: 0,
        kernel_version: String::new(),
        platform: String::new(),
        artemis_version: String::from("0.9.0"),
    };
    let res_result = request_server("endpoints/info", data, Method::POST).await;
    let response = match res_result {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to send request for endpoints: {err:?}");
            return beat;
        }
    };

    let result_json = response.json().await;
    match result_json {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to get endpoint list: {err:?}");
            beat
        }
    }
}
