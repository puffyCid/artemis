use std::{
    thread::{sleep, spawn},
    time::Duration,
};

use rand::Rng;

use crate::{
    collection::{
        collect::{CollectEndpoint, CollectionStatus},
        error::CollectError,
    },
    logging::{error::LoggingError, logs::LoggingEndpoint},
    utils::{
        config::{ServerToml, have_config, server},
        setup::{setup_collection, setup_config, setup_enrollment},
        time::time_now,
    },
};

pub(crate) struct DaemonConfig {
    pub(crate) server: ServerToml,
}

/// Start artemis as a daemon and collect data based on remote server responses
pub fn start_daemon(path: Option<String>, alt_base: Option<String>) {
    // We will enroll to a remote server based on a server.toml config
    // By default we assume server.toml is in same directory as binary
    let mut server_path = String::from("server.toml");

    if let Some(config_path) = path {
        server_path = config_path;
    }

    // If we have an existing config already (from prior enrollment). Reuse that
    if let Some(existing_config) = have_config() {
        server_path = existing_config;
    }

    // Attempt to read to server TOML config file
    let server_config = match server(&server_path, alt_base.as_deref()) {
        Ok(result) => result,
        Err(_err) => return,
    };

    let mut config = DaemonConfig {
        server: server_config,
    };

    if config.server.daemon.endpoint_id.is_empty() {
        // Attempt to connect to server
        setup_enrollment(&mut config);
    }
    // If our endpoint ID is empty. We cannot communicate with the server
    if config.server.daemon.endpoint_id.is_empty() {
        return;
    }

    setup_config(&mut config);

    start(&mut config);
}

/// Continuously poll our server for jobs and collections
fn start(config: &mut DaemonConfig) {
    let max_attempts = 6;

    let pause = 8;
    let collection_poll = 60;
    let mut attempt = 1;
    let mut rng = rand::rng();

    loop {
        let jitter: u16 = rng.random_range(..=10);
        let backoff = if attempt <= max_attempts {
            pause * attempt + jitter
        } else {
            // If 6 attempts fail. Then backoff for 5 mins
            300 + jitter
        };

        // Upload any logs accumulated
        match config.log_upload() {
            Ok(log_status) => {
                // If server responded with invalid endpoint, we have to re-enroll
                if log_status.endpoint_invalid {
                    setup_enrollment(config);
                    continue;
                }
                attempt = 0;
            }
            Err(err) => {
                // If we failed to open or read the file log
                // It may be because we have no logs
                // Do not reattempt on failed file reads/opens
                if err != LoggingError::OpenFile && err != LoggingError::ClearLog {
                    attempt += 1;
                    sleep(Duration::from_secs(backoff as u64));
                    continue;
                }
            }
        };

        let collection = match config.collect_request() {
            Ok(result) => result,
            Err(_err) => {
                attempt += 1;
                sleep(Duration::from_secs(backoff as u64));
                continue;
            }
        };
        attempt = 0;
        let collection_id = collection.collection_id;

        if collection.endpoint_invalid {
            setup_enrollment(config);
            continue;
        }

        // Allow the collection to run for allocated timer. Default should be 300 seconds
        let timeout = time_now() + collection.collection_timeout;
        let handle = spawn(move || setup_collection(&collection));

        // While thread is running continue to poll the server
        while !handle.is_finished() {
            let jitter: u16 = rng.random_range(..=10);
            let backoff = if attempt <= max_attempts {
                pause * attempt + jitter
            } else {
                // If 6 attempts fail. Then backoff for 5 mins
                300 + jitter
            };
            let collection = match config.collect_request() {
                Ok(result) => result,
                Err(err) => {
                    if err != CollectError::NoCollection {
                        attempt += 1;
                    }
                    sleep(Duration::from_secs(backoff as u64));
                    continue;
                }
            };

            if collection.endpoint_invalid {
                setup_enrollment(config);
                continue;
            }
            // Next poll will be in 60 seconds
            sleep(Duration::from_secs(collection_poll));
        }

        let status = match handle.join() {
            Ok(status) => status,
            Err(_err) => CollectionStatus::Error,
        };

        // The final part of a remote forensic collection
        // Sending POST request to let the server know the collection is done
        loop {
            let jitter: u16 = rng.random_range(..=10);
            let backoff = if attempt <= max_attempts {
                pause * attempt + jitter
            } else {
                // If 6 attempts fail. Then backoff for 5 mins
                300 + jitter
            };

            // Now send request to mark collection as completed or error
            let response = match config.complete_collection(status, collection_id) {
                Ok(result) => result,
                Err(_err) => {
                    attempt += 1;
                    sleep(Duration::from_secs(backoff as u64));
                    continue;
                }
            };

            // If server responded with invalid endpoint, we have to re-enroll
            if response.endpoint_invalid {
                setup_enrollment(config);
            }
            break;
        }

        // Next poll will be in 60 seconds
        sleep(Duration::from_secs(collection_poll));
    }
}
