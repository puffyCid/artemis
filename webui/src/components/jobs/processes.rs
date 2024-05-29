use crate::web::server::request_server;
use common::server::jobs::ProcessJob;
use leptos::{component, logging::error, view, IntoView};
use reqwest::Method;

#[component]
/// Display process listing info
pub(crate) fn HostProcesses(procs: Option<ProcessJob>) -> impl IntoView {
    if procs.is_none() {
        return view! { <div>"No processes"</div> };
    }
    let proc_data = procs.unwrap();
    let headers = vec!["Path", "Name", "PID", "PPID", "Start Time", ""];

    view! {
      <div class="overflow-x-auto col-span-full">
        <table class="table table-zebra">
          // Table Header
          <thead>
            <tr>
              {headers.into_iter().map(|entry| view! { <th>{entry}</th> }).collect::<Vec<_>>()}
            </tr>
          </thead>
          // Table Rows
          <tbody>
            {proc_data
                .data
                .into_iter()
                .map(|res| {
                    view! {
                      <tr>
                        <td>{res.full_path}</td>
                        <td>{res.name}</td>
                        <td>{res.pid}</td>
                        <td>{res.ppid}</td>
                        <td>{res.start_time}</td>
                        <th>Details</th>
                      </tr>
                    }
                })
                .collect::<Vec<_>>()}
          </tbody>
        </table>
      </div>
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
