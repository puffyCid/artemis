use crate::artifacts::os::windows::tasks::text::read_text_unescaped;
use common::windows::{IdleSettings, MaintenanceSettings, NetworkSettings, RestartType, Settings};
use log::error;
use quick_xml::{Reader, events::Event};

/// Parse all Settings associated with Task
pub(crate) fn parse_settings(reader: &mut Reader<&[u8]>) -> Settings {
    let mut info = Settings {
        allow_start_on_demand: None,
        restart_on_failure: None,
        multiple_instances_policy: None,
        disallow_start_if_on_batteries: None,
        stop_if_going_on_batteries: None,
        allow_hard_terminate: None,
        start_when_available: None,
        network_profile_name: None,
        run_only_if_network_available: None,
        wake_to_run: None,
        enabled: None,
        hidden: None,
        delete_expired_tasks_after: None,
        idle_settings: None,
        network_settings: None,
        execution_time_limit: None,
        priority: None,
        run_only_if_idle: None,
        use_unified_scheduling_engine: None,
        disallow_start_on_remote_app_session: None,
        maintenance_settings: None,
        volatile: None,
    };

    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read Settings xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"Settings" {
                    break;
                }
            }
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"AllowStartOnDemand" => {
                    info.allow_start_on_demand = Some(
                        read_text_unescaped(reader, tag.name())
                            .parse()
                            .unwrap_or(true),
                    );
                }
                b"DisallowStartIfOnBatteries" => {
                    info.disallow_start_if_on_batteries = Some(
                        read_text_unescaped(reader, tag.name())
                            .parse()
                            .unwrap_or(true),
                    );
                }
                b"StopIfGoingOnBatteries" => {
                    info.stop_if_going_on_batteries = Some(
                        read_text_unescaped(reader, tag.name())
                            .parse()
                            .unwrap_or(true),
                    );
                }
                b"AllowHardTerminate" => {
                    info.allow_hard_terminate = Some(
                        read_text_unescaped(reader, tag.name())
                            .parse()
                            .unwrap_or(true),
                    );
                }
                b"StartWhenAvailable" => {
                    info.start_when_available = Some(
                        read_text_unescaped(reader, tag.name())
                            .parse()
                            .unwrap_or(false),
                    );
                }
                b"RunOnlyIfNetworkAvailable" => {
                    info.run_only_if_network_available = Some(
                        read_text_unescaped(reader, tag.name())
                            .parse()
                            .unwrap_or(false),
                    );
                }
                b"WakeToRun" => {
                    info.wake_to_run = Some(
                        read_text_unescaped(reader, tag.name())
                            .parse()
                            .unwrap_or(false),
                    );
                }
                b"Enabled" => {
                    info.enabled = Some(
                        read_text_unescaped(reader, tag.name())
                            .parse()
                            .unwrap_or(true),
                    );
                }
                b"Hidden" => {
                    info.hidden = Some(
                        read_text_unescaped(reader, tag.name())
                            .parse()
                            .unwrap_or(false),
                    );
                }
                b"RunOnlyIfIdle" => {
                    info.run_only_if_idle = Some(
                        read_text_unescaped(reader, tag.name())
                            .parse()
                            .unwrap_or(false),
                    );
                }
                b"UseUnifiedSchedulingEngine" => {
                    info.use_unified_scheduling_engine = Some(
                        read_text_unescaped(reader, tag.name())
                            .parse()
                            .unwrap_or(false),
                    );
                }
                b"DisallowStartOnRemoteAppSession" => {
                    info.disallow_start_on_remote_app_session = Some(
                        read_text_unescaped(reader, tag.name())
                            .parse()
                            .unwrap_or(false),
                    );
                }
                b"Volatile" => {
                    info.volatile = Some(
                        read_text_unescaped(reader, tag.name())
                            .parse()
                            .unwrap_or(false),
                    );
                }
                b"NetworkProfileName" => {
                    info.network_profile_name = Some(read_text_unescaped(reader, tag.name()));
                }
                b"DeleteExpiredTaskAfter" => {
                    info.delete_expired_tasks_after = Some(read_text_unescaped(reader, tag.name()));
                }
                b"ExecutionTimeLimit" => {
                    info.execution_time_limit = Some(read_text_unescaped(reader, tag.name()));
                }
                b"Priority" => {
                    info.priority =
                        Some(read_text_unescaped(reader, tag.name()).parse().unwrap_or(7));
                }
                b"MultipleInstancesPolicy" => {
                    info.multiple_instances_policy = Some(read_text_unescaped(reader, tag.name()));
                }
                b"RestartOnFailure" => process_restart(&mut info, reader),
                b"IdleSettings" => process_idle(&mut info, reader),
                b"NetworkSettings" => process_network(&mut info, reader),
                b"MaintenanceSettings" => process_maintenance(&mut info, reader),
                _ => break,
            },
            _ => (),
        }
    }

    info
}

