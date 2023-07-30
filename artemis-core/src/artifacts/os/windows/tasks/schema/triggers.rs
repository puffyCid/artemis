use log::error;
use quick_xml::{events::Event, name::QName, Reader};

use crate::utils::strings::{extract_utf16_string, extract_utf8_string};

#[derive(Debug)]
pub(crate) struct Triggers {
    boot: Option<BootTrigger>,
    registration: Option<BootTrigger>,
    idle: Option<IdleTrigger>,
    time: Option<TimeTrigger>,
    event: Option<EventTrigger>,
    logon: Option<LogonTrigger>,
    session: Option<SessionTrigger>,
    calendar: Option<CalendarTrigger>,
}

#[derive(Debug)]
struct BaseTriggers {
    id: Option<String>,
    start_boundary: Option<String>,
    end_boundary: Option<String>,
    enabled: Option<bool>,
    execution_time_limit: Option<String>,
    repetition: Option<Repetition>,
}

#[derive(Debug)]
struct Repetition {
    interval: String,
    duration: Option<String>,
    stop_at_duration_end: Option<bool>,
}

#[derive(Debug)]
struct BootTrigger {
    common: Option<BaseTriggers>,
    delay: Option<String>,
}

#[derive(Debug)]
struct IdleTrigger {
    common: Option<BaseTriggers>,
}

#[derive(Debug)]
struct TimeTrigger {
    common: Option<BaseTriggers>,
    random_delay: Option<String>,
}

#[derive(Debug)]
struct EventTrigger {
    common: Option<BaseTriggers>,
    subscription: String,
    delay: Option<String>,
    number_of_occurrences: Option<u8>,
    period_of_occurrence: Option<String>,
    matching_element: Option<String>,
    value_queries: Option<Vec<String>>,
}

#[derive(Debug)]
struct LogonTrigger {
    common: Option<BaseTriggers>,
    user_id: Option<String>,
    delay: Option<String>,
}

#[derive(Debug)]
struct SessionTrigger {
    common: Option<BaseTriggers>,
    user_id: Option<String>,
    delay: Option<String>,
    state_change: Option<String>,
}

#[derive(Debug)]
struct CalendarTrigger {
    common: Option<BaseTriggers>,
    random_delay: Option<String>,
    schedule_by_day: Option<ByDay>,
    schedule_by_week: Option<ByWeek>,
    schedule_by_month: Option<ByMonth>,
    schedule_by_month_day_of_week: Option<ByMonthDayWeek>,
}

#[derive(Debug)]
struct ByDay {
    days_interval: Option<u16>,
}

#[derive(Debug)]
struct ByWeek {
    weeks_interval: Option<u8>,
    days_of_week: Option<Vec<String>>,
}

#[derive(Debug)]
struct ByMonth {
    days_of_month: Option<Vec<String>>,
    months: Option<Vec<String>>,
}

#[derive(Debug)]
struct ByMonthDayWeek {
    weeks: Option<Vec<String>>,
    days_of_week: Option<Vec<String>>,
    months: Option<Vec<String>>,
}

/// Parse all Task Trigger options.
pub(crate) fn parse_trigger(reader: &mut Reader<&[u8]>) -> Triggers {
    let mut info = Triggers {
        boot: None,
        registration: None,
        idle: None,
        time: None,
        event: None,
        logon: None,
        session: None,
        calendar: None,
    };

    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read Triggers xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"BootTrigger" => process_boot(&mut info, reader, &true),
                b"RegistrationTrigger" => process_boot(&mut info, reader, &false),
                b"IdleTrigger" => process_idle(&mut info, reader),
                b"TimeTrigger" => process_time(&mut info, reader),
                b"EventTrigger" => process_event(&mut info, reader),
                b"LogonTrigger" => process_logon(&mut info, reader),
                b"SessionStateChangeTrigger" => process_session(&mut info, reader),
                b"CalendarTrigger" => process_calendar(&mut info, reader),
                _ => break,
            },
            Ok(Event::End(tag)) => match tag.name().as_ref() {
                b"Triggers" => break,
                _ => continue,
            },
            _ => (),
        }
    }

    info
}

