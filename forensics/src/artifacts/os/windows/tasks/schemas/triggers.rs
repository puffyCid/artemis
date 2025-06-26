use crate::utils::strings::extract_utf8_string;
use common::windows::{
    BaseTriggers, BootTrigger, ByDay, ByMonth, ByMonthDayWeek, ByWeek, CalendarTrigger,
    EventTrigger, IdleTrigger, LogonTrigger, Repetition, SessionTrigger, TimeTrigger, Triggers,
    WnfTrigger,
};
use log::error;
use quick_xml::{Reader, events::Event, name::QName};

/// Parse all Task Trigger options.
pub(crate) fn parse_trigger(reader: &mut Reader<&[u8]>) -> Triggers {
    let mut info = Triggers {
        boot: Vec::new(),
        registration: Vec::new(),
        idle: Vec::new(),
        time: Vec::new(),
        event: Vec::new(),
        logon: Vec::new(),
        session: Vec::new(),
        calendar: Vec::new(),
        wnf: Vec::new(),
    };

    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read Triggers xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"BootTrigger" => process_boot(&mut info, reader, true),
                b"RegistrationTrigger" => process_boot(&mut info, reader, false),
                b"IdleTrigger" => process_idle(&mut info, reader),
                b"TimeTrigger" => process_time(&mut info, reader),
                b"EventTrigger" => process_event(&mut info, reader),
                b"LogonTrigger" => process_logon(&mut info, reader),
                b"SessionStateChangeTrigger" => process_session(&mut info, reader),
                b"CalendarTrigger" => process_calendar(&mut info, reader),
                b"WnfStateChangeTrigger" => process_notification(&mut info, reader),
                _ => break,
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"Triggers" {
                    break;
                }
            }
            _ => (),
        }
    }

    info
}

/// Parse `BootTrigger` options
fn process_boot(info: &mut Triggers, reader: &mut Reader<&[u8]>, is_boot: bool) {
    let mut boot = BootTrigger {
        common: None,
        delay: None,
    };

    let mut common = BaseTriggers {
        id: None,
        start_boundary: None,
        end_boundary: None,
        enabled: None,
        execution_time_limit: None,
        repetition: None,
    };
    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read BootTrigger xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"Delay" => {
                    boot.delay = Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                _ => process_common(&mut common, &tag.name(), reader),
            },
            Ok(Event::End(tag)) => match tag.name().as_ref() {
                b"BootTrigger" | b"RegistrationTrigger" => break,
                _ => (),
            },
            _ => (),
        }
    }

    boot.common = Some(common);
    if is_boot {
        info.boot.push(boot);
    } else {
        info.registration.push(boot);
    }
}

/// Parse `Wnf` (Windows Notification) options
fn process_notification(info: &mut Triggers, reader: &mut Reader<&[u8]>) {
    let mut wnf = WnfTrigger {
        common: None,
        delay: None,
        state_name: String::new(),
        data: None,
        data_offset: None,
    };

    let mut common = BaseTriggers {
        id: None,
        start_boundary: None,
        end_boundary: None,
        enabled: None,
        execution_time_limit: None,
        repetition: None,
    };
    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read WnfTrigger xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"Delay" => {
                    wnf.delay = Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                b"StateName" => {
                    wnf.state_name = reader.read_text(tag.name()).unwrap_or_default().to_string();
                }
                b"Data" => {
                    wnf.data = Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                b"DataOffset" => {
                    wnf.data_offset =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                _ => process_common(&mut common, &tag.name(), reader),
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"WnfStateChangeTrigger" {
                    break;
                }
            }
            _ => (),
        }
    }

    wnf.common = Some(common);
    info.wnf.push(wnf);
}

