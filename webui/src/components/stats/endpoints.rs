use crate::web::server::request_server;
use common::server::EndpointOS;
use leptos::logging::error;
use leptos::{component, create_resource, view, IntoView, SignalGet, Transition};
use reqwest::Method;

#[component]
/// Calculate endpoint counts
pub(crate) fn Stats(
    /// Endpoint OS to count
    os: EndpointOS,
    html: String,
) -> impl IntoView {
    let count = create_resource(|| (), move |_| async move { endpoint_stats(&os).await });

    view! {
        <div class="stat shadow">
            <div class="stat-figure text-primary" inner_html=html></div>
            <div class="stat-title"> {format!("{os:?} Endpoint Count")}</div>
            <div class="stat-value">
                <Transition fallback=move || view!{<p> "Loading..."</p>}>
                    {move || count.get()}
                </Transition>
            </div>
        </div>
    }
}

/// Request count of endpoints enrolled
async fn endpoint_stats(os: &EndpointOS) -> u32 {
    let res_result = request_server(
        "endpoint/stats",
        serde_json::to_string(&os).unwrap_or_default(),
        Method::POST,
    )
    .await;

    let response = match res_result {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to make request for {os:?} endpoint count: {err:?}");
            return 0;
        }
    };

    let res = response.text().await;
    let count_str = match res {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to get {os:?} endpoint count: {err:?}");
            return 0;
        }
    };
    let count_result = count_str.parse::<u32>();
    match count_result {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to parse {os:?} endpoint count {count_str}: {err:?}");
            0
        }
    }
}