/// Parse `RestartTypes`
fn process_restart(info: &mut Settings, reader: &mut Reader<&[u8]>) {
    let mut restart = RestartType {
        interval: String::new(),
        count: 0,
    };
    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read RestartSettings xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"Interval" => {
                    restart.interval = read_text_unescaped(reader, tag.name());
                }
                b"Count" => {
                    restart.count = read_text_unescaped(reader, tag.name())
                        .parse()
                        .unwrap_or_default();
                }
                _ => break,
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"RestartOnFailure" {
                    break;
                }
            }
            _ => (),
        }
    }
    info.restart_on_failure = Some(restart);
}

/// Parse `IdleSettings`
fn process_idle(info: &mut Settings, reader: &mut Reader<&[u8]>) {
    let mut idle = IdleSettings {
        duration: None,
        wait_timeout: None,
        stop_on_idle_end: None,
        restart_on_idle: None,
    };
    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read IdleSettings xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"Duration" => {
                    idle.duration = Some(read_text_unescaped(reader, tag.name()));
                }
                b"WaitTimeout" => {
                    idle.wait_timeout = Some(read_text_unescaped(reader, tag.name()));
                }
                b"StopOnIdleEnd" => {
                    idle.stop_on_idle_end = Some(
                        read_text_unescaped(reader, tag.name())
                            .parse()
                            .unwrap_or(true),
                    );
                }
                b"RestartOnIdle" => {
                    idle.restart_on_idle = Some(
                        read_text_unescaped(reader, tag.name())
                            .parse()
                            .unwrap_or(false),
                    );
                }
                _ => break,
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"IdleSettings" {
                    break;
                }
            }
            _ => (),
        }
    }
    info.idle_settings = Some(idle);
}

/// Parse `NetworkSettings`
fn process_network(info: &mut Settings, reader: &mut Reader<&[u8]>) {
    let mut net = NetworkSettings {
        name: None,
        id: None,
    };
    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read NetworkSettings xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"Name" => {
                    net.name = Some(read_text_unescaped(reader, tag.name()));
                }
                b"Id" => {
                    net.id = Some(read_text_unescaped(reader, tag.name()));
                }
                _ => break,
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"NetworkSettings" {
                    break;
                }
            }
            _ => (),
        }
    }
    info.network_settings = Some(net);
}

/// Parse `MaintenanceSettings`
fn process_maintenance(info: &mut Settings, reader: &mut Reader<&[u8]>) {
    let mut main = MaintenanceSettings {
        period: String::new(),
        deadline: None,
        exclusive: None,
    };
    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read MaintenanceSettings xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"MaintenanceSettings" {
                    break;
                }
            }
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"Period" => {
                    main.period = read_text_unescaped(reader, tag.name());
                }
                b"Deadline" => {
                    main.deadline = Some(read_text_unescaped(reader, tag.name()));
                }
                b"Exclusive" => {
                    main.exclusive = Some(
                        read_text_unescaped(reader, tag.name())
                            .parse()
                            .unwrap_or_default(),
                    );
                }
                _ => break,
            },
            _ => (),
        }
    }
    info.maintenance_settings = Some(main);
}

#[cfg(test)]
mod tests {
    use super::parse_settings;
    use crate::artifacts::os::windows::tasks::schemas::settings::{
        Settings, process_idle, process_maintenance, process_network, process_restart,
    };
    use quick_xml::Reader;

    #[test]
    fn test_parse_settings() {
        let xml = r#"
        <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
        <StopIfGoingOnBatteries>true</StopIfGoingOnBatteries>
        <Hidden>true</Hidden>
        <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
        <StartWhenAvailable>true</StartWhenAvailable>
        <IdleSettings>
          <Duration>PT10M</Duration>
          <WaitTimeout>PT1H</WaitTimeout>
          <StopOnIdleEnd>true</StopOnIdleEnd>
          <RestartOnIdle>false</RestartOnIdle>
        </IdleSettings>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let result = parse_settings(&mut reader);
        assert_eq!(result.disallow_start_if_on_batteries.unwrap(), false);
        assert_eq!(result.hidden.unwrap(), true);
    }

