use crate::bambu::BambuPrintOptions;
use crate::hash::md5_hex_upper;
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
/// `md5` should be uppercase hex of the uploaded bytes (or empty to skip).
pub fn project_file_payload(
    sequence_id: &str,
    remote_file_name: &str,
    plate_gcode_path: &str,
    options: &BambuPrintOptions,
    md5: &str,
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
            r#""project_id":"0","profile_id":"0","task_id":"0","subtask_id":"0","md5":"{md5}","#,
            r#""timelapse":{timelapse},"bed_type":"{bed}","bed_levelling":{bed_levelling},"#,
            r#""flow_cali":{flow},"vibration_cali":{vib},"layer_inspect":{inspect},"use_ams":{ams}"#,
            r#"}}}}"#
        ),
        seq = escape_json(sequence_id),
        param = escape_json(plate_gcode_path),
        url = escape_json(&url),
        md5 = escape_json(md5),
        timelapse = bool_json(options.timelapse),
        bed = escape_json(&options.bed_type),
        bed_levelling = bool_json(options.bed_levelling),
        flow = bool_json(options.flow_cali),
        vib = bool_json(options.vibration_cali),
        inspect = bool_json(options.layer_inspect),
        ams = bool_json(options.use_ams),
    ))
}

pub fn artifact_md5(bytes: &[u8]) -> String {
    md5_hex_upper(bytes)
}

fn bool_json(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn escape_json(value: &str) -> String {
    crate::http::escape_json(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_file_includes_md5() {
        let payload = project_file_payload(
            "1",
            "box.gcode.3mf",
            "Metadata/plate_1.gcode",
            &BambuPrintOptions::default(),
            "AABBCC",
        )
        .unwrap();
        assert!(payload.contains(r#""command":"project_file""#));
        assert!(payload.contains(r#""url":"ftp:///box.gcode.3mf""#));
        assert!(payload.contains(r#""md5":"AABBCC""#));
        assert!(payload.contains(r#""use_ams":false"#));
    }
}