/// Parse `IdleTrigger` options
fn process_idle(info: &mut Triggers, reader: &mut Reader<&[u8]>) {
    let mut common = BaseTriggers {
        id: None,
        start_boundary: None,
        end_boundary: None,
        enabled: None,
        execution_time_limit: None,
        repetition: None,
    };
    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read IdleTrigger xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => {
                process_common(&mut common, &tag.name(), reader);
            }
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"IdleTrigger" {
                    break;
                }
            }
            _ => (),
        }
    }
    let idle = IdleTrigger {
        common: Some(common),
    };
    info.idle.push(idle);
}

/// Parse `TimeTrigger` options
fn process_time(info: &mut Triggers, reader: &mut Reader<&[u8]>) {
    let mut time = TimeTrigger {
        common: None,
        random_delay: None,
    };

    let mut common = BaseTriggers {
        id: None,
        start_boundary: None,
        end_boundary: None,
        enabled: None,
        execution_time_limit: None,
        repetition: None,
    };
    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read TimeTrigger xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"RandomDelay" => {
                    time.random_delay =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                _ => process_common(&mut common, &tag.name(), reader),
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"TimeTrigger" {
                    break;
                }
            }
            _ => (),
        }
    }

    time.common = Some(common);
    info.time.push(time);
}

/// Parse `EventTrigger` options
fn process_event(info: &mut Triggers, reader: &mut Reader<&[u8]>) {
    let mut event = EventTrigger {
        common: None,
        subscription: Vec::new(),
        delay: None,
        number_of_occurrences: None,
        period_of_occurrence: None,
        matching_element: None,
        value_queries: None,
    };

    let mut common = BaseTriggers {
        id: None,
        start_boundary: None,
        end_boundary: None,
        enabled: None,
        execution_time_limit: None,
        repetition: None,
    };
    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read EventTrigger xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"Subscription" => {
                    event
                        .subscription
                        .push(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                b"Delay" => {
                    event.delay =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                b"MatchingElement" => {
                    event.matching_element =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                b"PeriodOfOccurrence" => {
                    event.period_of_occurrence =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                b"NumberOfOccurrences" => {
                    event.number_of_occurrences = Some(
                        str::parse(&reader.read_text(tag.name()).unwrap_or_default())
                            .unwrap_or_default(),
                    );
                }
                b"ValueQueries" => event.value_queries = Some(process_event_values(reader)),
                _ => process_common(&mut common, &tag.name(), reader),
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"TimeTrigger" {
                    break;
                }
            }
            _ => (),
        }
    }

    event.common = Some(common);
    info.event.push(event);
}

/// Parse `LogonTrigger` options
fn process_logon(info: &mut Triggers, reader: &mut Reader<&[u8]>) {
    let mut logon = LogonTrigger {
        common: None,
        user_id: None,
        delay: None,
    };

    let mut common = BaseTriggers {
        id: None,
        start_boundary: None,
        end_boundary: None,
        enabled: None,
        execution_time_limit: None,
        repetition: None,
    };
    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read LogonTrigger xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"UserId" => {
                    logon.user_id =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                b"Delay" => {
                    logon.delay =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                _ => process_common(&mut common, &tag.name(), reader),
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"LogonTrigger" {
                    break;
                }
            }
            _ => (),
        }
    }

    logon.common = Some(common);
    info.logon.push(logon);
}

/// Parse `SessionTrigger` options
fn process_session(info: &mut Triggers, reader: &mut Reader<&[u8]>) {
    let mut session = SessionTrigger {
        common: None,
        delay: None,
        user_id: None,
        state_change: None,
    };

    let mut common = BaseTriggers {
        id: None,
        start_boundary: None,
        end_boundary: None,
        enabled: None,
        execution_time_limit: None,
        repetition: None,
    };
    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read SessionStateChangeTrigger xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"Delay" => {
                    session.delay =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                b"StateChange" => {
                    session.state_change =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                b"UserId" => {
                    session.user_id =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                _ => process_common(&mut common, &tag.name(), reader),
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"SessionStateChangeTrigger" {
                    break;
                }
            }
            _ => (),
        }
    }

    session.common = Some(common);
    info.session.push(session);
}

/// Parse `CalendarTrigger` options
fn process_calendar(info: &mut Triggers, reader: &mut Reader<&[u8]>) {
    let mut cal = CalendarTrigger {
        common: None,
        schedule_by_day: None,
        schedule_by_month: None,
        schedule_by_month_day_of_week: None,
        schedule_by_week: None,
        random_delay: None,
    };

    let mut common = BaseTriggers {
        id: None,
        start_boundary: None,
        end_boundary: None,
        enabled: None,
        execution_time_limit: None,
        repetition: None,
    };
    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read CalendarTrigger xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"RandomDelay" => {
                    cal.random_delay =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                b"ScheduleByDay" => cal.schedule_by_day = Some(process_cal_day(reader)),
                b"ScheduleByWeek" => cal.schedule_by_week = Some(process_cal_week(reader)),
                b"ScheduleByMonth" => cal.schedule_by_month = Some(process_cal_month(reader)),
                b"ScheduleByMonthDayOfWeek" => {
                    cal.schedule_by_month_day_of_week = Some(process_cal_month_day_week(reader));
                }
                _ => process_common(&mut common, &tag.name(), reader),
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"CalendarTrigger" {
                    break;
                }
            }
            _ => (),
        }
    }

    cal.common = Some(common);
    info.calendar.push(cal);
}

