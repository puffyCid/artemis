use crate::artifacts::os::windows::{ese::parser::TableDump, srum::error::SrumError};
use log::error;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
struct ApplicationInfo {
    auto_inc_id: i32,
    timestamp: i64,
    app_id: String,
    user_id: String,
    foreground_cycle_time: i64,
    background_cycle_time: i64,
    facetime: i64,
    foreground_context_switches: i32,
    background_context_switches: i32,
    foreground_bytes_read: i64,
    foreground_bytes_written: i64,
    foreground_num_read_operations: i32,
    foreground_num_write_options: i32,
    foreground_number_of_flushes: i32,
    background_bytes_read: i64,
    background_bytes_written: i64,
    background_num_read_operations: i32,
    background_num_write_operations: i32,
    background_number_of_flushes: i32,
}

#[derive(Debug, Serialize)]
struct AppTimelineInfo {
    auto_inc_id: i32,
    timestamp: i64,
    app_id: String,
    user_id: String,
    flags: i32,
    end_time: i64,
    duration_ms: i32,
    span_ms: i32,
    timeline_end: i32,
    in_focus_timeline: i64,
    user_input_timeline: i64,
    comp_rendered_timeline: i64,
    comp_dirtied_timeline: i64,
    comp_propagated_timeline: i64,
    audio_in_timeline: i64,
    audio_out_timeline: i64,
    cpu_timeline: i64,
    disk_timeline: i64,
    network_timeline: i64,
    mbb_timeline: i64,
    in_focus_s: i32,
    psm_foreground_s: i32,
    user_input_s: i32,
    comp_rendered_s: i32,
    comp_dirtied_s: i32,
    comp_propagated_s: i32,
    audio_in_s: i32,
    audio_out_s: i32,
    cycles: i64,
    cycles_breakdown: i64,
    cycles_attr: i64,
    cycles_attr_breakdown: i64,
    cycles_wob: i64,
    cycles_wob_breakdown: i64,
    disk_raw: i64,
    network_tail_raw: i64,
    network_bytes_raw: i64,
    mbb_tail_raw: i64,
    mbb_bytes_raw: i64,
    display_required_s: i64,
    display_required_timeline: i64,
    keyboard_input_timeline: i64,
    keyboard_input_s: i32,
    mouse_input_s: i32,
}

#[derive(Debug, Serialize)]
struct AppVfu {
    auto_inc_id: i32,
    timestamp: i64,
    app_id: String,
    user_id: String,
    flags: i32,
    start_time: i64,
    end_time: i64,
    usage: String,
}

/// Parse the application table from SRUM
pub(crate) fn parse_application(
    column_rows: &[Vec<TableDump>],
    lookups: &HashMap<String, String>,
) -> Result<(Value, String), SrumError> {
    let mut app_vec: Vec<ApplicationInfo> = Vec::new();
    for rows in column_rows {
        let mut app = ApplicationInfo {
            auto_inc_id: 0,
            timestamp: 0,
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
                    app.timestamp = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "AppId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        app.app_id = value.clone();
                        continue;
                    }
                    app.app_id = column.column_data.clone();
                }
                "UserId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        app.user_id = value.clone();
                        continue;
                    }
                    app.user_id = column.column_data.clone();
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

                _ => continue,
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
    Ok((serde_data, String::from("srum_application")))
}

