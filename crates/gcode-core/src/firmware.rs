//! Explicit firmware-state analysis. Targets are requested values, never measured temperatures.
use crate::foreign_words::{self, Command};
use crate::{Flavor, MAX_LINE_BYTES, MAX_MOVES, MAX_OUTPUT_BYTES, Result, invalid, number};
#[derive(Clone, Debug, PartialEq)]
pub struct ToolState {
    pub id: u32,
    pub target_c: Option<f64>,
    pub standby_c: Option<f64>,
    pub retracted: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WaitPolicy {
    None,
    Heating,
    HeatingOrCooling,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FirmwareEvent {
    pub line: usize,
    pub tool: Option<u32>,
    pub target_c: Option<f64>,
    pub wait: WaitPolicy,
    pub retract_offset_mm: Option<f64>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FirmwareState {
    pub tools: Vec<ToolState>,
    pub active_tool: u32,
    pub bed_target_c: Option<f64>,
    pub chamber_target_c: Option<f64>,
    pub events: Vec<FirmwareEvent>,
    pub unverified_lines: Vec<usize>,
}
/// Analyze the explicitly selected firmware dialect without changing logical E or preview material totals.
pub fn analyze_firmware(text: &str, flavor: Flavor) -> Result<FirmwareState> {
    if text.len() > MAX_OUTPUT_BYTES {
        return Err(invalid("GCODE_LIMIT", "Firmware input exceeds 4 MiB"));
    }
    let mut state = FirmwareState {
        tools: vec![],
        active_tool: 0,
        bed_target_c: None,
        chamber_target_c: None,
        events: vec![],
        unverified_lines: vec![],
    };
    let mut retract = None;
    let mut inches = false;
    let mut celsius = true;
    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        if line_number > MAX_MOVES {
            return Err(invalid("GCODE_LIMIT", "Firmware line limit exceeded"));
        }
        if line.len() > MAX_LINE_BYTES {
            return Err(invalid("GCODE_LIMIT", "Firmware line exceeds limit"));
        }
        let code = foreign_words::code(line);
        let mut words = foreign_words::words(&code);
        let Some(head) = words.next() else { continue };
        let command = foreign_words::command(head);
        if command == Command::M(149) {
            let flags: Vec<_> = words.collect();
            celsius = flags.len() == 1 && flags[0].eq_ignore_ascii_case("C");
            if !celsius {
                state.unverified_lines.push(line_number);
                for tool in &mut state.tools {
                    tool.target_c = None;
                    tool.standby_c = None;
                }
                state.bed_target_c = None;
                state.chamber_target_c = None;
            }
            continue;
        }
        if !celsius
            && matches!(
                command,
                Command::M(104 | 109 | 140 | 190 | 141 | 191) | Command::G(10)
            )
        {
            state.unverified_lines.push(line_number);
            continue;
        }

        let relevant = matches!(
            command,
            Command::T(_)
                | Command::G(10 | 11 | 20 | 21)
                | Command::M(104 | 109 | 140 | 190 | 141 | 191 | 207)
        );
        if !relevant {
            if matches!(
                command,
                Command::M(208 | 209) | Command::Other | Command::InvalidTool
            ) {
                state.unverified_lines.push(line_number);
            }
            continue;
        }
        if flavor == Flavor::Klipper
            && matches!(command, Command::G(10 | 11) | Command::M(141 | 191 | 207))
        {
            state.unverified_lines.push(line_number);
            continue;
        }
        let mut params = vec![];
        for word in words {
            if word.len() < 2 || !word.is_ascii() {
                return Err(invalid(
                    "GCODE_SYNTAX",
                    "Expected numeric firmware parameter",
                ));
            }
            let key = word.as_bytes()[0].to_ascii_uppercase();
            if params.iter().any(|(k, _)| *k == key) {
                return Err(invalid("GCODE_SYNTAX", "Duplicate firmware parameter"));
            }
            params.push((key, number(&word[1..])?));
        }
        let get = |key| params.iter().find(|(k, _)| *k == key).map(|(_, v)| *v);
        if matches!(command, Command::G(20 | 21)) {
            inches = command == Command::G(20);
            continue;
        }
        if command == Command::M(207) {
            if flavor != Flavor::Marlin
                || params
                    .iter()
                    .any(|(k, _)| !matches!(*k, b'S' | b'F' | b'Z'))
            {
                state.unverified_lines.push(line_number);
                continue;
            }
            if params.iter().any(|(_, v)| *v < 0.0) {
                return Err(invalid(
                    "GCODE_INVALID_SETTINGS",
                    "Negative retract setting",
                ));
            }
            if let Some(s) = get(b'S') {
                let length = s * if inches { 25.4 } else { 1.0 };
                if !(0.0..=1000.0).contains(&length) {
                    return Err(invalid(
                        "GCODE_INVALID_SETTINGS",
                        "Retract length out of bounds",
                    ));
                }
                retract = Some(length)
            }
            continue;
        }
        let temperature_g10 =
            command == Command::G(10) && flavor == Flavor::RepRapFirmware && get(b'P').is_some();
        let selector = if temperature_g10 {
            get(b'P')
        } else {
            get(b'T')
        };
        let id = if let Command::T(id) = command {
            id
        } else if let Some(v) = selector {
            if v < 0.0 || v > u32::MAX as f64 || v.fract() != 0.0 {
                return Err(invalid("GCODE_INVALID_SETTINGS", "Invalid firmware tool"));
            }
            v as u32
        } else {
            state.active_tool
        };
        let position = if let Some(i) = state.tools.iter().position(|t| t.id == id) {
            i
        } else {
            if state.tools.len() >= 256 {
                return Err(invalid("GCODE_LIMIT", "Firmware tool limit exceeded"));
            }
            state.tools.push(ToolState {
                id,
                target_c: None,
                standby_c: None,
                retracted: None,
            });
            state.tools.len() - 1
        };
        if matches!(command, Command::T(_)) {
            state.active_tool = id;
            continue;
        }
        let mut event = FirmwareEvent {
            line: line_number,
            tool: Some(id),
            target_c: None,
            wait: WaitPolicy::None,
            retract_offset_mm: None,
        };
        if matches!(command, Command::G(10 | 11)) && !temperature_g10 {
            if !params.is_empty() || !matches!(flavor, Flavor::Marlin | Flavor::RepRapFirmware) {
                state.unverified_lines.push(line_number);
                continue;
            }
            let tool = &mut state.tools[position];
            tool.retracted = Some(command == Command::G(10));
            event.retract_offset_mm = if tool.retracted == Some(true) {
                retract
            } else {
                Some(0.0)
            };
            if retract.is_none() {
                state.unverified_lines.push(line_number)
            }
        } else {
            let allowed = if temperature_g10 {
                b"PSR".as_slice()
            } else {
                b"TSR".as_slice()
            };
            if params.iter().any(|(k, _)| !allowed.contains(k))
                || (!temperature_g10 && get(b'S').is_some() && get(b'R').is_some())
            {
                state.unverified_lines.push(line_number);
                continue;
            }
            let waiting = matches!(command, Command::M(109 | 190 | 191));
            if !temperature_g10 && get(b'R').is_some() && !waiting {
                state.unverified_lines.push(line_number);
                continue;
            }
            let target = if temperature_g10 {
                get(b'S')
            } else {
                get(b'R').or(get(b'S'))
            };
            for value in [target, if temperature_g10 { get(b'R') } else { None }]
                .into_iter()
                .flatten()
            {
                if !(0.0..=500.0).contains(&value) {
                    return Err(invalid(
                        "GCODE_INVALID_SETTINGS",
                        "Temperature out of bounds",
                    ));
                }
            }
            let destination = match command {
                Command::M(140 | 190) => {
                    event.tool = None;
                    &mut state.bed_target_c
                }
                Command::M(141 | 191) => {
                    event.tool = None;
                    &mut state.chamber_target_c
                }
                _ => &mut state.tools[position].target_c,
            };
            if let Some(value) = target {
                *destination = Some(value)
            }
            event.target_c = *destination;
            if waiting {
                event.wait = if get(b'R').is_some() {
                    WaitPolicy::HeatingOrCooling
                } else {
                    WaitPolicy::Heating
                }
            }
            if temperature_g10 {
                if let Some(value) = get(b'R') {
                    state.tools[position].standby_c = Some(value)
                }
            }
        }
        if state.events.len() >= 8192 {
            return Err(invalid("GCODE_LIMIT", "Firmware state exceeds 8192 events"));
        }
        state.events.push(event);
        if state.events.len() + state.unverified_lines.len() > MAX_MOVES {
            return Err(invalid("GCODE_LIMIT", "Firmware event limit exceeded"));
        }
    }
    Ok(state)
}