/// Parse common values between all triggers
fn process_common(common: &mut BaseTriggers, name: &QName<'_>, reader: &mut Reader<&[u8]>) {
    match name.as_ref() {
        b"id" => {
            common.id = Some(reader.read_text(*name).unwrap_or_default().to_string());
        }
        b"StartBoundary" => {
            common.start_boundary = Some(reader.read_text(*name).unwrap_or_default().to_string());
        }
        b"EndBoundary" => {
            common.end_boundary = Some(reader.read_text(*name).unwrap_or_default().to_string());
        }
        b"ExecutionTimeLimit" => {
            common.execution_time_limit =
                Some(reader.read_text(*name).unwrap_or_default().to_string());
        }
        b"Enabled" => {
            common.enabled =
                Some(str::parse(&reader.read_text(*name).unwrap_or_default()).unwrap_or_default());
        }
        b"Repetition" => {
            process_repetition(common, reader);
        }
        _ => (),
    }
}

/// Parse repetition values
fn process_repetition(common: &mut BaseTriggers, reader: &mut Reader<&[u8]>) {
    let mut repetition = Repetition {
        interval: String::new(),
        duration: None,
        stop_at_duration_end: None,
    };

    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read Repetition xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"Interval" => {
                    repetition.interval =
                        reader.read_text(tag.name()).unwrap_or_default().to_string();
                }
                b"Duration" => {
                    repetition.duration =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                b"StopAtDurationEnd" => {
                    repetition.stop_at_duration_end = Some(
                        str::parse(&reader.read_text(tag.name()).unwrap_or_default())
                            .unwrap_or_default(),
                    );
                }
                _ => break,
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"Repetition" {
                    break;
                }
            }
            _ => (),
        }
    }
    common.repetition = Some(repetition);
}

/// Process the Values in `ValueQueries` in `EventTriggers`
fn process_event_values(reader: &mut Reader<&[u8]>) -> Vec<String> {
    let mut values = Vec::new();
    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read EventTrigger Values xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"Value" => {
                    values.push(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                _ => break,
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"ValueQueries" {
                    break;
                }
            }
            _ => (),
        }
    }
    values
}

/// Parse Day information from `CalendarTrigger`
fn process_cal_day(reader: &mut Reader<&[u8]>) -> ByDay {
    let mut day = ByDay {
        days_interval: None,
    };
    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read Calendar ByDay Values xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"DaysInterval" => {
                    day.days_interval = Some(
                        str::parse(&reader.read_text(tag.name()).unwrap_or_default())
                            .unwrap_or_default(),
                    );
                }
                _ => break,
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"ScheduleByDay" {
                    break;
                }
            }
            _ => (),
        }
    }
    day
}

