//! Firmware flavors for the job dialect. A flavor selects the startup, prologue,
//! layer and shutdown commands a firmware family understands; motion is shared.

use crate::{JobProfile, Result, invalid, number};

/// Target firmware family for `emit_job`. Names follow PrusaSlicer/Cura `gcode_flavor`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Flavor {
    /// Marlin 1/2 and compatible forks (Creality, Prusa Buddy, Ender…).
    #[default]
    Marlin,
    /// Klipper: no `M200`, native heater/print-stats extended commands.
    Klipper,
    /// RepRapFirmware (Duet): `G10` tool temperatures, `M116` wait.
    RepRapFirmware,
}

impl Flavor {
    pub const ALL: [Flavor; 3] = [Flavor::Marlin, Flavor::Klipper, Flavor::RepRapFirmware];

    /// Stable machine-readable name used by hosts and the `;FLAVOR:` header.
    pub fn name(self) -> &'static str {
        match self {
            Flavor::Marlin => "marlin",
            Flavor::Klipper => "klipper",
            Flavor::RepRapFirmware => "reprapfirmware",
        }
    }

    /// Human label written in the file header (`;FLAVOR:<label>`), Cura style.
    pub fn header_label(self) -> &'static str {
        match self {
            Flavor::Marlin => "Marlin",
            Flavor::Klipper => "Klipper",
            Flavor::RepRapFirmware => "RepRapFirmware",
        }
    }

    /// Accepts the canonical names plus common aliases from slicer profiles.
    pub fn from_name(name: &str) -> Result<Flavor> {
        let key = name.trim().to_ascii_lowercase();
        match key.as_str() {
            "marlin" | "marlin2" | "marlinfirmware" => Ok(Flavor::Marlin),
            "klipper" => Ok(Flavor::Klipper),
            "reprapfirmware" | "rrf" | "reprap" | "duet" => Ok(Flavor::RepRapFirmware),
            _ => Err(invalid(
                "GCODE_FLAVOR",
                "Unsupported G-code flavor; expected marlin, klipper or reprapfirmware",
            )),
        }
    }

    /// Units/modes prologue. Klipper reports `M200` as unknown, so it is omitted there.
    pub fn prologue(self) -> &'static [&'static str] {
        match self {
            Flavor::Klipper => &["G21", "G90", "M82", "G92 E0"],
            Flavor::Marlin | Flavor::RepRapFirmware => &["G21", "G90", "M82", "M200 D0", "G92 E0"],
        }
    }

    pub(crate) fn prologue_error(self) -> &'static str {
        match self {
            Flavor::Klipper => "Expected G21, G90, M82, then G92 E0",
            Flavor::Marlin | Flavor::RepRapFirmware => {
                "Expected G21, G90, M82, M200 D0, then G92 E0"
            }
        }
    }

    /// Heating commands before homing, in emitted order.
    pub(crate) fn heat_commands(self, job: &JobProfile) -> Vec<String> {
        let bed = format!("{:.0}", job.bed_temp_c);
        let nozzle = format!("{:.0}", job.nozzle_temp_c);
        match self {
            Flavor::Marlin => vec![
                format!("M140 S{bed}"),
                format!("M104 S{nozzle}"),
                format!("M190 S{bed}"),
                format!("M109 S{nozzle}"),
            ],
            Flavor::Klipper => vec![
                format!("SET_HEATER_TEMPERATURE HEATER=heater_bed TARGET={bed}"),
                format!("SET_HEATER_TEMPERATURE HEATER=extruder TARGET={nozzle}"),
                format!("TEMPERATURE_WAIT SENSOR=heater_bed MINIMUM={bed}"),
                format!("TEMPERATURE_WAIT SENSOR=extruder MINIMUM={nozzle}"),
            ],
            Flavor::RepRapFirmware => vec![
                format!("M140 S{bed}"),
                format!("G10 P0 S{nozzle} R{nozzle}"),
                "T0".to_string(),
                format!("M190 S{bed}"),
                "M116".to_string(),
            ],
        }
    }

    /// Optional command emitted right after `;LAYER:<index>` / `;Z:` metadata.
    pub(crate) fn layer_command(self, index: usize, total: usize) -> Option<String> {
        match self {
            Flavor::Klipper => Some(if index == 0 {
                format!("SET_PRINT_STATS_INFO TOTAL_LAYER={total} CURRENT_LAYER=1")
            } else {
                format!("SET_PRINT_STATS_INFO CURRENT_LAYER={}", index + 1)
            }),
            Flavor::Marlin | Flavor::RepRapFirmware => None,
        }
    }

    /// Cooldown after fan-off and optional end homing. The last one ends the job.
    pub(crate) fn shutdown_commands(self) -> &'static [&'static str] {
        match self {
            Flavor::Marlin | Flavor::RepRapFirmware => &["M104 S0", "M140 S0"],
            Flavor::Klipper => &["TURN_OFF_HEATERS", "M84"],
        }
    }

    /// Classifies a startup command for the strict job parser.
    pub(crate) fn startup_step(self, command: &str, rest: &[&str]) -> Result<Option<StartupStep>> {
        Ok(Some(match (self, command) {
            (Flavor::Marlin | Flavor::RepRapFirmware, "M140") => {
                temp_word(command, rest)?;
                StartupStep::BedSet
            }
            (Flavor::Marlin, "M104") => {
                temp_word(command, rest)?;
                StartupStep::NozzleSet
            }
            (Flavor::Marlin | Flavor::RepRapFirmware, "M190") => {
                temp_word(command, rest)?;
                StartupStep::BedWait
            }
            (Flavor::Marlin, "M109") => {
                temp_word(command, rest)?;
                StartupStep::NozzleWait
            }
            (Flavor::RepRapFirmware, "G10") => {
                let mut saw_tool = false;
                let mut saw_temp = false;
                for word in rest {
                    match word.as_bytes().first() {
                        Some(b'P') => saw_tool = true,
                        Some(b'S') | Some(b'R') => {
                            admit_temperature(number(&word[1..])?)?;
                            saw_temp = true;
                        }
                        _ => return Err(invalid("GCODE_SYNTAX", "G10 needs P and S/R words")),
                    }
                }
                if !saw_tool || !saw_temp {
                    return Err(invalid("GCODE_SYNTAX", "G10 needs P and S/R words"));
                }
                StartupStep::NozzleSet
            }
            (Flavor::RepRapFirmware, "T0") => {
                if !rest.is_empty() {
                    return Err(invalid("GCODE_SYNTAX", "Tool select takes no words"));
                }
                StartupStep::ToolSelect
            }
            (Flavor::RepRapFirmware, "M116") => {
                if !rest.is_empty() {
                    return Err(invalid("GCODE_SYNTAX", "M116 takes no words"));
                }
                StartupStep::NozzleWait
            }
            (Flavor::Klipper, "SET_HEATER_TEMPERATURE") => {
                let heater = keyword(rest, "HEATER")?;
                admit_temperature(number(keyword(rest, "TARGET")?)?)?;
                match heater {
                    "heater_bed" => StartupStep::BedSet,
                    "extruder" => StartupStep::NozzleSet,
                    _ => return Err(invalid("GCODE_SYNTAX", "Unknown heater name")),
                }
            }
            (Flavor::Klipper, "TEMPERATURE_WAIT") => {
                let sensor = keyword(rest, "SENSOR")?;
                admit_temperature(number(keyword(rest, "MINIMUM")?)?)?;
                match sensor {
                    "heater_bed" => StartupStep::BedWait,
                    "extruder" => StartupStep::NozzleWait,
                    _ => return Err(invalid("GCODE_SYNTAX", "Unknown sensor name")),
                }
            }
            _ => return Ok(None),
        }))
    }

    /// Classifies a shutdown command for the strict job parser.
    pub(crate) fn shutdown_step(
        self,
        command: &str,
        rest: &[&str],
    ) -> Result<Option<ShutdownStep>> {
        Ok(Some(match (self, command) {
            (Flavor::Marlin | Flavor::RepRapFirmware, "M104") => {
                zero_temp(rest)?;
                ShutdownStep::NozzleOff
            }
            (Flavor::Marlin | Flavor::RepRapFirmware, "M140") => {
                zero_temp(rest)?;
                ShutdownStep::Final
            }
            (Flavor::Klipper, "TURN_OFF_HEATERS") => {
                if !rest.is_empty() {
                    return Err(invalid("GCODE_SYNTAX", "TURN_OFF_HEATERS takes no words"));
                }
                ShutdownStep::NozzleOff
            }
            (Flavor::Klipper, "M84") => {
                if !rest.is_empty() {
                    return Err(invalid("GCODE_SYNTAX", "M84 takes no words"));
                }
                ShutdownStep::Final
            }
            _ => return Ok(None),
        }))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StartupStep {
    BedSet,
    NozzleSet,
    BedWait,
    NozzleWait,
    ToolSelect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShutdownStep {
    NozzleOff,
    Final,
}

pub(crate) fn admit_temperature(value: f64) -> Result<()> {
    if !value.is_finite() || !(0.0..=500.0).contains(&value) {
        return Err(invalid(
            "GCODE_INVALID_SETTINGS",
            "Temperature must be finite and within 0..500 C",
        ));
    }
    Ok(())
}

fn temp_word(command: &str, rest: &[&str]) -> Result<()> {
    let [word] = rest else {
        return Err(invalid(
            "GCODE_SYNTAX",
            &format!("{command} needs exactly one S word"),
        ));
    };
    let Some(value) = word.strip_prefix('S') else {
        return Err(invalid(
            "GCODE_SYNTAX",
            &format!("{command} needs an S word"),
        ));
    };
    admit_temperature(number(value)?)
}

fn zero_temp(rest: &[&str]) -> Result<()> {
    let [word] = rest else {
        return Err(invalid("GCODE_SYNTAX", "Temperature needs an S word"));
    };
    if !word.starts_with('S') || number(&word[1..])? != 0.0 {
        return Err(invalid(
            "GCODE_UNSUPPORTED_COMMAND",
            "Only S0 cooldown is allowed after layers",
        ));
    }
    Ok(())
}

fn keyword<'a>(rest: &[&'a str], key: &str) -> Result<&'a str> {
    rest.iter()
        .find_map(|word| {
            let (name, value) = word.split_once('=')?;
            (name == key).then_some(value)
        })
        .ok_or_else(|| invalid("GCODE_SYNTAX", &format!("Missing {key}= parameter")))
}
