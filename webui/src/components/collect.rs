use crate::web::server::request_server;
use crate::web::time::unixepoch_to_rfc;
use common::server::collections::{CollectionInfo, CollectionRequest, CollectionTargets};
use common::server::webui::CollectRequest;
use leptos::logging::error;
use leptos::{
    component, create_node_ref, create_resource, create_signal, html, view, IntoView, NodeRef,
    ReadSignal, Resource, Show, SignalGet, SignalSet, SignalUpdate, Transition, WriteSignal,
};
use reqwest::Method;

#[component]
/// Gather collections launched by artemis
pub(crate) fn CollectScripts() -> impl IntoView {
    let headers = vec![
        "ID",
        "Name",
        "Created",
        "Start Time",
        "Systems Remaining",
        "Systems Completed",
    ];

    let request = CollectRequest {
        offset: 0,
        tags: Vec::new(),
        search: String::new(),
        count: 50,
    };

    let (request_get, request_set) = create_signal(request);
    let (asc_ord, set_ord) = create_signal(true);
    let info = create_resource(move || request_get.get(), request_collections);

    view! {
      <div class="col-span-full m-2 mb-14">
        <SearchCollections request_set request_get info />
        <table class="table border">
          // Table Header
          <thead>
            <tr>
              {headers
                  .into_iter()
                  .map(|entry| {
                      view! {
                        <Show when=move || { entry == "ID" || entry == "Name" }>
                          // ID or Name column is sortable
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
                        <Show when=move || { entry != "ID" && entry != "Name" }>
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
                                  let (view_status, set_status) = create_signal(true);
                                  let mut all_targets = Vec::from_iter(entry.targets.clone());
                                  all_targets
                                      .append(&mut Vec::from_iter(entry.targets_completed.clone()));
                                  let info_request = CollectionTargets {
                                      offset: 0,
                                      limit: 50,
                                      targets: all_targets,
                                      id: entry.info.id,
                                  };
                                  let (info_get, _info_set) = create_signal(info_request);
                                  let collect_info = create_resource(
                                      move || info_get.get(),
                                      request_endpoints_collection,
                                  );
                                  view! {
                                    <tr
                                      class="cursor-pointer"
                                      on:click=move |_| set_status.set(!view_status.get())
                                    >
                                      <td>{entry.info.id}</td>
                                      <td>{&entry.info.name}</td>
                                      <td>{unixepoch_to_rfc(entry.info.created as i64)}</td>
                                      <td>{unixepoch_to_rfc(entry.info.start_time as i64)}</td>
                                      <td>{entry.targets.len()}</td>
                                      <td>{entry.targets_completed.len()}</td>
                                    </tr>
                                    <CollectionDetails view_status info=collect_info />
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
/// View the collection details per endpoint
fn CollectionDetails(
    view_status: ReadSignal<bool>,
    info: Resource<CollectionTargets, Vec<CollectionInfo>>,
) -> impl IntoView {
    let headers = vec![
        "Hostname",
        "Endpoint ID",
        "Started Time",
        "Completed Time",
        "Status",
    ];

    view! {
      <tr class:hidden=move || view_status.get()>
        <td colspan="5">
          <div class="overflow-x">
            <table class="table table-zebra border m-2 w-full overflow-scroll">
              <thead>
                <tr>
                  {headers
                      .into_iter()
                      .map(|entry| {
                          view! {
                            <th>
                              <p class="flex items-center justify-between gap-2 leading-none">
                                {entry}
                              </p>
                            </th>
                          }
                      })
                      .collect::<Vec<_>>()}
                </tr>
              </thead>
              <tbody>
                {move || {
                    info.get()
                        .map(|res| {
                            res.into_iter()
                                .map(|entry| {
                                    view! {
                                      <tr>
                                        <td>{entry.hostname}</td>
                                        <td>{entry.endpoint_id}</td>
                                        <td>{unixepoch_to_rfc(entry.started as i64)}</td>
                                        <td>{unixepoch_to_rfc(entry.completed as i64)}</td>
                                        <td>{format!("{:?}", entry.status)}</td>
                                      </tr>
                                    }
                                })
                                .collect::<Vec<_>>()
                        })
                }}
              </tbody>
            </table>
          </div>
        </td>
      </tr>
    }
}

#[component]
/// Search for specific collections
fn SearchCollections(
    request_set: WriteSignal<CollectRequest>,
    request_get: ReadSignal<CollectRequest>,
    info: Resource<CollectRequest, Vec<CollectionRequest>>,
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
      <div class="grid grid-cols-5 p-2 gap-2">
        <form on:submit=search_submit>
          <label class="input input-sm input-bordered flex items-center gap-2">
            <input type="text" class="grow" node_ref=search_form placeholder="Search Collections" />
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
        <button class="btn btn-sm btn-outline btn-primary">
          "New Collection"
          <svg
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            stroke-width="1.5"
            stroke="currentColor"
            class="size-6"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0 3.181 3.183a8.25 8.25 0 0 0 13.803-3.7M4.031 9.865a8.25 8.25 0 0 1 13.803-3.7l3.181 3.182m0-4.991v4.99"
            ></path>
          </svg>
        </button>
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

/// Make a request for collection list
async fn request_collections(body: CollectRequest) -> Vec<CollectionRequest> {
    let list = Vec::new();
    let res_result = request_server(
        "collections/list",
        serde_json::to_string(&body).unwrap_or_default(),
        Method::POST,
    )
    .await;
    let response = match res_result {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to send request for collections: {err:?}");
            return list;
        }
    };

    let result_json = response.json().await;
    match result_json {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to get collection list: {err:?}");
            list
        }
    }
}

async fn request_endpoints_collection(body: CollectionTargets) -> Vec<CollectionInfo> {
    let list = Vec::new();
    let res_result = request_server(
        "collections/endpoints",
        serde_json::to_string(&body).unwrap_or_default(),
        Method::POST,
    )
    .await;
    let response = match res_result {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to send request for endpoint collection info: {err:?}");
            return list;
        }
    };

    let result_json = response.json().await;
    match result_json {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to get collection info list: {err:?}");
            list
        }
    }
}

/// Sort the table by column. Right now only ID and Name is supported
fn sort_table(
    order: bool,
    update_order: &WriteSignal<bool>,
    info: &Resource<CollectRequest, Vec<CollectionRequest>>,
    column: &str,
) {
    if column == "ID" {
        if order {
            info.update(|collections| {
                collections
                    .as_mut()
                    .unwrap()
                    .sort_by(|a, b| a.info.id.cmp(&b.info.id))
            });
            update_order.set(false);
            return;
        }
        info.update(|collections| {
            collections
                .as_mut()
                .unwrap()
                .sort_by(|a, b| b.info.id.cmp(&a.info.id))
        });
        update_order.set(true);
    } else if column == "Name" {
        if order {
            info.update(|collections| {
                collections
                    .as_mut()
                    .unwrap()
                    .sort_by(|a, b| a.info.name.to_lowercase().cmp(&b.info.name.to_lowercase()))
            });
            update_order.set(false);
            return;
        }
        info.update(|collections| {
            collections
                .as_mut()
                .unwrap()
                .sort_by(|a, b| b.info.name.to_lowercase().cmp(&a.info.name.to_lowercase()))
        });
        update_order.set(true);
    }
}
