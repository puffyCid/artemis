use crate::web::server::request_server;
use common::server::{EndpointList, EndpointOS, EndpointRequest};
use leptos::logging::error;
use leptos::{component, create_resource, view, IntoView, SignalGet, Transition};
use reqwest::Method;

#[component]
/// Render table of endpoints
pub(crate) fn Enrollment() -> impl IntoView {
    let headers = vec!["OS", "Hostname", "Version", "Last Pulse", ""];
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
                          <td>{entry.os}</td>
                          <td>{entry.hostname}</td>
                          <td>{entry.version}</td>
                          <td>{entry.last_pulse}</td>
                          <th>
                            <button class="btn btn-ghost btn-sm">Info</button>
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
        "endpoint_list",
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
