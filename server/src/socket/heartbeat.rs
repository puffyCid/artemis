use crate::utils::filesystem::append_file;
use common::server::heartbeat::Heartbeat;
use log::error;
use serde_json::Error;

/// Parse a heartbeat from a system. Heartbeat occurs every 300 seconds
pub(crate) async fn parse_heartbeat(data: &str, ip: &str, endpoint_path: &str) -> (String, String) {
    let beat_result: Result<Heartbeat, Error> = serde_json::from_str(data);
    let beat = match beat_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to deserialize heartbeat from {ip}: {err:?}");
            return (String::new(), String::new());
        }
    };

    // Heartbeat.jsonl size limit is 10MB
    let beat_size_limit = 10485760;
    let path = format!(
        "{endpoint_path}/{}/{}/heartbeat.jsonl",
        beat.platform, beat.endpoint_id
    );

    let status = append_file(
        &serde_json::to_string(&beat).unwrap(),
        &path,
        &beat_size_limit,
    )
    .await;
    if status.is_err() {
        println!(
            "[server] Could not update heartbeat.jsonl file from {ip}: {:?}",
            status.unwrap_err()
        );
    }
    (beat.endpoint_id, beat.platform)
}

#[cfg(test)]
mod tests {
    use super::parse_heartbeat;
    use crate::utils::filesystem::create_dirs;

    #[tokio::test]
    async fn test_parse_heartbeat() {
        let test = r#"{"endpoint_id":"randomkey","ip":"127.0.0.1","artemis_version":"0.9.0","hostname":"hello","platform":"Darwin","boot_time":0,"os_version":"12.0","uptime":110,"kernel_version":"12.1","heartbeat":true,"timestamp":1111111,"jobs_running":0,"cpu":[{"frequency":0,"cpu_usage":25.70003890991211,"name":"1","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":25.076454162597656,"name":"2","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":8.922499656677246,"name":"3","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":6.125399112701416,"name":"4","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":4.081260681152344,"name":"5","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":3.075578451156616,"name":"6","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":2.0113024711608887,"name":"7","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":1.5097296237945557,"name":"8","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":1.288386583328247,"name":"9","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":1.1674108505249023,"name":"10","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10}],"disks":[{"disk_type":"SSD","file_system":"97112102115","mount_point":"/","total_space":494384795648 ,"available_space":295755320592 ,"removable":false},{"disk_type":"SSD","file_system":"97112102115","mount_point":"/System/Volumes/Data","total_space":494384795648 ,"available_space":295755320592 ,"removable":false}],"memory":{"available_memory":20146110464 ,"free_memory":6238076928 ,"free_swap":0,"total_memory":34359738368 ,"total_swap":0,"used_memory":18717523968 ,"used_swap":0}}"#;
        let ip = "127.0.0.1";
        let path = "./tmp";
        create_dirs(path).await.unwrap();
        let (id, plat) = parse_heartbeat(test, ip, path).await;
        assert_eq!(id, "randomkey");
        assert_eq!(plat, "Darwin");
    }

    #[tokio::test]
    async fn test_parse_heartbeat_bad_path() {
        let test = r#"{"endpoint_id":"randomkey","ip":"127.0.0.1","artemis_version":"0.9.0","hostname":"hello","platform":"Darwin","boot_time":0,"os_version":"12.0","uptime":110,"kernel_version":"12.1","heartbeat":true,"timestamp":1111111,"jobs_running":0,"cpu":[{"frequency":0,"cpu_usage":25.70003890991211,"name":"1","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":25.076454162597656,"name":"2","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":8.922499656677246,"name":"3","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":6.125399112701416,"name":"4","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":4.081260681152344,"name":"5","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":3.075578451156616,"name":"6","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":2.0113024711608887,"name":"7","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":1.5097296237945557,"name":"8","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":1.288386583328247,"name":"9","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":1.1674108505249023,"name":"10","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10}],"disks":[{"disk_type":"SSD","file_system":"97112102115","mount_point":"/","total_space":494384795648 ,"available_space":295755320592 ,"removable":false},{"disk_type":"SSD","file_system":"97112102115","mount_point":"/System/Volumes/Data","total_space":494384795648 ,"available_space":295755320592 ,"removable":false}],"memory":{"available_memory":20146110464 ,"free_memory":6238076928 ,"free_swap":0,"total_memory":34359738368 ,"total_swap":0,"used_memory":18717523968 ,"used_swap":0}}"#;
        let ip = "127.0.0.1";
        let path = "./tmp2";
        let (id, plat) = parse_heartbeat(test, ip, path).await;
        assert_eq!(id, "randomkey");
        assert_eq!(plat, "Darwin");
    }
}
