use crate::{
    artifacts::os::windows::srum::error::SrumError,
    utils::time::{filetime_to_unixepoch, unixepoch_to_iso},
};
use common::windows::{AppTimelineInfo, AppVfu, ApplicationInfo, TableDump};
use log::error;
use serde_json::Value;
use std::collections::HashMap;

/// Parse the application table from SRUM
pub(crate) fn parse_application(
    column_rows: &[Vec<TableDump>],
    lookups: &HashMap<String, String>,
) -> Result<Value, SrumError> {
    let mut app_vec: Vec<ApplicationInfo> = Vec::new();
    for rows in column_rows {
        let mut app = ApplicationInfo {
            auto_inc_id: 0,
            timestamp: String::new(),
            app_id: String::new(),
            user_id: String::new(),
            foreground_cycle_time: 0,
            background_cycle_time: 0,
            facetime: 0,
            foreground_context_switches: 0,
            background_context_switches: 0,
            foreground_bytes_read: 0,
            foreground_bytes_written: 0,
            foreground_num_read_operations: 0,
            foreground_num_write_options: 0,
            foreground_number_of_flushes: 0,
            background_bytes_read: 0,
            background_bytes_written: 0,
            background_num_read_operations: 0,
            background_num_write_operations: 0,
            background_number_of_flushes: 0,
        };

        for column in rows {
            match column.column_name.as_str() {
                "AutoIncId" => {
                    app.auto_inc_id = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "TimeStamp" => {
                    app.timestamp.clone_from(&column.column_data);
                }
                "AppId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        app.app_id.clone_from(value);
                        continue;
                    }
                    app.app_id.clone_from(&column.column_data);
                }
                "UserId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        app.user_id.clone_from(value);
                        continue;
                    }
                    app.user_id.clone_from(&column.column_data);
                }
                "ForegroundCycleTime" => {
                    app.foreground_cycle_time =
                        column.column_data.parse::<i64>().unwrap_or_default();
                }
                "BackgroundCycleTime" => {
                    app.background_cycle_time =
                        column.column_data.parse::<i64>().unwrap_or_default();
                }
                "FaceTime" => app.facetime = column.column_data.parse::<i64>().unwrap_or_default(),
                "ForegroundContextSwitches" => {
                    app.foreground_context_switches =
                        column.column_data.parse::<i32>().unwrap_or_default();
                }
                "BackgroundContextSwitches" => {
                    app.background_context_switches =
                        column.column_data.parse::<i32>().unwrap_or_default();
                }
                "ForegroundBytesRead" => {
                    app.foreground_bytes_read =
                        column.column_data.parse::<i64>().unwrap_or_default();
                }
                "ForegroundBytesWritten" => {
                    app.foreground_bytes_written =
                        column.column_data.parse::<i64>().unwrap_or_default();
                }
                "ForegroundNumReadOperations" => {
                    app.foreground_num_read_operations =
                        column.column_data.parse::<i32>().unwrap_or_default();
                }
                "ForegroundNumWriteOperations" => {
                    app.foreground_num_write_options =
                        column.column_data.parse::<i32>().unwrap_or_default();
                }
                "ForegroundNumberOfFlushes" => {
                    app.foreground_number_of_flushes =
                        column.column_data.parse::<i32>().unwrap_or_default();
                }
                "BackgroundBytesRead" => {
                    app.background_bytes_read =
                        column.column_data.parse::<i64>().unwrap_or_default();
                }
                "BackgroundBytesWritten" => {
                    app.background_bytes_written =
                        column.column_data.parse::<i64>().unwrap_or_default();
                }
                "BackgroundNumReadOperations" => {
                    app.background_num_read_operations =
                        column.column_data.parse::<i32>().unwrap_or_default();
                }
                "BackgroundNumWriteOperations" => {
                    app.background_num_write_operations =
                        column.column_data.parse::<i32>().unwrap_or_default();
                }
                "BackgroundNumberOfFlushes" => {
                    app.background_number_of_flushes =
                        column.column_data.parse::<i32>().unwrap_or_default();
                }

                _ => (),
            }
        }
        app_vec.push(app);
    }

    let serde_data_result = serde_json::to_value(&app_vec);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[srum] Failed to serialize SRUM application table: {err:?}");
            return Err(SrumError::Serialize);
        }
    };
    Ok(serde_data)
}