    #[test]
    fn test_process_restart() {
        let xml = r#"
        <Interval>P10M</Interval>
        <Count>5</Count>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut info = Settings {
            allow_start_on_demand: None,
            restart_on_failure: None,
            multiple_instances_policy: None,
            disallow_start_if_on_batteries: None,
            stop_if_going_on_batteries: None,
            allow_hard_terminate: None,
            start_when_available: None,
            network_profile_name: None,
            run_only_if_network_available: None,
            wake_to_run: None,
            enabled: None,
            hidden: None,
            delete_expired_tasks_after: None,
            idle_settings: None,
            network_settings: None,
            execution_time_limit: None,
            priority: None,
            run_only_if_idle: None,
            use_unified_scheduling_engine: None,
            disallow_start_on_remote_app_session: None,
            maintenance_settings: None,
            volatile: None,
        };
        process_restart(&mut info, &mut reader);
        assert_eq!(info.restart_on_failure.unwrap().interval, "P10M");
    }

    #[test]
    fn test_process_idle() {
        let xml = r#"
        <Duration>P10M</Duration>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut info = Settings {
            allow_start_on_demand: None,
            restart_on_failure: None,
            multiple_instances_policy: None,
            disallow_start_if_on_batteries: None,
            stop_if_going_on_batteries: None,
            allow_hard_terminate: None,
            start_when_available: None,
            network_profile_name: None,
            run_only_if_network_available: None,
            wake_to_run: None,
            enabled: None,
            hidden: None,
            delete_expired_tasks_after: None,
            idle_settings: None,
            network_settings: None,
            execution_time_limit: None,
            priority: None,
            run_only_if_idle: None,
            use_unified_scheduling_engine: None,
            disallow_start_on_remote_app_session: None,
            maintenance_settings: None,
            volatile: None,
        };
        process_idle(&mut info, &mut reader);
        assert_eq!(info.idle_settings.unwrap().duration.unwrap(), "P10M");
    }

    #[test]
    fn test_process_network() {
        let xml = r#"
        <Name>My WIFI</Name>
        <Id>Whatever</Id>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut info = Settings {
            allow_start_on_demand: None,
            restart_on_failure: None,
            multiple_instances_policy: None,
            disallow_start_if_on_batteries: None,
            stop_if_going_on_batteries: None,
            allow_hard_terminate: None,
            start_when_available: None,
            network_profile_name: None,
            run_only_if_network_available: None,
            wake_to_run: None,
            enabled: None,
            hidden: None,
            delete_expired_tasks_after: None,
            idle_settings: None,
            network_settings: None,
            execution_time_limit: None,
            priority: None,
            run_only_if_idle: None,
            use_unified_scheduling_engine: None,
            disallow_start_on_remote_app_session: None,
            maintenance_settings: None,
            volatile: None,
        };
        process_network(&mut info, &mut reader);
        assert_eq!(info.network_settings.unwrap().name.unwrap(), "My WIFI");
    }

    #[test]
    fn test_process_maintence() {
        let xml = r#"
        <Period>P10M</Period>
        <Deadline>Now</Deadline>
        <Exclusive>false</Exclusive>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut info = Settings {
            allow_start_on_demand: None,
            restart_on_failure: None,
            multiple_instances_policy: None,
            disallow_start_if_on_batteries: None,
            stop_if_going_on_batteries: None,
            allow_hard_terminate: None,
            start_when_available: None,
            network_profile_name: None,
            run_only_if_network_available: None,
            wake_to_run: None,
            enabled: None,
            hidden: None,
            delete_expired_tasks_after: None,
            idle_settings: None,
            network_settings: None,
            execution_time_limit: None,
            priority: None,
            run_only_if_idle: None,
            use_unified_scheduling_engine: None,
            disallow_start_on_remote_app_session: None,
            maintenance_settings: None,
            volatile: None,
        };
        process_maintenance(&mut info, &mut reader);
        assert_eq!(info.maintenance_settings.unwrap().deadline.unwrap(), "Now");
    }
}