/// Parse Week information from `CalendarTrigger`
fn process_cal_week(reader: &mut Reader<&[u8]>) -> ByWeek {
    let mut week = ByWeek {
        weeks_interval: None,
        days_of_week: None,
    };
    let mut days = Vec::new();
    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read Calendar ByWeek Values xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"WeeksInterval" => {
                    week.weeks_interval = Some(
                        str::parse(&reader.read_text(tag.name()).unwrap_or_default())
                            .unwrap_or_default(),
                    );
                }
                b"DaysOfWeek" => (),
                // Push days of week values. Ex: Monday, Tuesday, etc
                _ => days.push(extract_utf8_string(tag.name().0)),
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"ScheduleByWeek" {
                    break;
                }
            }
            _ => (),
        }
    }
    week.days_of_week = Some(days);
    week
}

/// Parse Month information from `CalendarTrigger`
fn process_cal_month(reader: &mut Reader<&[u8]>) -> ByMonth {
    let mut month = ByMonth {
        days_of_month: None,
        months: None,
    };
    let mut days = Vec::new();
    let mut months = Vec::new();
    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read Calendar ByWeek Values xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"Months" => (),
                b"DaysOfMonth" => {
                    days.push(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                // Push Months. Ex: July, Auguest, etc
                _ => months.push(extract_utf8_string(tag.name().0)),
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"ScheduleByMonth" {
                    break;
                }
            }
            _ => (),
        }
    }
    month.days_of_month = Some(days);
    month.months = Some(months);

    month
}

/// Parse Month-Day-Week information from `CalendarTrigger`
fn process_cal_month_day_week(reader: &mut Reader<&[u8]>) -> ByMonthDayWeek {
    let mut month = ByMonthDayWeek {
        weeks: None,
        days_of_week: None,
        months: None,
    };
    let mut days = Vec::new();
    let mut months = Vec::new();
    let mut weeks = Vec::new();

    let mut value = "";
    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read Calendar ByWeek Values xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"Months" => value = "months",
                b"DaysOfWeek" => value = "days",
                b"Weeks" => value = "weeks",
                _ => {
                    if value == "months" {
                        months.push(extract_utf8_string(tag.name().0));
                    } else if value == "weeks" {
                        weeks.push(reader.read_text(tag.name()).unwrap_or_default().to_string());
                    } else if value == "days" {
                        days.push(extract_utf8_string(tag.name().0));
                    }
                }
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"ScheduleByMonthDayOfWeek" {
                    break;
                }
            }
            _ => (),
        }
    }
    month.days_of_week = Some(days);
    month.weeks = Some(weeks);
    month.months = Some(months);

    month
}

#[cfg(test)]
mod tests {
    use super::parse_trigger;
    use crate::artifacts::os::windows::tasks::schemas::triggers::{
        BaseTriggers, Triggers, process_boot, process_cal_day, process_cal_month,
        process_cal_month_day_week, process_cal_week, process_calendar, process_common,
        process_event, process_event_values, process_idle, process_logon, process_notification,
        process_repetition, process_session, process_time,
    };
    use quick_xml::{Reader, events::Event};

    #[test]
    fn test_parse_trigger() {
        let xml = r#"
        <CalendarTrigger>
          <StartBoundary>2019-10-21T12:26:22</StartBoundary>
          <ScheduleByDay>
            <DaysInterval>1</DaysInterval>
          </ScheduleByDay>
        </CalendarTrigger>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let result = parse_trigger(&mut reader);
        assert_eq!(
            result.calendar[0]
                .common
                .as_ref()
                .unwrap()
                .start_boundary
                .as_ref()
                .unwrap(),
            "2019-10-21T12:26:22"
        );
    }

