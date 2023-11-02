use leptos::{component, view, IntoView};

#[derive(Debug)]
pub(crate) enum EndpointOS {
    Windows,
    MacOS,
    Linux,
    All,
}

#[component]
/// Calculate endpoint counts
pub(crate) fn Stats(
    /// Endpoint OS to count
    os: EndpointOS,
) -> impl IntoView {
    view! {
        <div class="stat shadow">
            <div class="stat-figure text-primary">{format!("{os:?} icon")}</div>
            <div class="stat-title"> {format!("{os:?} Endpoint Count")}</div>
            <div class="stat-value"> {endpoint_stats(os)}</div>
        </div>
    }
}

/// Request count of endpoints enrolled
fn endpoint_stats(os: EndpointOS) -> u32 {
    match os {
        EndpointOS::All => 10,
        EndpointOS::Linux => 5,
        EndpointOS::MacOS => 25,
        EndpointOS::Windows => 100,
    }
}

#[cfg(test)]
mod tests {
    use super::{endpoint_stats, EndpointOS};

    #[test]
    fn test_endpoint_stats() {
        let os = EndpointOS::All;
        let stats = endpoint_stats(os);
        assert_eq!(stats, 10);
    }
}