/// Parse BookTrigger options
fn process_boot(info: &mut Triggers, reader: &mut Reader<&[u8]>, is_boot: &bool) {
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
                    boot.delay = Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                _ => process_common(&mut common, &tag.name(), reader),
            },
            Ok(Event::End(tag)) => match tag.name().as_ref() {
                b"BootTrigger" => break,
                b"RegistrationTrigger" => break,
                _ => continue,
            },
            _ => (),
        }
    }

    boot.common = Some(common);
    if *is_boot {
        info.boot = Some(boot)
    } else {
        info.registration = Some(boot)
    }
}

/// Parse IdleTrigger options
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
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                _ => process_common(&mut common, &tag.name(), reader),
            },
            Ok(Event::End(tag)) => match tag.name().as_ref() {
                b"IdleTrigger" => break,
                _ => continue,
            },
            _ => (),
        }
    }
    let idle = IdleTrigger {
        common: Some(common),
    };
    info.idle = Some(idle)
}

/// Parse TimeTrigger options
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
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                _ => process_common(&mut common, &tag.name(), reader),
            },
            Ok(Event::End(tag)) => match tag.name().as_ref() {
                b"TimeTrigger" => break,
                _ => continue,
            },
            _ => (),
        }
    }

    time.common = Some(common);
    info.time = Some(time);
}

/// Parse EventTrigger options
fn process_event(info: &mut Triggers, reader: &mut Reader<&[u8]>) {
    let mut event = EventTrigger {
        common: None,
        subscription: String::new(),
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
                    event.subscription =
                        reader.read_text(tag.name()).unwrap_or_default().to_string()
                }
                b"Delay" => {
                    event.delay = Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"MatchingElement" => {
                    event.matching_element =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"PeriodOfOccurrence" => {
                    event.period_of_occurrence =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"NumberOfOccurrences" => {
                    event.number_of_occurrences = Some(
                        str::parse(&reader.read_text(tag.name()).unwrap_or_default().to_string())
                            .unwrap_or_default(),
                    )
                }
                b"ValueQueries" => event.value_queries = Some(process_event_values(reader)),
                _ => process_common(&mut common, &tag.name(), reader),
            },
            Ok(Event::End(tag)) => match tag.name().as_ref() {
                b"TimeTrigger" => break,
                _ => continue,
            },
            _ => (),
        }
    }

    event.common = Some(common);
    info.event = Some(event);
}

/// Parse LogonTrigger options
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
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"Delay" => {
                    logon.delay = Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                _ => process_common(&mut common, &tag.name(), reader),
            },
            Ok(Event::End(tag)) => match tag.name().as_ref() {
                b"LogonTrigger" => break,
                _ => continue,
            },
            _ => (),
        }
    }

    logon.common = Some(common);
    info.logon = Some(logon);
}

/// Parse SessionTrigger options
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
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"StateChange" => {
                    session.state_change =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"UserId" => {
                    session.user_id =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                _ => process_common(&mut common, &tag.name(), reader),
            },
            Ok(Event::End(tag)) => match tag.name().as_ref() {
                b"SessionStateChangeTrigger" => break,
                _ => continue,
            },
            _ => (),
        }
    }

    session.common = Some(common);
    info.session = Some(session);
}