    #[test]
    fn test_process_boot() {
        let xml = r#"
          <id>asdfsadfsadfsadf</id>
          <Delay>20</Delay>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut result = Triggers {
            boot: Vec::new(),
            registration: Vec::new(),
            idle: Vec::new(),
            time: Vec::new(),
            event: Vec::new(),
            logon: Vec::new(),
            session: Vec::new(),
            calendar: Vec::new(),
            wnf: Vec::new(),
        };
        process_boot(&mut result, &mut reader, true);
        assert_eq!(
            result.boot[0].common.as_ref().unwrap().id.as_ref().unwrap(),
            "asdfsadfsadfsadf"
        );
    }

    #[test]
    fn test_process_idle() {
        let xml = r#"
          <ExecutionTimeLimit>10D</ExecutionTimeLimit>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut result = Triggers {
            boot: Vec::new(),
            registration: Vec::new(),
            idle: Vec::new(),
            time: Vec::new(),
            event: Vec::new(),
            logon: Vec::new(),
            session: Vec::new(),
            calendar: Vec::new(),
            wnf: Vec::new(),
        };
        process_idle(&mut result, &mut reader);
        assert_eq!(
            result.idle[0]
                .common
                .as_ref()
                .unwrap()
                .execution_time_limit
                .as_ref()
                .unwrap(),
            "10D"
        );
    }

    #[test]
    fn test_process_notification() {
        let xml = r#"
          <Delay>10D</Delay>
          <StateName>asdfasdfasdfsadf</StateName>
          <Data>11111</Data>
          <DataOffset>4</DataOffset>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut result = Triggers {
            boot: Vec::new(),
            registration: Vec::new(),
            idle: Vec::new(),
            time: Vec::new(),
            event: Vec::new(),
            logon: Vec::new(),
            session: Vec::new(),
            calendar: Vec::new(),
            wnf: Vec::new(),
        };
        process_notification(&mut result, &mut reader);
        assert_eq!(result.wnf[0].state_name, "asdfasdfasdfsadf");
    }

    #[test]
    fn test_process_time() {
        let xml = r#"
          <RandomDelay>PTOM</RandomDelay>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut result = Triggers {
            boot: Vec::new(),
            registration: Vec::new(),
            idle: Vec::new(),
            time: Vec::new(),
            event: Vec::new(),
            logon: Vec::new(),
            session: Vec::new(),
            calendar: Vec::new(),
            wnf: Vec::new(),
        };
        process_time(&mut result, &mut reader);
        assert_eq!(result.time[0].random_delay.as_ref().unwrap(), "PTOM");
    }

    #[test]
    fn test_process_event() {
        let xml = r#"
          <Delay>PTOM</Delay>
          <Subscription>rusty</Subscription>
          <Repetition>
            <Interval>10</Interval>
            </Repetition>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut result = Triggers {
            boot: Vec::new(),
            registration: Vec::new(),
            idle: Vec::new(),
            time: Vec::new(),
            event: Vec::new(),
            logon: Vec::new(),
            session: Vec::new(),
            calendar: Vec::new(),
            wnf: Vec::new(),
        };
        process_event(&mut result, &mut reader);
        assert_eq!(result.event[0].subscription[0], "rusty");
    }

    #[test]
    fn test_process_logon() {
        let xml = r#"
          <UserId>bob</UserId>
          <Delay>PTOM</Delay>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut result = Triggers {
            boot: Vec::new(),
            registration: Vec::new(),
            idle: Vec::new(),
            time: Vec::new(),
            event: Vec::new(),
            logon: Vec::new(),
            session: Vec::new(),
            calendar: Vec::new(),
            wnf: Vec::new(),
        };
        process_logon(&mut result, &mut reader);
        assert_eq!(result.logon[0].user_id.as_ref().unwrap(), "bob");
    }

