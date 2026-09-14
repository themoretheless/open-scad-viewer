//! Parse Bambu MQTT report JSON (observed `push_status` shape).

use crate::http::{find_number_field, find_string_field};
use crate::job::{map_vendor_state, JobState, JobStatus};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BambuReport {
    pub command: String,
    pub msg: Option<i64>,
    pub gcode_state: String,
    pub percent: Option<u8>,
    pub layer: Option<u32>,
    pub print_error: Option<i64>,
    pub raw: String,
}

impl BambuReport {
    pub fn is_full_snapshot(&self) -> bool {
        self.command == "push_status" && self.msg == Some(0)
    }

    pub fn is_start_like(&self) -> bool {
        matches!(
            map_vendor_state(&self.gcode_state),
            JobState::Preparing | JobState::Running
        )
    }

    pub fn has_print_error(&self) -> bool {
        matches!(self.print_error, Some(code) if code != 0)
    }

    pub fn to_status(&self) -> JobStatus {
        let mut state = map_vendor_state(&self.gcode_state);
        if self.has_print_error() && state != JobState::Running {
            state = JobState::Failed;
        }
        JobStatus {
            state,
            vendor_state: self.gcode_state.clone(),
            percent: self.percent,
            layer: self.layer,
            raw: self.raw.clone(),
        }
    }
}

pub fn parse_bambu_report(raw: &str) -> BambuReport {
    let command = find_string_field(raw, "command").unwrap_or_default();
    let gcode_state = find_string_field(raw, "gcode_state").unwrap_or_default();
    let msg = find_number_field(raw, "msg").map(|v| v as i64);
    let print_error = find_number_field(raw, "print_error").map(|v| v as i64);
    let percent = find_number_field(raw, "mc_percent").and_then(|v| {
        if (0.0..=100.0).contains(&v) {
            Some(v as u8)
        } else {
            None
        }
    });
    let layer = find_number_field(raw, "layer_num").map(|v| v as u32);
    BambuReport {
        command,
        msg,
        gcode_state,
        percent,
        layer,
        print_error,
        raw: raw.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_idle_snapshot() {
        let raw = r#"{"print":{"command":"push_status","msg":0,"gcode_state":"IDLE","mc_percent":0,"layer_num":0,"print_error":0}}"#;
        let r = parse_bambu_report(raw);
        assert!(r.is_full_snapshot());
        assert_eq!(r.to_status().state, JobState::Idle);
        assert!(!r.has_print_error());
    }

    #[test]
    fn printing_and_error() {
        let printing = parse_bambu_report(
            r#"{"print":{"command":"push_status","msg":0,"gcode_state":"RUNNING","mc_percent":12,"layer_num":3}}"#,
        );
        assert!(printing.is_start_like());
        let err = parse_bambu_report(
            r#"{"print":{"command":"push_status","msg":0,"gcode_state":"IDLE","print_error":83935248}}"#,
        );
        assert!(err.has_print_error());
        assert_eq!(err.to_status().state, JobState::Failed);
    }

    #[test]
    fn fixtures_parse() {
        let idle = include_str!("../../tests/fixtures/bambu_push_status_idle.json");
        let printing = include_str!("../../tests/fixtures/bambu_push_status_printing.json");
        let error = include_str!("../../tests/fixtures/bambu_push_status_print_error.json");
        assert!(parse_bambu_report(idle).is_full_snapshot());
        assert!(parse_bambu_report(printing).is_start_like());
        assert!(parse_bambu_report(error).has_print_error());
    }
}
