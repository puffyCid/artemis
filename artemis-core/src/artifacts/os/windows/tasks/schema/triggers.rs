use log::error;
use quick_xml::{events::Event, Reader};

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

struct BaseTriggers {
    id: Option<String>,
    start_boundary: Option<String>,
    end_boundary: Option<String>,
    enabled: Option<bool>,
    execution_time_limit: Option<String>,
    repetition: Option<Repetition>,
}

struct Repetition {
    interval: String,
    duration: Option<String>,
    stop_at_duration_end: Option<bool>,
}

struct BootTrigger {
    common: Option<BaseTriggers>,
    delay: Option<String>,
}

struct IdleTrigger {
    common: Option<BaseTriggers>,
}

struct TimeTrigger {
    common: Option<BaseTriggers>,
    random_delay: String,
}

struct EventTrigger {
    common: Option<BaseTriggers>,
    subscription: String,
    delay: String,
    number_of_occurrences: u8,
    period_of_occurrence: String,
    matching_element: String,
    value_queries: Vec<String>,
}

struct LogonTrigger {
    common: Option<BaseTriggers>,
    user_id: String,
    delay: String,
}

struct SessionTrigger {
    common: Option<BaseTriggers>,
    user_id: String,
    delay: String,
    state_change: String,
}

struct CalendarTrigger {
    common: Option<BaseTriggers>,
    schedule_by_day: Option<ByDay>,
    schedule_by_week: Option<ByWeek>,
    schedule_by_month: Option<ByMonth>,
    schedule_by_month_day_of_week: Option<ByMonthDayWeek>,
}

struct ByDay {
    days_interval: u16,
}

struct ByWeek {
    weeks_interval: u8,
    days_of_week: Vec<String>,
}

struct ByMonth {
    days_of_month: Vec<String>,
    months: Vec<String>,
}

struct ByMonthDayWeek {
    weeks: Vec<String>,
    days_of_week: Vec<String>,
    months: Vec<String>,
    random_delay: String,
}

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

    let mut trig_type = "";
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
                b"IdleTrigger" => trig_type = "IdleTrigger",
                b"TimeTrigger" => trig_type = "TimeTrigger",
                b"EventTrigger" => trig_type = "EventTrigger",
                b"LogonTrigger" => trig_type = "LogonTrigger",
                b"SessionStateChangeTrigger" => trig_type = "SessionStateChangeTrigger",
                b"CalendarTrigger" => trig_type = "CalendarTrigger",
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
                b"id" => {
                    common.id = Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"StartBoundary" => {
                    common.start_boundary =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"EndBoundary" => {
                    common.end_boundary =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"ExecutionTimeLimit" => {
                    common.execution_time_limit =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"Enabled" => {
                    common.enabled = Some(
                        str::parse(&reader.read_text(tag.name()).unwrap_or_default().to_string())
                            .unwrap_or_default(),
                    )
                }
                b"Repetition" => process_repetition(&mut common, reader),
                b"Delay" => {
                    boot.delay = Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                _ => break,
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

fn process_repetition(common: &mut BaseTriggers, reader: &mut Reader<&[u8]>) {
    let mut repetitiion = Repetition {
        interval: String::new(),
        duration: None,
        stop_at_duration_end: None,
    };

    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read BootTrigger xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"Interval" => {
                    repetitiion.interval =
                        reader.read_text(tag.name()).unwrap_or_default().to_string()
                }
                b"Duration" => {
                    repetitiion.duration =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"StopAtDurationEnd" => {
                    repetitiion.stop_at_duration_end = Some(
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
}