/// Parse the app timeline table from SRUM
pub(crate) fn parse_app_timeline(
    column_rows: &[Vec<TableDump>],
    lookups: &HashMap<String, String>,
) -> Result<Value, SrumError> {
    let mut energy_vec: Vec<AppTimelineInfo> = Vec::new();
    for rows in column_rows {
        let mut energy = AppTimelineInfo {
            auto_inc_id: 0,
            timestamp: String::new(),
            app_id: String::new(),
            user_id: String::new(),
            flags: 0,
            end_time: String::new(),
            duration_ms: 0,
            span_ms: 0,
            timeline_end: 0,
            in_focus_timeline: 0,
            user_input_timeline: 0,
            comp_rendered_timeline: 0,
            comp_dirtied_timeline: 0,
            comp_propagated_timeline: 0,
            audio_in_timeline: 0,
            audio_out_timeline: 0,
            cpu_timeline: 0,
            disk_timeline: 0,
            network_timeline: 0,
            mbb_timeline: 0,
            in_focus_s: 0,
            psm_foreground_s: 0,
            user_input_s: 0,
            comp_rendered_s: 0,
            comp_dirtied_s: 0,
            comp_propagated_s: 0,
            audio_in_s: 0,
            audio_out_s: 0,
            cycles: 0,
            cycles_breakdown: 0,
            cycles_attr: 0,
            cycles_attr_breakdown: 0,
            cycles_wob: 0,
            cycles_wob_breakdown: 0,
            disk_raw: 0,
            network_tail_raw: 0,
            network_bytes_raw: 0,
            mbb_tail_raw: 0,
            mbb_bytes_raw: 0,
            display_required_s: 0,
            display_required_timeline: 0,
            keyboard_input_timeline: 0,
            keyboard_input_s: 0,
            mouse_input_s: 0,
        };

        let null_values = ["3038287259199220266", "707406378"];
        for column in rows {
            // Sometimes SRUM values will be ******** which is Null
            if null_values.contains(&column.column_data.as_str()) {
                continue;
            }
            match column.column_name.as_str() {
                "AutoIncId" => {
                    energy.auto_inc_id = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "TimeStamp" => {
                    energy.timestamp.clone_from(&column.column_data);
                }
                "AppId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        energy.app_id.clone_from(value);
                        continue;
                    }
                    energy.app_id.clone_from(&column.column_data);
                }
                "UserId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        energy.user_id.clone_from(value);
                        continue;
                    }
                    energy.user_id.clone_from(&column.column_data);
                }
                "Flags" => energy.flags = column.column_data.parse::<i32>().unwrap_or_default(),
                "EndTime" => {
                    energy.end_time = unixepoch_to_iso(&filetime_to_unixepoch(
                        &column.column_data.parse::<u64>().unwrap_or_default(),
                    ));
                }
                "DurationMS" => {
                    energy.duration_ms = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "SpanMS" => energy.span_ms = column.column_data.parse::<i32>().unwrap_or_default(),
                "TimelineEnd" => {
                    energy.timeline_end = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "InFocusTimeline" => {
                    energy.in_focus_timeline =
                        column.column_data.parse::<i64>().unwrap_or_default();
                }
                "UserInputTimeline" => {
                    energy.user_input_timeline =
                        column.column_data.parse::<i64>().unwrap_or_default();
                }
                "CompRenderedTimeline" => {
                    energy.comp_rendered_timeline =
                        column.column_data.parse::<i64>().unwrap_or_default();
                }
                "CompDirtiedTimeline" => {
                    energy.comp_dirtied_timeline =
                        column.column_data.parse::<i64>().unwrap_or_default();
                }
                "CompPropagatedTimeline" => {
                    energy.comp_propagated_timeline =
                        column.column_data.parse::<i64>().unwrap_or_default();
                }
                "AudioInTimeline" => {
                    energy.audio_in_timeline =
                        column.column_data.parse::<i64>().unwrap_or_default();
                }
                "AudioOutTimeline" => {
                    energy.audio_out_timeline =
                        column.column_data.parse::<i64>().unwrap_or_default();
                }
                "CpuTimeline" => {
                    energy.cpu_timeline = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "DiskTimeline" => {
                    energy.disk_timeline = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "NetworkTimeline" => {
                    energy.network_timeline = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "MBBTimeline" => {
                    energy.mbb_timeline = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "InFocusS" => {
                    energy.in_focus_s = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "PSMForegroundS" => {
                    energy.psm_foreground_s = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "UserInputS" => {
                    energy.user_input_s = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "CompRenderedS" => {
                    energy.comp_rendered_s = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "CompDirtiedS" => {
                    energy.comp_dirtied_s = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "CompPropagatedS" => {
                    energy.comp_propagated_s =
                        column.column_data.parse::<i32>().unwrap_or_default();
                }
                "AudioInS" => {
                    energy.audio_in_s = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "AudioOutS" => {
                    energy.audio_out_s = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "Cycles" => energy.cycles = column.column_data.parse::<i64>().unwrap_or_default(),
                "CyclesBreakdown" => {
                    energy.cycles_breakdown = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "CyclesAttr" => {
                    energy.cycles_attr = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "CyclesAttrBreakdown" => {
                    energy.cycles_attr_breakdown =
                        column.column_data.parse::<i64>().unwrap_or_default();
                }
                "CyclesWOB" => {
                    energy.cycles_wob = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "CyclesWOBBreakdown" => {
                    energy.cycles_wob_breakdown =
                        column.column_data.parse::<i64>().unwrap_or_default();
                }
                "DiskRaw" => {
                    energy.disk_raw = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "NetworkTailRaw" => {
                    energy.network_tail_raw = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "NetworkBytesRaw" => {
                    energy.network_bytes_raw =
                        column.column_data.parse::<i64>().unwrap_or_default();
                }
                "MBBTailRaw" => {
                    energy.mbb_tail_raw = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "MBBBytesRaw" => {
                    energy.mbb_bytes_raw = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "DisplayRequiredS" => {
                    energy.display_required_s =
                        column.column_data.parse::<i64>().unwrap_or_default();
                }
                "DisplayRequiredTimeline" => {
                    energy.display_required_timeline =
                        column.column_data.parse::<i64>().unwrap_or_default();
                }
                "KeyboardInputTimeline" => {
                    energy.keyboard_input_timeline =
                        column.column_data.parse::<i64>().unwrap_or_default();
                }
                "KeyboardInputS" => {
                    energy.keyboard_input_s = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "MouseInputS" => {
                    energy.mouse_input_s = column.column_data.parse::<i32>().unwrap_or_default();
                }
                _ => (),
            }
        }
        energy_vec.push(energy);
    }

    let serde_data_result = serde_json::to_value(&energy_vec);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[srum] Failed to serialize SRUM App Timeline table: {err:?}");
            return Err(SrumError::Serialize);
        }
    };

    Ok(serde_data)
}

/// Parse VFU table from SRUM. Not sure what this table is used for
pub(crate) fn parse_vfu_provider(
    column_rows: &[Vec<TableDump>],
    lookups: &HashMap<String, String>,
) -> Result<Value, SrumError> {
    let mut app_vec: Vec<AppVfu> = Vec::new();
    for rows in column_rows {
        let mut app = AppVfu {
            auto_inc_id: 0,
            timestamp: String::new(),
            app_id: String::new(),
            user_id: String::new(),
            flags: 0,
            start_time: String::new(),
            end_time: String::new(),
            usage: String::new(),
        };

        for column in rows {
            match column.column_name.as_str() {
                "AutoIncId" => {
                    app.auto_inc_id = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "TimeStamp" => {
                    app.timestamp.clone_from(&column.column_data);
                }
                "AppId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        app.app_id.clone_from(value);
                        continue;
                    }
                    app.app_id.clone_from(&column.column_data);
                }
                "UserId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        app.user_id.clone_from(value);
                        continue;
                    }
                    app.user_id.clone_from(&column.column_data);
                }
                "Flags" => app.flags = column.column_data.parse::<i32>().unwrap_or_default(),
                "StartTime" => {
                    app.start_time = unixepoch_to_iso(&filetime_to_unixepoch(
                        &(column.column_data.parse::<i64>().unwrap_or_default() as u64),
                    ));
                }
                "EndTime" => {
                    app.end_time = unixepoch_to_iso(&filetime_to_unixepoch(
                        &(column.column_data.parse::<i64>().unwrap_or_default() as u64),
                    ));
                }
                "Usage" => app.usage.clone_from(&column.column_data),
                _ => (),
            }
        }
        app_vec.push(app);
    }

    let serde_data_result = serde_json::to_value(&app_vec);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[srum] Failed to serialize SRUM application vfu table: {err:?}");
            return Err(SrumError::Serialize);
        }
    };

    Ok(serde_data)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{parse_app_timeline, parse_application, parse_vfu_provider};
    use crate::artifacts::os::windows::srum::{
        resource::get_srum_ese, tables::index::parse_id_lookup,
    };

    #[test]
    fn test_parse_app_timeline() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";

        let indexes = get_srum_ese(test_path, "SruDbIdMapTable").unwrap();
        let lookups = parse_id_lookup(&indexes);
        let srum_data = get_srum_ese(test_path, "{5C8CF1C7-7257-4F13-B223-970EF5939312}").unwrap();

        let results = parse_app_timeline(&srum_data, &lookups).unwrap();
        assert_eq!(results.is_null(), false)
    }

    #[test]
    fn test_parse_application() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";

        let indexes = get_srum_ese(test_path, "SruDbIdMapTable").unwrap();
        let lookups = parse_id_lookup(&indexes);
        let srum_data = get_srum_ese(test_path, "{D10CA2FE-6FCF-4F6D-848E-B2E99266FA89}").unwrap();

        let results = parse_application(&srum_data, &lookups).unwrap();
        assert_eq!(results.is_null(), false)
    }

    #[test]
    fn test_parse_vfu_provider() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";

        let indexes = get_srum_ese(test_path, "SruDbIdMapTable").unwrap();
        let lookups = parse_id_lookup(&indexes);
        let srum_data = get_srum_ese(test_path, "{7ACBBAA3-D029-4BE4-9A7A-0885927F1D8F}").unwrap();

        let results = parse_vfu_provider(&srum_data, &lookups).unwrap();
        assert_eq!(results.is_null(), false)
    }
}
