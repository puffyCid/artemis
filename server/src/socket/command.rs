use crate::filestore::{endpoints::glob_paths, jobs::save_job};
use axum::extract::ws::Message;
use common::server::jobs::{Command, JobType};
use log::error;
use serde_json::Error;
use std::collections::HashMap;
use tokio::sync::{mpsc, RwLockReadGuard};

/// Parse the Server command Job info. If the `JobType` is a collection save the job to disk for client to pickup on checkin
pub(crate) async fn parse_command(data: &str, path: &str) -> Result<(), Error> {
    let command_result: Result<Command, Error> = serde_json::from_str(data);
    let command = match command_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to deserialize command: {err:?}");
            return Err(err);
        }
    };

    if command.job.job_type != JobType::Collection {
        return Ok(());
    }

    // Only Collection Jobs are saved. All other Jobs run in real time
    for target in command.targets {
        let glob_path = glob_paths(&format!("{path}/*/{target}")).unwrap_or_default();
        for endpoint_path in glob_path {
            let status = save_job(command.job.clone(), &endpoint_path.full_path).await;
            if status.is_err() {
                error!(
                    "[server] Could not save job collection at {}",
                    endpoint_path.full_path
                );
            }
        }
    }

    Ok(())
}

/// Send jobs to client endpoints from server. The data is uploaded via websockets
pub(crate) async fn quick_jobs(
    data: &str,
    channels: &RwLockReadGuard<'_, HashMap<String, mpsc::Sender<Message>>>,
) -> Result<(), Error> {
    println!("commdn: {data}");
    let command_result: Result<Command, Error> = serde_json::from_str(data);
    let command = match command_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to deserialize command: {err:?}");
            return Err(err);
        }
    };

    // Cannot send collection jobs through websockets. These are picked up when the client connects via heartbeat
    if command.job.job_type == JobType::Collection {
        return Ok(());
    }

    // Loop through target endpoint IDs that should receive the job
    for target in command.targets {
        // Check if target endpoint ID found in HashMap
        if let Some(sender) = channels.get(&target) {
            let job_result = serde_json::to_string(&command.job);
            let job = match job_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[server] Failed to serialize job: {err:?}");
                    continue;
                }
            };

            // Send job to the async client task. The job will only be sent the associated endpoint ID
            let result = sender.send(Message::Text(job)).await;
            if result.is_err() {
                error!(
                    "[server] Could not send quick job command: {:?}",
                    result.unwrap_err()
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::socket::command::{parse_command, quick_jobs};
    use crate::utils::filesystem::create_dirs;
    use axum::extract::ws::Message;
    use std::collections::HashMap;
    use tokio::sync::{mpsc, RwLock};

    #[tokio::test]
    async fn test_parse_command() {
        let data = r#"{"targets":["0998b365-b60d-4c0c-a629-f631afa83d2c", "madeup"],"job":{"id":1,"name":"processes","created":10000,"started":10001,"finished":20000,"status":"NotStarted","collection":"adssafasdfsadfs==","duration":10,"start_time":100,"action":"Start","job_type":"Collection"}}"#;
        let path = "./tmp";
        create_dirs(path).await.unwrap();
        parse_command(data, path).await.unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "Error")]
    async fn test_parse_command_bad_data() {
        let data = r#"{"asdfasdf"}"#;
        let path = "./tmp";
        parse_command(data, path).await.unwrap();
    }

    #[tokio::test]
    async fn test_quick_jobs() {
        let data = r#"{"targets":["0998b365-b60d-4c0c-a629-f631afa83d2c"],"job":{"id":1,"name":"processes","created":10000,"started":10001,"finished":20000,"status":"NotStarted","collection":"adssafasdfsadfs==","duration":10,"start_time":100,"action":"Start","job_type":"Processes"}}"#;
        let mut test = HashMap::new();
        let (client_send, mut client_recv) = mpsc::channel(5);
        test.insert(
            String::from("0998b365-b60d-4c0c-a629-f631afa83d2c"),
            client_send.clone(),
        );

        let _send_task = tokio::spawn(async move {
            while let Some(_msg) = client_recv.recv().await {
                client_send
                    .send(Message::Text(data.to_string()))
                    .await
                    .unwrap();
            }
        });

        let rw = RwLock::new(test);

        quick_jobs(data, &rw.read().await).await.unwrap();
    }
}