/// Parse the app timeline table from SRUM
pub(crate) fn parse_app_timeline(
    column_rows: &[Vec<TableDump>],
    lookups: &HashMap<String, String>,
) -> Result<(Value, String), SrumError> {
    let mut energy_vec: Vec<AppTimelineInfo> = Vec::new();
    for rows in column_rows {
        let mut energy = AppTimelineInfo {
            auto_inc_id: 0,
            timestamp: 0,
            app_id: String::new(),
            user_id: String::new(),
            flags: 0,
            end_time: 0,
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
        for column in rows {
            match column.column_name.as_str() {
                "AutoIncId" => {
                    energy.auto_inc_id = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "TimeStamp" => {
                    energy.timestamp = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "AppId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        energy.app_id = value.clone();
                        continue;
                    }
                    energy.app_id = column.column_data.clone();
                }
                "UserId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        energy.user_id = value.clone();
                        continue;
                    }
                    energy.user_id = column.column_data.clone();
                }
                "Flags" => energy.flags = column.column_data.parse::<i32>().unwrap_or_default(),
                "EndTime" => {
                    energy.end_time = column.column_data.parse::<i64>().unwrap_or_default();
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
                _ => continue,
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

    Ok((serde_data, String::from("srum_app_timeline")))
}

/// Parse VFU table from SRUM. Not sure what this table is used for
pub(crate) fn parse_vfu_provider(
    column_rows: &[Vec<TableDump>],
    lookups: &HashMap<String, String>,
) -> Result<(Value, String), SrumError> {
    let mut app_vec: Vec<AppVfu> = Vec::new();
    for rows in column_rows {
        let mut app = AppVfu {
            auto_inc_id: 0,
            timestamp: 0,
            app_id: String::new(),
            user_id: String::new(),
            flags: 0,
            start_time: 0,
            end_time: 0,
            usage: String::new(),
        };

        for column in rows {
            match column.column_name.as_str() {
                "AutoIncId" => {
                    app.auto_inc_id = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "TimeStamp" => {
                    app.timestamp = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "AppId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        app.app_id = value.clone();
                        continue;
                    }
                    app.app_id = column.column_data.clone();
                }
                "UserId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        app.user_id = value.clone();
                        continue;
                    }
                    app.user_id = column.column_data.clone();
                }
                "Flags" => app.flags = column.column_data.parse::<i32>().unwrap_or_default(),
                "StartTime" => {
                    app.start_time = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "EndTime" => app.end_time = column.column_data.parse::<i64>().unwrap_or_default(),
                "Usage" => app.usage = column.column_data.clone(),
                _ => continue,
            }
        }
        app_vec.push(app);
    }

    let serde_data_result = serde_json::to_value(&app_vec);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[srum] Failed to serialize SRUM appication vfu table: {err:?}");
            return Err(SrumError::Serialize);
        }
    };

    Ok((serde_data, String::from("srum_app_vfu")))
}

#[cfg(test)]
mod tests {
    use super::{parse_app_timeline, parse_application, parse_vfu_provider};
    use crate::artifacts::os::windows::{
        ese::parser::grab_ese_tables_path, srum::tables::index::parse_id_lookup,
    };

    #[test]
    fn test_parse_app_timeline() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";
        let table = vec![
            String::from("SruDbIdMapTable"),
            String::from("{5C8CF1C7-7257-4F13-B223-970EF5939312}"),
        ];
        let test_data = grab_ese_tables_path(test_path, &table).unwrap();
        let ids = test_data.get("SruDbIdMapTable").unwrap();
        let id_results = parse_id_lookup(&ids);
        let energy = test_data
            .get("{5C8CF1C7-7257-4F13-B223-970EF5939312}")
            .unwrap();

        let (results, _) = parse_app_timeline(&energy, &id_results).unwrap();
        assert_eq!(results.is_null(), false)
    }

    #[test]
    fn test_parse_application() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";
        let table = vec![
            String::from("SruDbIdMapTable"),
            String::from("{D10CA2FE-6FCF-4F6D-848E-B2E99266FA89}"),
        ];
        let test_data = grab_ese_tables_path(test_path, &table).unwrap();
        let ids = test_data.get("SruDbIdMapTable").unwrap();
        let id_results = parse_id_lookup(&ids);
        let energy = test_data
            .get("{D10CA2FE-6FCF-4F6D-848E-B2E99266FA89}")
            .unwrap();

        let (results, _) = parse_application(&energy, &id_results).unwrap();
        assert_eq!(results.is_null(), false)
    }

    #[test]
    fn test_parse_vfu_provider() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";
        let table = vec![
            String::from("SruDbIdMapTable"),
            String::from("{7ACBBAA3-D029-4BE4-9A7A-0885927F1D8F}"),
        ];
        let test_data = grab_ese_tables_path(test_path, &table).unwrap();
        let ids = test_data.get("SruDbIdMapTable").unwrap();
        let id_results = parse_id_lookup(&ids);
        let energy = test_data
            .get("{7ACBBAA3-D029-4BE4-9A7A-0885927F1D8F}")
            .unwrap();

        let (results, _) = parse_vfu_provider(&energy, &id_results).unwrap();
        assert_eq!(results.is_null(), false)
    }
}
