use crate::bambu::BambuPrintOptions;
use crate::job::admit_remote_name;
use crate::{invalid, Result};

pub fn request_topic(serial: &str) -> String {
    format!("device/{serial}/request")
}

pub fn report_topic(serial: &str) -> String {
    format!("device/{serial}/report")
}

pub fn pushall_payload(sequence_id: &str) -> String {
    format!(
        r#"{{"pushing":{{"sequence_id":"{}","command":"pushall"}}}}"#,
        escape_json(sequence_id)
    )
}

pub fn pause_payload(sequence_id: &str) -> String {
    format!(
        r#"{{"print":{{"sequence_id":"{}","command":"pause"}}}}"#,
        escape_json(sequence_id)
    )
}

pub fn resume_payload(sequence_id: &str) -> String {
    format!(
        r#"{{"print":{{"sequence_id":"{}","command":"resume"}}}}"#,
        escape_json(sequence_id)
    )
}

pub fn stop_payload(sequence_id: &str) -> String {
    format!(
        r#"{{"print":{{"sequence_id":"{}","command":"stop"}}}}"#,
        escape_json(sequence_id)
    )
}

/// Build `print.project_file` for a root FTPS upload.
///
/// `url` is `ftp:///<file_name>` (three slashes) as observed for LAN jobs.
pub fn project_file_payload(
    sequence_id: &str,
    remote_file_name: &str,
    plate_gcode_path: &str,
    options: &BambuPrintOptions,
) -> Result<String> {
    admit_remote_name(remote_file_name)?;
    if plate_gcode_path.is_empty() {
        return Err(invalid(
            "PRINTER_PLATE_PATH",
            "plate_gcode_path must be non-empty",
        ));
    }
    let url = format!("ftp:///{remote_file_name}");
    Ok(format!(
        concat!(
            r#"{{"print":{{"#,
            r#""sequence_id":"{seq}","command":"project_file","param":"{param}","url":"{url}","#,
            r#""project_id":"0","profile_id":"0","task_id":"0","subtask_id":"0","md5":"","#,
            r#""timelapse":{timelapse},"bed_type":"{bed}","bed_levelling":{bed_levelling},"#,
            r#""flow_cali":{flow},"vibration_cali":{vib},"layer_inspect":{inspect},"use_ams":{ams}"#,
            r#"}}}}"#
        ),
        seq = escape_json(sequence_id),
        param = escape_json(plate_gcode_path),
        url = escape_json(&url),
        timelapse = bool_json(options.timelapse),
        bed = escape_json(&options.bed_type),
        bed_levelling = bool_json(options.bed_levelling),
        flow = bool_json(options.flow_cali),
        vib = bool_json(options.vibration_cali),
        inspect = bool_json(options.layer_inspect),
        ams = bool_json(options.use_ams),
    ))
}

fn bool_json(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn escape_json(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                use std::fmt::Write;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_file_uses_root_ftp_url() {
        let payload = project_file_payload(
            "1",
            "box.gcode.3mf",
            "Metadata/plate_1.gcode",
            &BambuPrintOptions::default(),
        )
        .unwrap();
        assert!(payload.contains(r#""command":"project_file""#));
        assert!(payload.contains(r#""url":"ftp:///box.gcode.3mf""#));
        assert!(payload.contains(r#""param":"Metadata/plate_1.gcode""#));
        assert!(payload.contains(r#""use_ams":false"#));
    }
}
