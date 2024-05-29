use crate::web::server::request_server;
use common::server::webui::ServerInfo;
use leptos::logging::error;
use leptos::{component, create_resource, view, IntoView, SignalGet, Transition};
use reqwest::Method;

#[component]
/// Resources page
pub(crate) fn Resources() -> impl IntoView {
    let info = create_resource(|| {}, move |_| async move { get_info().await });
    view! {
      <div class="stat shadow">
        <div class="stat-title">Server CPU Usage</div>
        <div class="stat-value">
          <Transition fallback=move || {
              view! { <p>"Loading..."</p> }
          }>
            {move || {
                info.get()
                    .map(|res| {
                        (res.cpu_usage.iter().sum::<f32>() as f64 / res.cpu_usage.len() as f64)
                            as u64
                    })
            }}

          </Transition>
          %
        </div>
      </div>
      <div class="stat shadow">
        <div class="stat-title">Server Memory Usage</div>
        <div class="stat-value">
          <Transition fallback=move || {
              view! { <p>"Loading..."</p> }
          }>{move || info.get().map(|res| { res.memory_used / (1024 * 1024 * 1024) })}</Transition>
          GB
        </div>
        <div classs="stat-desc">
          <Transition fallback=move || {
              view! { <p>"Loading..."</p> }
          }>{move || info.get().map(|res| { res.total_memory / (1024 * 1024 * 1024) })}</Transition>
          GB of Total Memory
        </div>
      </div>
      <div class="stat shadow">
        <div class="stat-title">Server Disk Usage</div>
        <div class="stat-value">
          <Transition fallback=move || {
              view! { <p>"Loading..."</p> }
          }>
            {move || {
                info.get()
                    .map(|res| {
                        let mut usage = 0;
                        for disk in res.disk_info {
                            if disk.disk_usage > usage {
                                usage = disk.disk_usage;
                            }
                        }
                        usage / (1000 * 1000 * 1000)
                    })
            }}

          </Transition>
          GB
        </div>
        <div classs="stat-desc">
          <Transition fallback=move || {
              view! { <p>"Loading..."</p> }
          }>
            {move || {
                info.get()
                    .map(|res| {
                        let mut size = 0;
                        for disk in res.disk_info {
                            if disk.disk_size > size {
                                size = disk.disk_size;
                            }
                        }
                        size / (1000 * 1000 * 1000)
                    })
            }}

          </Transition>
          GB Total Disk Size
        </div>
      </div>
      <div class="stat shadow">
        <div class="stat-title">Server Uptime</div>
        <div class="stat-value">
          <Transition fallback=move || {
              view! { <p>"Loading..."</p> }
          }>{move || info.get().map(|res| { calculate_uptime(&res.uptime) })}</Transition>
        </div>
      </div>
    }
}

/// Query server to get resource usage
async fn get_info() -> ServerInfo {
    let info = ServerInfo {
        memory_used: 0,
        total_memory: 0,
        uptime: 0,
        cpu_usage: Vec::new(),
        disk_info: Vec::new(),
    };

    let res_result = request_server("server/stats", String::new(), Method::GET).await;
    let response = match res_result {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to get server resources: {err:?}");
            return info;
        }
    };

    let result_json = response.json().await;
    match result_json {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to get server resources: {err:?}");
            info
        }
    }
}

/// Determine system uptime
pub(crate) fn calculate_uptime(uptime: &u64) -> String {
    let mins = 60;
    let hours = 3600;
    let days = 86400;
    let months = 2628003;
    let years = 31536000;

    let mut value = format!("{uptime} seconds");

    if uptime >= &mins {
        value = format!("{} mins", uptime / mins)
    }
    if uptime >= &hours {
        value = format!("{} hours", uptime / hours)
    }
    if uptime >= &days {
        value = format!("{} days", uptime / days)
    }
    if uptime >= &months {
        value = format!("{} months", uptime / months)
    }
    if uptime >= &years {
        value = format!("{} years", uptime / years)
    }

    value
}
