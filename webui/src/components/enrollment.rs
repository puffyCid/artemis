use crate::components::host::HostDetails;
use crate::components::host_navigation::Navigate;
use crate::web::server::request_server;
use common::server::{EndpointList, EndpointOS, EndpointRequest, Heartbeat};
use common::system::Memory;
use leptos::logging::error;
use leptos::{
    component, create_resource, create_signal, view, IntoView, ReadSignal, Show, SignalGet,
    Transition, WriteSignal,
};
use leptos_router::{use_query_map, Form};
use reqwest::Method;

#[component]
/// Render table of endpoints
pub(crate) fn Enrollment() -> impl IntoView {
    let headers = vec!["OS", "Hostname", "Version", "IP", "ID", ""];
    let info = create_resource(|| {}, move |_| async move { get_endpoints().await });

    view! {
        <div class="overflow-x-auto col-span-full">
          <table class="table table-zebra">
            // Table Header
            <thead>
              <tr>
              {headers.into_iter().map(|entry| view!{<th>{entry}</th>}).collect::<Vec<_>>()}
              </tr>
            </thead>
            // Table Rows
            <tbody>
              <Transition fallback=move || view!{<tr><th>Loading...</th></tr>}>
                {move || info.get().map(|res| {
                    res.into_iter().map(|entry| view!{
                        <tr>
                          <td>{&entry.os}</td>
                          <td>{&entry.hostname}</td>
                          <td>{&entry.version}</td>
                          <td>TODO</td>
                          <td>{entry.id.clone()}</td>
                          <th>
                            <Form action="info" method="get">
                              <input type="hidden" name="query" value={format!("{}.{}", entry.os, entry.id)} />
                              <input type="submit" class="btn btn-ghost btn-sm" value="info"/>
                            </Form>
                          </th>
                        </tr>
                    }).collect::<Vec<_>>()
                })}
              </Transition>
            </tbody>
          </table>
        </div>
    }
}

pub(crate) struct InfoValue {
    pub(crate) info: ReadSignal<bool>,
    pub(crate) users: ReadSignal<bool>,
    pub(crate) proc: ReadSignal<bool>,
    pub(crate) set_info: WriteSignal<bool>,
    pub(crate) set_users: WriteSignal<bool>,
    pub(crate) set_proc: WriteSignal<bool>,
}

#[component]
/// Get the details from queried endpoint
pub(crate) fn GetInfo() -> impl IntoView {
    let query = use_query_map();
    // search stored as ?q=
    let search = move || query.get().get("query").cloned().unwrap_or_default();
    let results = create_resource(search, endpoint_info);

    let (info, set_info) = create_signal(true);
    let (users, set_users) = create_signal(false);
    let (proc, set_proc) = create_signal(false);

    let values = InfoValue {
        info,
        users,
        proc,
        set_info,
        set_users,
        set_proc,
    };
    view! {
      <Show when=move || {info.get()} fallback= || view!{<p> "Loading..."</p>}>
        <Transition fallback=move || view!{<p> "Loading..."</p>}>
          {move || results.get().map(|res| {
            view!{<HostDetails beat=res/>}
          })}
        </Transition>
      </Show>
      <Navigate values />
    }
}

/// Request list of endpoints from server
async fn get_endpoints() -> Vec<EndpointList> {
    let list = Vec::new();
    let body = EndpointRequest {
        pagination: String::new(),
        filter: EndpointOS::All,
        tags: Vec::new(),
        search: String::new(),
    };

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
        os_version: String::new(),
        uptime: 0,
        kernel_version: String::new(),
        platform: String::new(),
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
