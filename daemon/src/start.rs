use std::{
    thread::{sleep, spawn},
    time::Duration,
};

use crate::{
    collection::{collect::CollectEndpoint, error::CollectError},
    logging::logs::LoggingEndpoint,
    utils::{
        config::{Daemon, DaemonToml, ServerToml, server},
        setup::{move_server_config, setup_collection, setup_config, setup_enrollment},
    },
};

pub(crate) struct DaemonConfig {
    pub(crate) server: ServerToml,
    pub(crate) client: DaemonToml,
}

/// Start artemis as a daemon and collect data based on remote server responses
pub fn start_daemon(path: Option<&str>, alt_base: Option<&str>) {
    // We will enroll to a remote server based on a server.toml config
    // By default we assume server.toml is in same directory as binary
    let mut server_path = "server.toml";

    if let Some(config_path) = path {
        server_path = config_path;
    }

    // Attempt to read to server TOML config file
    let server_config = match server(server_path, alt_base) {
        Ok(result) => result,
        Err(_err) => return,
    };

    let mut config = DaemonConfig {
        server: server_config,
        client: DaemonToml {
            daemon: Daemon {
                endpoint_id: String::new(),
                collection_path: String::new(),
                log_level: String::new(),
            },
        },
    };

    // Attempt to connect to server
    setup_enrollment(&mut config);
    setup_config(&mut config);

    // We have enough info connect to our server.
    // Can move our server.toml to our base config directory. Ex: /var/artemis/server.toml
    move_server_config(server_path, alt_base);
    start(&mut config);
}

/// Continuously poll our server for jobs and collections
fn start(config: &mut DaemonConfig) {
    let max_attempts = 8;
    let mut count = 0;

    let pause = 8;
    let collection_poll = 60;
    loop {
        if count == max_attempts {
            let long_pause = 300;

            sleep(Duration::from_secs(long_pause));
            count = 0;
        }
        let collection = match config.collect_request() {
            Ok(result) => result,
            Err(_err) => {
                count += 1;
                sleep(Duration::from_secs(pause));
                continue;
            }
        };

        if collection.endpoint_invalid {
            setup_enrollment(config);
            continue;
        }

        let handle = spawn(move || {
            setup_collection(&collection);
        });

        // While thread is running continue to poll the server
        while !handle.is_finished() {
            println!("{}", handle.is_finished());

            if count == max_attempts {
                println!("long pause?");
                let long_pause = 300;

                sleep(Duration::from_secs(long_pause));
                count = 0;
            }
            let collection = match config.collect_request() {
                Ok(result) => result,
                Err(err) => {
                    if err != CollectError::NoCollection {
                        count += 1;
                    }
                    sleep(Duration::from_secs(pause));
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
        println!("log upload?");
        let _ = handle.join();
        // Upload any logs from the collection
        let log_status = match config.log_upload() {
            Ok(result) => result,
            Err(_err) => {
                count += 1;
                sleep(Duration::from_secs(pause));
                continue;
            }
        };

        // If server responded with invalid endpoint, we have to re-enroll
        if log_status.endpoint_invalid {
            setup_enrollment(config);
            continue;
        }

        // Next poll will be in 60 seconds
        sleep(Duration::from_secs(collection_poll));
    }
}
