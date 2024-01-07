use common::server::Heartbeat;
use leptos::{component, view, IntoView};

#[component]
pub(crate) fn HostDetails(beat: Heartbeat) -> impl IntoView {
    view! {
        <div class="prose px-3 py-1 flex w-full col-span-full">
          <div><h4>{beat.hostname}</h4></div>
          <div class="divider divider-horizontal"></div>
          <div><h6>{beat.endpoint_id}</h6></div>
        </div>
        <div class="prose p-3 flex col-span-full w-full">
          <div>
            <h6>Memory</h6>
            <p>{beat.memory.total_memory / (1024 * 1024 * 1024)} GB</p>
          </div>
          <div class="divider divider-horizontal"></div>
          <div>
            <h6>CPU</h6>
            <p>{beat.cpu.len()} x {beat.cpu[0].frequency} GHz</p>
          </div>
          <div class="divider divider-horizontal"></div>
          <div>
          <h6>Disk Size</h6>
            <p>{
              let mut size = 0;
              for disk in beat.disks {
                if disk.total_space > size {
                    size = disk.total_space;
                }
              }
              size / (1024 * 1024 * 1024)
             } GB</p>
          </div>
          <div class="divider divider-horizontal"></div>
          <div>
            <h6>OS Version</h6>
            <p>{beat.os_version}</p>
          </div>
          <div class="divider divider-horizontal"></div>
          <div>
            <h6>Kernel Version</h6>
            <p>{beat.kernel_version}</p>
          </div>
          <div class="divider divider-horizontal"></div>
          <div>
            <h6>Platform</h6>
            <p>{beat.platform}</p>
          </div>
        </div>
    }
}