/// Parse CalendarTrigger options
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
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"ScheduleByDay" => cal.schedule_by_day = Some(process_cal_day(reader)),
                b"ScheduleByWeek" => cal.schedule_by_week = Some(process_cal_week(reader)),
                b"ScheduleByMonth" => cal.schedule_by_month = Some(process_cal_month(reader)),
                b"ScheduleByMonthDayOfWeek" => {
                    cal.schedule_by_month_day_of_week = Some(process_cal_month_day_week(reader))
                }
                _ => process_common(&mut common, &tag.name(), reader),
            },
            Ok(Event::End(tag)) => match tag.name().as_ref() {
                b"CalendarTrigger" => break,
                _ => continue,
            },
            _ => (),
        }
    }

    cal.common = Some(common);
    info.calendar = Some(cal);
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
            common.enabled = Some(
                str::parse(&reader.read_text(*name).unwrap_or_default().to_string())
                    .unwrap_or_default(),
            );
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
                        reader.read_text(tag.name()).unwrap_or_default().to_string()
                }
                b"Duration" => {
                    repetition.duration =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"StopAtDurationEnd" => {
                    repetition.stop_at_duration_end = Some(
                        str::parse(&reader.read_text(tag.name()).unwrap_or_default().to_string())
                            .unwrap_or_default(),
                    )
                }
                _ => break,
            },
            Ok(Event::End(tag)) => match tag.name().as_ref() {
                b"Repetition" => break,
                _ => continue,
            },
            _ => (),
        }
    }
    common.repetition = Some(repetition)
}

/// Process the Values in ValueQueries in EventTriggers
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
                    values.push(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                _ => break,
            },
            Ok(Event::End(tag)) => match tag.name().as_ref() {
                b"ValueQueries" => break,
                _ => continue,
            },
            _ => (),
        }
    }
    values
}

/// Parse Day information from CalendarTrigger
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
                        str::parse(&reader.read_text(tag.name()).unwrap_or_default().to_string())
                            .unwrap_or_default(),
                    )
                }
                _ => break,
            },
            Ok(Event::End(tag)) => match tag.name().as_ref() {
                b"ScheduleByDay" => break,
                _ => continue,
            },
            _ => (),
        }
    }
    day
}

/// Parse Week information from CalendarTrigger
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
                        str::parse(&reader.read_text(tag.name()).unwrap_or_default().to_string())
                            .unwrap_or_default(),
                    )
                }
                b"DaysOfWeek" => continue,
                // Push days of week values. Ex: Monday, Tuesday, etc
                _ => days.push(extract_utf8_string(tag.name().0)),
            },
            Ok(Event::End(tag)) => match tag.name().as_ref() {
                b"ScheduleByWeek" => break,
                _ => continue,
            },
            _ => (),
        }
    }
    week.days_of_week = Some(days);
    week
}

/// Parse Month information from CalendarTrigger
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
                b"Months" => continue,
                b"DaysOfMonth" => {
                    days.push(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                // Push Months. Ex: July, Auguest, etc
                _ => months.push(extract_utf8_string(tag.name().0)),
            },
            Ok(Event::End(tag)) => match tag.name().as_ref() {
                b"ScheduleByMonth" => break,
                _ => continue,
            },
            _ => (),
        }
    }
    month.days_of_month = Some(days);
    month.months = Some(months);

    month
}

