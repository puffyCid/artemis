use crate::web::time::unixepoch_to_rfc;
use common::{server::Heartbeat, system::Cpus};
use leptos::{component, view, IntoView};

#[component]
/// Render host details from Heartbeat
pub(crate) fn HostDetails(beat: Heartbeat) -> impl IntoView {
    view! {
        <div class="px-3 py-1 m-1 gap-8 col-span-full flex">
          <div><p class="font-semibold">{beat.hostname}</p></div>
          <div class="divider divider-horizontal"></div>
          <div><p class="font-semibold">{format!("ID: {}",beat.endpoint_id)}</p></div>
        </div>
        <br />
        <div class="p-8 m-4 border-2 rounded-lg col-span-full shadow-xl flex place-content-around">
          <div>
            <p class="font-semibold">Memory</p>
            <p>{beat.memory.total_memory / (1024 * 1024 * 1024)} GB</p>
          </div>
          <div>
            <p class="font-semibold">CPU Usage</p>
            <p>{
              let mut sum = 0.0;
              for cpu in &beat.cpu {
                sum += cpu.cpu_usage;
              }
              sum as usize / beat.cpu.len()
            } %</p>
          </div>
          <div>
          <p class="font-semibold">Disk Size</p>
            <p>{
              let mut size = 0;
              for disk in beat.disks {
                if disk.total_space > size {
                    size = disk.total_space;
                }
              }
              size / (1000 * 1000 * 1000)
             } GB</p>
          </div>
          <div>
            <p class="font-semibold">OS Version</p>
            <p>{beat.os_version}</p>
          </div>
          <div>
            <p class="font-semibold">Kernel Version</p>
            <p>{beat.kernel_version}</p>
          </div>
          <div>
            <p class="font-semibold">Platform</p>
            <p>{beat.platform}</p>
          </div>
        </div>
        <UptimeBootime count={beat.uptime} seconds={beat.boot_time as i64} />
        <CpuInfo cpus={beat.cpu} />
    }
}

#[component]
/// Display Uptime and Bootime data
fn UptimeBootime(count: u64, seconds: i64) -> impl IntoView {
    view! {
      <div class="p-8 m-4 border-2 rounded-lg flex shadow-xl place-content-evenly">
        <div>
          <p class="font-semibold">Uptime in Seconds</p>
          <p>{count}</p>
        </div>
        <div>
          <p class="font-semibold">Boot Time</p>
          <p>{unixepoch_to_rfc(seconds)}</p>
        </div>
      </div>
    }
}

#[component]
/// Display CPU data
fn CpuInfo(cpus: Vec<Cpus>) -> impl IntoView {
    view! {
      <div class="m-4 rounded-lg shadow-xl join join-vertical overflow-auto flex flex-nowrap">
        {cpus.into_iter().map(|value| view!{
          <div class="collapse collapse-arrow join-item border-2">
            <input type="radio" name="cpu-accordion-4" checked="checked" />
            <div class="collapse-title font-medium">
              {format!("Brand: {}", value.brand)}
            </div>
            <div class="collapse-content">
              <p>{format!("Core: {}", value.name)}</p>
              <p>{format!("CPU Usage: {}", value.cpu_usage)}</p>
              <p>{format!("Vendor: {}", value.vendor_id)}</p>
            </div>
          </div>
        }).collect::<Vec<_>>()}
      </div>
    }
}