    #[test]
    fn test_process_session() {
        let xml = r#"
          <UserId>PTOM</UserId>'
          <Delay>10</Delay>
          <StateChange>ConsoleConnect</StateChange>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut result = Triggers {
            boot: Vec::new(),
            registration: Vec::new(),
            idle: Vec::new(),
            time: Vec::new(),
            event: Vec::new(),
            logon: Vec::new(),
            session: Vec::new(),
            calendar: Vec::new(),
            wnf: Vec::new(),
        };
        process_session(&mut result, &mut reader);
        assert_eq!(
            result.session[0].state_change.as_ref().unwrap(),
            "ConsoleConnect"
        );
    }

    #[test]
    fn test_process_calendar() {
        let xml = r#"
        <CalendarTrigger>
          <StartBoundary>2019-10-21T12:26:22</StartBoundary>
          <ScheduleByDay>
            <DaysInterval>1</DaysInterval>
          </ScheduleByDay>
        </CalendarTrigger>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut result = Triggers {
            boot: Vec::new(),
            registration: Vec::new(),
            idle: Vec::new(),
            time: Vec::new(),
            event: Vec::new(),
            logon: Vec::new(),
            session: Vec::new(),
            calendar: Vec::new(),
            wnf: Vec::new(),
        };
        process_calendar(&mut result, &mut reader);
        assert_eq!(
            result.calendar[0]
                .schedule_by_day
                .as_ref()
                .unwrap()
                .days_interval
                .unwrap(),
            1
        );
    }

    #[test]
    fn test_process_common() {
        let xml = r#"
          <EndBoundary>rusty</EndBoundary>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut result = BaseTriggers {
            id: None,
            start_boundary: None,
            end_boundary: None,
            enabled: None,
            execution_time_limit: None,
            repetition: None,
        };
        loop {
            match reader.read_event() {
                Ok(Event::Start(tag)) => match tag.name().as_ref() {
                    _ => process_common(&mut result, &tag.name(), &mut reader),
                },
                _ => break,
            }
        }
        assert_eq!(result.end_boundary.as_ref().unwrap(), "rusty");
    }

    #[test]
    fn test_process_repetition() {
        let xml = r#"
          <Interval>10</Interval>
          <Duration>20</Duration>
            <StopAtDurationEnd>true</StopAtDurationEnd>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut result = BaseTriggers {
            id: None,
            start_boundary: None,
            end_boundary: None,
            enabled: None,
            execution_time_limit: None,
            repetition: None,
        };
        process_repetition(&mut result, &mut reader);
        assert!(
            result
                .repetition
                .as_ref()
                .unwrap()
                .stop_at_duration_end
                .as_ref()
                .unwrap()
        );
    }

    #[test]
    fn test_process_event_values() {
        let xml = r#"
          <Value>10</Value>
          <Value>20</Value>
            <Value>true</Value>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let values = process_event_values(&mut reader);
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn test_process_cal_day() {
        let xml = r#"
            <DaysInterval>1</DaysInterval>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let result = process_cal_day(&mut reader);
        assert_eq!(result.days_interval.unwrap(), 1);
    }

    #[test]
    fn test_process_cal_week() {
        let xml = r#"
            <WeeksInterval>1</WeeksInterval>
            <Monday></Monday>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let result = process_cal_week(&mut reader);
        assert_eq!(result.days_of_week.unwrap()[0], "Monday");
    }

    #[test]
    fn test_process_cal_month() {
        let xml = r#"
            <DaysOfMonth>1</DaysOfMonth>
            <July></July>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let result = process_cal_month(&mut reader);
        assert_eq!(result.months.unwrap()[0], "July");
    }

    #[test]
    fn test_process_cal_month_day_week() {
        let xml = r#"
            <Weeks>
              <Week>Last</Week>
            </Weeks>
            <DaysOfWeek>
              <Tuesday></Tuesday>
            </DaysOfWeek>
            <Months>
              <July></July>
            </Months>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let result = process_cal_month_day_week(&mut reader);
        assert_eq!(result.months.unwrap()[0], "July");
    }
}