/// Parse Month-Day-Week information from CalendarTrigger
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
                        weeks.push(reader.read_text(tag.name()).unwrap_or_default().to_string())
                    } else if value == "days" {
                        days.push(extract_utf8_string(tag.name().0))
                    }
                }
            },
            Ok(Event::End(tag)) => match tag.name().as_ref() {
                b"ScheduleByMonthDayOfWeek" => break,
                _ => continue,
            },
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
    use crate::artifacts::os::windows::tasks::schema::triggers::{
        process_boot, process_cal_day, process_cal_month, process_cal_month_day_week,
        process_cal_week, process_calendar, process_common, process_event, process_event_values,
        process_idle, process_logon, process_repetition, process_session, process_time,
        BaseTriggers, Triggers,
    };
    use quick_xml::{events::Event, Reader};

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
        reader.trim_text(true);

        let result = parse_trigger(&mut reader);
        assert_eq!(
            result
                .calendar
                .unwrap()
                .common
                .unwrap()
                .start_boundary
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
        reader.trim_text(true);

        let mut result = Triggers {
            boot: None,
            registration: None,
            idle: None,
            time: None,
            event: None,
            logon: None,
            session: None,
            calendar: None,
        };
        process_boot(&mut result, &mut reader, &true);
        assert_eq!(
            result.boot.unwrap().common.unwrap().id.unwrap(),
            "asdfsadfsadfsadf"
        );
    }

    #[test]
    fn test_process_idle() {
        let xml = r#"
          <ExecutionTimeLimit>10D</ExecutionTimeLimit>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);

        let mut result = Triggers {
            boot: None,
            registration: None,
            idle: None,
            time: None,
            event: None,
            logon: None,
            session: None,
            calendar: None,
        };
        process_idle(&mut result, &mut reader);
        assert_eq!(
            result
                .idle
                .unwrap()
                .common
                .unwrap()
                .execution_time_limit
                .unwrap(),
            "10D"
        );
    }

    #[test]
    fn test_process_time() {
        let xml = r#"
          <RandomDelay>PTOM</RandomDelay>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);

        let mut result = Triggers {
            boot: None,
            registration: None,
            idle: None,
            time: None,
            event: None,
            logon: None,
            session: None,
            calendar: None,
        };
        process_time(&mut result, &mut reader);
        assert_eq!(result.time.unwrap().random_delay.unwrap(), "PTOM");
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
        reader.trim_text(true);

        let mut result = Triggers {
            boot: None,
            registration: None,
            idle: None,
            time: None,
            event: None,
            logon: None,
            session: None,
            calendar: None,
        };
        process_event(&mut result, &mut reader);
        assert_eq!(result.event.unwrap().subscription, "rusty");
    }

    #[test]
    fn test_process_logon() {
        let xml = r#"
          <UserId>bob</UserId>
          <Delay>PTOM</Delay>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);

        let mut result = Triggers {
            boot: None,
            registration: None,
            idle: None,
            time: None,
            event: None,
            logon: None,
            session: None,
            calendar: None,
        };
        process_logon(&mut result, &mut reader);
        assert_eq!(result.logon.unwrap().user_id.unwrap(), "bob");
    }

    #[test]
    fn test_process_session() {
        let xml = r#"
          <UserId>PTOM</UserId>'
          <Delay>10</Delay>
          <StateChange>ConsoleConnect</StateChange>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);

        let mut result = Triggers {
            boot: None,
            registration: None,
            idle: None,
            time: None,
            event: None,
            logon: None,
            session: None,
            calendar: None,
        };
        process_session(&mut result, &mut reader);
        assert_eq!(
            result.session.unwrap().state_change.unwrap(),
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
        reader.trim_text(true);

        let mut result = Triggers {
            boot: None,
            registration: None,
            idle: None,
            time: None,
            event: None,
            logon: None,
            session: None,
            calendar: None,
        };
        process_calendar(&mut result, &mut reader);
        assert_eq!(
            result
                .calendar
                .unwrap()
                .schedule_by_day
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
        reader.trim_text(true);

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
        assert_eq!(result.end_boundary.unwrap(), "rusty");
    }

    #[test]
    fn test_process_repetition() {
        let xml = r#"
          <Interval>10</Interval>
          <Duration>20</Duration>
            <StopAtDurationEnd>true</StopAtDurationEnd>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);

        let mut result = BaseTriggers {
            id: None,
            start_boundary: None,
            end_boundary: None,
            enabled: None,
            execution_time_limit: None,
            repetition: None,
        };
        process_repetition(&mut result, &mut reader);
        assert!(result.repetition.unwrap().stop_at_duration_end.unwrap());
    }

    #[test]
    fn test_process_event_values() {
        let xml = r#"
          <Value>10</Value>
          <Value>20</Value>
            <Value>true</Value>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);

        let values = process_event_values(&mut reader);
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn test_process_cal_day() {
        let xml = r#"
            <DaysInterval>1</DaysInterval>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);

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
        reader.trim_text(true);

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
        reader.trim_text(true);

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
        reader.trim_text(true);

        let result = process_cal_month_day_week(&mut reader);
        assert_eq!(result.months.unwrap()[0], "July");
    }
}
