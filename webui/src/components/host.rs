use crate::{components::stats::resources::calculate_uptime, web::time::unixepoch_to_rfc};
use common::{
    server::heartbeat::Heartbeat,
    system::{Cpus, DiskDrives},
};
use leptos::{component, view, IntoView};

#[component]
/// Render host details from Heartbeat
pub(crate) fn EndpointDetails(beat: Heartbeat) -> impl IntoView {
    view! {
      <div class="px-3 py-1 m-1 gap-8 col-span-full flex">
        <div>
          <p class="font-semibold">{beat.hostname}</p>
        </div>
        <div class="divider divider-horizontal"></div>
        <div>
          <p class="font-semibold">{beat.ip}</p>
        </div>
        <div class="divider divider-horizontal"></div>
        <div>
          <p class="font-semibold">{format!("ID: {}", beat.endpoint_id)}</p>
        </div>
        <div class="divider divider-horizontal"></div>
        <div>
          <p class="font-semibold">
            {format!("Last Heartbeat: {}", unixepoch_to_rfc(beat.timestamp as i64))}
          </p>
        </div>
        <div class="divider divider-horizontal"></div>
        <div>
          <p class="font-semibold">{format!("Jobs Running: {}", beat.jobs_running)}</p>
        </div>
      </div>
      <br/>
      <div class="p-8 m-4 border-2 rounded-lg col-span-full shadow-xl flex place-content-around">
        <div>
          <p class="font-semibold">Memory</p>
          <p>{format!("{} GB", beat.memory.total_memory / (1024 * 1024 * 1024))}</p>
        </div>
        <div>
          <p class="font-semibold">CPU Usage</p>
          <p>

            {
                let mut sum = 0.0;
                for cpu in &beat.cpu {
                    sum += cpu.cpu_usage;
                }
                sum as usize / beat.cpu.len()
            } %
          </p>
        </div>
        <div>
          <p class="font-semibold">Disk Size</p>
          <p>

            {
                let mut size = 0;
                for disk in &beat.disks {
                    if disk.total_space > size {
                        size = disk.total_space;
                    }
                }
                size / (1000 * 1000 * 1000)
            } " GB"
          </p>
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
      <UptimeBootime count=beat.uptime seconds=beat.boot_time as i64/>
      <CpuInfo cpus=beat.cpu/>
      <DiskInfo disks=beat.disks/>
    }
}

#[component]
/// Display Uptime and Bootime data
fn UptimeBootime(count: u64, seconds: i64) -> impl IntoView {
    view! {
      <div class="p-8 m-4 border-2 rounded-lg flex shadow-xl place-content-evenly">
        <div>
          <p class="font-semibold">Uptime</p>
          <p>{calculate_uptime(&count)}</p>
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
      <div class="m-4 rounded-lg shadow-xl border-2 carousel">

        {cpus
            .clone()
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                let mut previous = index;
                let mut next = index + 1;
                if index == 0 {
                    previous = cpus.len() - 1;
                } else {
                    previous -= 1;
                }
                if next == cpus.len() {
                    next = 0;
                }
                view! {
                  <div id=format!("core{index}") class="carousel-item relative w-full">
                    <div class="m-2 flex place-content-evenly pl-8 ml-8">
                      <div class="stat">
                        <div class="stat-title text-zinc-600">
                          {format!(
                              "CPU Usage (Core {} of {})",
                              index + 1,
                              value.physical_core_count,
                          )}

                        </div>
                        <div class="stat-value">{format!("{}%", value.cpu_usage as u64)}</div>
                        <div class="stat-desc">{value.brand}</div>
                      </div>
                    </div>
                    <div class="absolute flex justify-between transform -translate-y-1/2 left-5 right-5 top-1/2">
                      <a href=format!("#core{previous}") class="btn btn-circle">
                        {"<"}
                      </a>
                      <a href=format!("#core{next}") class="btn btn-circle">
                        {">"}
                      </a>
                    </div>
                  </div>
                }
            })
            .collect::<Vec<_>>()}
      </div>
    }
}

#[component]
/// Display Disk data
fn DiskInfo(disks: Vec<DiskDrives>) -> impl IntoView {
    view! {
      <div class="m-4 rounded-lg shadow-xl border-2 carousel">

        {disks
            .clone()
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                let mut previous = index;
                let mut next = index + 1;
                if index == 0 {
                    previous = disks.len() - 1;
                } else {
                    previous -= 1;
                }
                if next == disks.len() {
                    next = 0;
                }
                view! {
                  <div id=format!("disk{index}") class="carousel-item relative w-full">
                    <div class="m-2 flex place-content-evenly pl-8 ml-8">
                      <div class="stat">
                        <div class="stat-title text-zinc-600">
                          {format!("Disk Drive {} of {}", index + 1, disks.len())}
                        </div>
                        <div class="stat-value">
                          {format!(
                              "{} GB Used",
                              (value.total_space - value.available_space) / (1000 * 1000 * 1000),
                          )}

                        </div>
                        <div class="stat-desc">
                          {format!(
                              "Drive Size: {} GB ({})",
                              value.total_space / (1000 * 1000 * 1000),
                              value.mount_point,
                          )}

                        </div>
                      </div>
                    </div>
                    <div class="absolute flex justify-between transform -translate-y-1/2 left-5 right-5 top-1/2">
                      <a href=format!("#disk{previous}") class="btn btn-circle">
                        {"<"}
                      </a>
                      <a href=format!("#disk{next}") class="btn btn-circle">
                        {">"}
                      </a>
                    </div>
                  </div>
                }
            })
            .collect::<Vec<_>>()}
      </div>
    }
}
