//! Machine job dialect: heating, retract, start/end. Not LAN upload.

use crate::flavor::{ShutdownStep, StartupStep};
use crate::{
    BoundedOutput, Flavor, GcodePreview, MAX_COORDINATE_MM, MAX_LAYERS, MAX_LINE_BYTES,
    MAX_OUTPUT_BYTES, MachineProfile, MotionParser, PlannedLayer, Result, estimated_output_bytes,
    fixed7, invalid, number, output_limit, require_layers, rounded_coordinate, valid_dimension,
    words,
};
use std::fmt::Write;

pub const JOB_DIALECT: &str = "open-scad-viewer/print-job 1";
const MIN_FEEDRATE_MM_S: f64 = 0.001 / 60.0;

#[derive(Clone, Debug, PartialEq)]
pub struct JobProfile {
    pub machine: MachineProfile,
    pub nozzle_temp_c: f64,
    pub bed_temp_c: f64,
    pub retract_length_mm: f64,
    pub retract_feedrate_mm_s: f64,
    pub unretract_feedrate_mm_s: f64,
    /// Travel length that triggers retract between paths.
    pub retract_min_travel_mm: f64,
    /// PWM 0..=255; `0` omits fan commands.
    pub fan_speed: u8,
    pub home_axes: bool,
    /// Firmware family that selects startup/prologue/shutdown commands.
    pub flavor: Flavor,
}

impl Default for JobProfile {
    fn default() -> Self {
        Self {
            machine: MachineProfile::default(),
            nozzle_temp_c: 210.0,
            bed_temp_c: 60.0,
            retract_length_mm: 0.8,
            retract_feedrate_mm_s: 40.0,
            unretract_feedrate_mm_s: 40.0,
            retract_min_travel_mm: 2.0,
            fan_speed: 255,
            home_axes: true,
            flavor: Flavor::Marlin,
        }
    }
}

impl JobProfile {
    pub fn validate(&self) -> Result<()> {
        self.machine.validate()?;
        for (label, value) in [("nozzle", self.nozzle_temp_c), ("bed", self.bed_temp_c)] {
            if !value.is_finite() || !(0.0..=500.0).contains(&value) {
                return Err(invalid(
                    "GCODE_INVALID_SETTINGS",
                    &format!("{label} temperature must be finite and within 0..500 C"),
                ));
            }
        }
        if !valid_dimension(self.retract_length_mm) {
            return Err(invalid(
                "GCODE_INVALID_SETTINGS",
                "retract_length_mm must be 0.00001..1000000 mm",
            ));
        }
        for speed in [self.retract_feedrate_mm_s, self.unretract_feedrate_mm_s] {
            if !speed.is_finite() || !(MIN_FEEDRATE_MM_S..=MAX_COORDINATE_MM).contains(&speed) {
                return Err(invalid(
                    "GCODE_INVALID_SETTINGS",
                    "Retract feedrates must be 0.001/60..1000000 mm/s",
                ));
            }
        }
        if !self.retract_min_travel_mm.is_finite() || self.retract_min_travel_mm < 0.0 {
            return Err(invalid(
                "GCODE_INVALID_SETTINGS",
                "retract_min_travel_mm must be finite and non-negative",
            ));
        }
        Ok(())
    }
}

fn travel_xy(a: [f64; 2], b: [f64; 2]) -> f64 {
    (b[0] - a[0]).hypot(b[1] - a[1])
}

/// Serialize a machine job: heat, optional home, absolute E, retract on long travels.
pub fn emit_job(layers: &[PlannedLayer], job: &JobProfile) -> Result<String> {
    let mut out = BoundedOutput(String::with_capacity(estimated_output_bytes(layers)));
    emit_job_body(layers, job, &mut out)?;
    Ok(out.0)
}

pub(crate) fn emit_job_body(
    layers: &[PlannedLayer],
    job: &JobProfile,
    out: &mut impl Write,
) -> Result<()> {
    job.validate()?;
    require_layers(layers)?;
    let machine = &job.machine;
    let ratio = machine.bead_area_mm2() / machine.filament_area_mm2();
    let print_f = machine.print_feedrate_mm_s * 60.0;
    let travel_f = machine.travel_feedrate_mm_s * 60.0;
    let retract_f = job.retract_feedrate_mm_s * 60.0;
    let unretract_f = job.unretract_feedrate_mm_s * 60.0;
    let flavor = job.flavor;
    writeln!(
        out,
        "; {JOB_DIALECT}\n;FLAVOR:{}\n; Machine job: heating and retract enabled; not a LAN upload certificate\n;FILAMENT_DIAMETER_MM:{}\n;NOZZLE_TEMP_C:{}\n;BED_TEMP_C:{}\n;EST_TIME_S:0",
        flavor.header_label(), machine.filament_diameter_mm, job.nozzle_temp_c, job.bed_temp_c
    )
    .map_err(output_limit)?;
    for command in flavor.heat_commands(job) {
        writeln!(out, "{command}").map_err(output_limit)?;
    }
    if job.home_axes {
        writeln!(out, "G28").map_err(output_limit)?;
    }
    for command in flavor.prologue() {
        writeln!(out, "{command}").map_err(output_limit)?;
    }
    if job.fan_speed > 0 {
        writeln!(out, "M106 S{}", job.fan_speed).map_err(output_limit)?;
    }
    let mut e = 0.0;
    let mut written_units = 0i64;
    let mut ebuf = String::with_capacity(24);
    let mut last_xy: Option<[f64; 2]> = None;
    let mut filament_retracted = false;
    for (index, layer) in layers.iter().enumerate() {
        let z = rounded_coordinate(layer.z_mm);
        writeln!(out, ";LAYER:{index}\n;Z:{z:.5}").map_err(output_limit)?;
        if let Some(command) = flavor.layer_command(index, layers.len()) {
            writeln!(out, "{command}").map_err(output_limit)?;
        }
        writeln!(out, "G1 Z{z:.5} F{travel_f:.3}").map_err(output_limit)?;
        for path in &layer.paths {
            let Some(first) = path.points.first() else {
                continue;
            };
            let start = first.map(rounded_coordinate);
            if let Some(previous) = last_xy {
                let hop = travel_xy(previous, start);
                if hop >= job.retract_min_travel_mm && !filament_retracted {
                    let next_e = e - job.retract_length_mm;
                    if !next_e.is_finite() || next_e < 0.0 {
                        return Err(invalid(
                            "GCODE_NUMERIC",
                            "Retract would drive absolute E below zero",
                        ));
                    }
                    ebuf.clear();
                    let next_units = fixed7::push_fixed7(&mut ebuf, next_e)?;
                    if next_units >= written_units {
                        return Err(invalid(
                            "GCODE_NUMERIC",
                            "Retract E cannot be represented at export precision",
                        ));
                    }
                    e = next_e;
                    written_units = next_units;
                    writeln!(out, "G1 E{ebuf} F{retract_f:.3}").map_err(output_limit)?;
                    filament_retracted = true;
                }
            }
            writeln!(out, "G0 X{:.5} Y{:.5} F{travel_f:.3}", start[0], start[1])
                .map_err(output_limit)?;
            if filament_retracted {
                let next_e = e + job.retract_length_mm;
                ebuf.clear();
                let next_units = fixed7::push_fixed7(&mut ebuf, next_e)?;
                if next_units <= written_units {
                    return Err(invalid(
                        "GCODE_NUMERIC",
                        "Unretract E cannot be represented at export precision",
                    ));
                }
                e = next_e;
                written_units = next_units;
                writeln!(out, "G1 E{ebuf} F{unretract_f:.3}").map_err(output_limit)?;
                filament_retracted = false;
            }
            let mut previous = start;
            for point in path
                .points
                .iter()
                .skip(1)
                .chain(path.closed.then_some(first))
            {
                let next = point.map(rounded_coordinate);
                let length = travel_xy(previous, next);
                if length == 0.0 {
                    continue;
                }
                let next_e = e + length * ratio;
                if !next_e.is_finite() || next_e <= e {
                    return Err(invalid(
                        "GCODE_NUMERIC",
                        "Extrusion accumulation exceeds numeric precision",
                    ));
                }
                ebuf.clear();
                let next_units = fixed7::push_fixed7(&mut ebuf, next_e)?;
                if next_units <= written_units {
                    return Err(invalid(
                        "GCODE_NUMERIC",
                        "A segment's extrusion cannot be represented at 0.0000001 mm precision",
                    ));
                }
                e = next_e;
                written_units = next_units;
                writeln!(
                    out,
                    "G1 X{:.5} Y{:.5} E{ebuf} F{print_f:.3}",
                    next[0], next[1]
                )
                .map_err(output_limit)?;
                previous = next;
            }
            last_xy = Some(previous);
        }
    }
    if filament_retracted {
        let next_e = e + job.retract_length_mm;
        ebuf.clear();
        fixed7::push_fixed7(&mut ebuf, next_e)?;
        writeln!(out, "G1 E{ebuf} F{unretract_f:.3}").map_err(output_limit)?;
    }
    if job.fan_speed > 0 {
        writeln!(out, "M107").map_err(output_limit)?;
    }
    if job.home_axes {
        writeln!(out, "G28 X Y").map_err(output_limit)?;
    }
    for command in flavor.shutdown_commands() {
        writeln!(out, "{command}").map_err(output_limit)?;
    }
    Ok(())
}

#[derive(Default)]
struct Startup {
    bed_set: bool,
    nozzle_set: bool,
    bed_wait: bool,
    nozzle_wait: bool,
    prologue: usize,
}

/// Reads the `;FLAVOR:` header. Files without it are the original Marlin layout.
pub fn job_flavor(gcode: &str) -> Result<Flavor> {
    for line in gcode.lines().take(16) {
        if let Some(label) = line.trim().strip_prefix(";FLAVOR:") {
            return Flavor::from_name(label);
        }
    }
    Ok(Flavor::Marlin)
}

/// Strict parser for `print-job 1` in any supported flavor. Returns preview totals.
pub fn parse_job(gcode: &str) -> Result<GcodePreview> {
    if gcode.len() > MAX_OUTPUT_BYTES {
        return Err(invalid("GCODE_OUTPUT_LIMIT", "G-code job exceeds 4 MiB"));
    }
    if gcode.lines().next() != Some(format!("; {JOB_DIALECT}").as_str()) {
        return Err(invalid(
            "GCODE_DIALECT",
            "Not an open-scad-viewer print job",
        ));
    }
    let flavor = job_flavor(gcode)?;
    let prologue = flavor.prologue();
    let mut parser = MotionParser::new();
    let mut startup = Startup::default();
    let mut finished = false;
    let mut nozzle_off = false;
    let mut peak_e: f64 = 0.0;
    for (line_index, raw) in gcode.lines().enumerate().skip(1) {
        let line_result: Result<()> = (|| {
            if finished {
                return Err(invalid(
                    "GCODE_TRAILING",
                    "Commands after job shutdown are not allowed",
                ));
            }
            if raw.len() > MAX_LINE_BYTES {
                return Err(invalid(
                    "GCODE_LINE_LIMIT",
                    "G-code line exceeds 1024 bytes",
                ));
            }
            let line = raw.trim();
            if let Some(result) = parser.admit_diameter(line) {
                return result;
            }
            if line.starts_with(";NOZZLE_TEMP_C:")
                || line.starts_with(";BED_TEMP_C:")
                || line.starts_with(";EST_TIME_S:")
                || line.starts_with(";FLAVOR:")
                || line.starts_with("; Machine job:")
            {
                return Ok(());
            }
            if let Some(text) = line.strip_prefix(";LAYER:") {
                return parser.admit_layer(
                    text,
                    startup_ready(&startup, prologue.len()),
                    "Layers require heat, modes, and filament metadata",
                    "G-code job exceeded 2048 layers",
                );
            }
            if let Some(result) = parser.admit_z_annotation(line) {
                return result;
            }
            let command_text = line.split(';').next().unwrap_or("").trim();
            if command_text.is_empty() {
                return Ok(());
            }
            let mut tokens = command_text.split_whitespace();
            let command = tokens.next().unwrap_or("");
            let rest: Vec<&str> = tokens.collect();
            if parser.result.layers == 0 && startup.prologue < prologue.len() {
                if let Some(step) = flavor.startup_step(command, &rest)? {
                    match step {
                        StartupStep::BedSet => startup.bed_set = true,
                        StartupStep::NozzleSet => startup.nozzle_set = true,
                        StartupStep::BedWait => startup.bed_wait = true,
                        StartupStep::NozzleWait => startup.nozzle_wait = true,
                        StartupStep::ToolSelect => {}
                    }
                    return Ok(());
                }
                if command == "G28" {
                    if !rest.is_empty() {
                        return Err(invalid(
                            "GCODE_SYNTAX",
                            "Startup G28 must have no axis words",
                        ));
                    }
                    return Ok(());
                }
                if !(startup.bed_set
                    && startup.nozzle_set
                    && startup.bed_wait
                    && startup.nozzle_wait)
                {
                    return Err(invalid(
                        "GCODE_PROLOGUE",
                        "Expected bed and nozzle heat-and-wait commands before units/modes",
                    ));
                }
                if command_text
                    .split_whitespace()
                    .ne(prologue[startup.prologue].split_whitespace())
                {
                    return Err(invalid("GCODE_PROLOGUE", flavor.prologue_error()));
                }
                startup.prologue += 1;
                return Ok(());
            }
            if parser.result.layers == 0 && startup.prologue == prologue.len() {
                if command == "M106" {
                    admit_fan(rest.first().copied())?;
                    return Ok(());
                }
                if command == "G0" || command == "G1" {
                    return Err(invalid("GCODE_LAYER", "Motion must follow a layer marker"));
                }
            }
            if command == "SET_PRINT_STATS_INFO" {
                if flavor != Flavor::Klipper
                    || parser.result.layers == 0
                    || parser.current_layer_z().is_some()
                {
                    return Err(invalid(
                        "GCODE_UNSUPPORTED_COMMAND",
                        "SET_PRINT_STATS_INFO is only allowed after a Klipper layer marker",
                    ));
                }
                for word in &rest {
                    let Some((key, value)) = word.split_once('=') else {
                        return Err(invalid("GCODE_SYNTAX", "Expected KEY=VALUE words"));
                    };
                    let value = value
                        .parse::<usize>()
                        .map_err(|_| invalid("GCODE_SYNTAX", "Layer counts must be integers"))?;
                    match key {
                        "CURRENT_LAYER" if value == parser.result.layers => {}
                        "TOTAL_LAYER"
                            if parser.result.layers == 1 && (1..=MAX_LAYERS).contains(&value) => {}
                        _ => {
                            return Err(invalid(
                                "GCODE_LAYER",
                                "Print stats layer numbers must match the layer markers",
                            ));
                        }
                    }
                }
                return Ok(());
            }
            let shutdown = flavor.shutdown_step(command, &rest)?;
            if shutdown.is_some() || command == "M107" || command == "G28" {
                if parser.result.layers == 0 {
                    return Err(invalid(
                        "GCODE_PROLOGUE",
                        "Shutdown commands require completed layers",
                    ));
                }
                if command == "M107" {
                    if !rest.is_empty() {
                        return Err(invalid("GCODE_SYNTAX", "M107 takes no arguments"));
                    }
                    return Ok(());
                }
                if command == "G28" {
                    let mut saw_x = false;
                    let mut saw_y = false;
                    for token in &rest {
                        match *token {
                            "X" => saw_x = true,
                            "Y" => saw_y = true,
                            _ => {
                                return Err(invalid(
                                    "GCODE_UNSUPPORTED_COMMAND",
                                    "End homing must be G28 X Y",
                                ));
                            }
                        }
                    }
                    if !saw_x || !saw_y {
                        return Err(invalid(
                            "GCODE_UNSUPPORTED_COMMAND",
                            "End homing must be G28 X Y",
                        ));
                    }
                    return Ok(());
                }
                match shutdown {
                    Some(ShutdownStep::NozzleOff) => nozzle_off = true,
                    Some(ShutdownStep::Final) => {
                        if !nozzle_off {
                            return Err(invalid(
                                "GCODE_PROLOGUE",
                                "Shutdown must turn heaters off before the final command",
                            ));
                        }
                        finished = true;
                    }
                    None => unreachable!(),
                }
                return Ok(());
            }
            if command != "G0" && command != "G1" {
                return Err(invalid(
                    "GCODE_UNSUPPORTED_COMMAND",
                    "Unsupported job command after setup",
                ));
            }
            if parser.result.layers == 0 {
                return Err(invalid("GCODE_LAYER", "Motion must follow a layer marker"));
            }
            parser.count_move("G-code job exceeded 100000 moves or paths")?;
            let words = words(rest.iter().copied())?;
            let old_position = parser.position();
            let was_known = parser.all_known();
            parser.apply_position_words(&words)?;
            parser.enforce_layer_z(&words)?;
            let feedrate = parser.admit_feedrate(&words)?;
            let e = words.e.unwrap_or(parser.result.extrusion_mm);
            if e < 0.0 || !e.is_finite() {
                return Err(invalid(
                    "GCODE_UNSUPPORTED_EXTRUSION",
                    "Absolute E must be finite and non-negative",
                ));
            }
            let extruded = e > parser.result.extrusion_mm;
            let retracted = e < parser.result.extrusion_mm;
            if retracted
                && (command != "G1" || words.x.is_some() || words.y.is_some() || words.z.is_some())
            {
                return Err(invalid(
                    "GCODE_UNSUPPORTED_EXTRUSION",
                    "Retract must be a pure G1 E move",
                ));
            }
            let is_known = parser.all_known();
            if extruded && words.x.is_none() && words.y.is_none() && words.z.is_none() {
                // Unretract: E-only advance.
                parser.result.extrusion_mm = e;
                peak_e = peak_e.max(e);
                return Ok(());
            }
            let distance = parser.distance_from(old_position, was_known);
            if extruded
                && (command != "G1"
                    || !was_known
                    || distance == 0.0
                    || parser.position()[2] != old_position[2])
            {
                return Err(invalid(
                    "GCODE_UNSUPPORTED_EXTRUSION",
                    "Extrusion needs a G1 XY segment from a known position at fixed Z",
                ));
            }
            parser.result.extrusion_mm = e;
            peak_e = peak_e.max(e);
            if !is_known {
                return Ok(());
            }
            if words.x.is_none() && words.y.is_none() && words.z.is_none() && words.e.is_none() {
                return Ok(());
            }
            if retracted {
                return Ok(());
            }
            parser.record_motion(old_position, was_known, feedrate, e, extruded);
            Ok(())
        })();
        line_result.map_err(|error| {
            invalid(
                error.code,
                &format!("Line {}: {}", line_index + 1, error.message),
            )
        })?;
    }
    if !startup_ready(&startup, prologue.len()) || parser.diameter.is_none() {
        return Err(invalid(
            "GCODE_PROLOGUE",
            "Job is missing heat, units/modes, or filament metadata",
        ));
    }
    parser.finish(peak_e, "Job totals exceed numeric precision")
}

fn startup_ready(startup: &Startup, prologue_len: usize) -> bool {
    startup.bed_set
        && startup.nozzle_set
        && startup.bed_wait
        && startup.nozzle_wait
        && startup.prologue == prologue_len
}

fn admit_fan(word: Option<&str>) -> Result<()> {
    let word = word.ok_or_else(|| invalid("GCODE_SYNTAX", "M106 needs an S word"))?;
    if !word.starts_with('S') {
        return Err(invalid("GCODE_SYNTAX", "M106 needs an S word"));
    }
    let value = number(&word[1..])?;
    if !(0.0..=255.0).contains(&value) || value.fract() != 0.0 {
        return Err(invalid(
            "GCODE_INVALID_SETTINGS",
            "Fan speed must be an integer 0..255",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PlannedPath;

    fn square() -> PlannedLayer {
        PlannedLayer {
            z_mm: 0.2,
            paths: vec![
                PlannedPath {
                    points: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
                    closed: true,
                },
                PlannedPath {
                    points: vec![[20.0, 0.0], [30.0, 0.0], [30.0, 10.0], [20.0, 10.0]],
                    closed: true,
                },
            ],
        }
    }

    #[test]
    fn writer_preserves_every_flavor_and_refuses_invalid_jobs_before_writing() {
        for flavor in [Flavor::Marlin, Flavor::Klipper, Flavor::RepRapFirmware] {
            let job = JobProfile {
                flavor,
                ..JobProfile::default()
            };
            let layers = [square()];
            let expected = emit_job(&layers, &job).unwrap();
            let mut actual = String::new();
            crate::emit_job_to(&layers, &job, &mut actual).unwrap();
            assert_eq!(actual, expected);
            parse_job(&actual).unwrap();
        }
        let invalid = JobProfile {
            nozzle_temp_c: f64::NAN,
            ..JobProfile::default()
        };
        let mut output = String::new();
        assert!(crate::emit_job_to(&[square()], &invalid, &mut output).is_err());
        assert!(output.is_empty());
    }

    #[test]
    fn job_round_trips_with_heat_and_retract() {
        let job = JobProfile::default();
        let gcode = emit_job(&[square()], &job).unwrap();
        assert!(gcode.starts_with(&format!("; {JOB_DIALECT}\n")));
        assert!(gcode.contains("M109 S210"));
        assert!(gcode.contains("M190 S60"));
        assert!(gcode.contains("G28\n"));
        assert!(gcode.contains("M106 S255"));
        assert!(gcode.contains("M104 S0"));
        assert!(gcode.contains("G1 E"));
        let preview = parse_job(&gcode).unwrap();
        assert_eq!(preview.layers, 1);
        assert!(preview.extrusion_mm > 0.0);
        assert!(preview.print_distance_mm > 0.0);
    }

    #[test]
    fn every_flavor_round_trips_and_rejects_other_flavors_prologue() {
        for flavor in Flavor::ALL {
            let job = JobProfile {
                flavor,
                ..JobProfile::default()
            };
            let gcode = emit_job(&[square()], &job).unwrap();
            assert!(gcode.contains(&format!(";FLAVOR:{}\n", flavor.header_label())));
            assert_eq!(job_flavor(&gcode).unwrap(), flavor);
            let preview = parse_job(&gcode).unwrap();
            assert_eq!(preview.layers, 1);
            assert!(preview.extrusion_mm > 0.0);
            for other in Flavor::ALL.into_iter().filter(|other| *other != flavor) {
                let relabeled = gcode.replacen(
                    &format!(";FLAVOR:{}", flavor.header_label()),
                    &format!(";FLAVOR:{}", other.header_label()),
                    1,
                );
                assert_eq!(parse_job(&relabeled).unwrap_err().code, "GCODE_PROLOGUE");
            }
        }
    }

    #[test]
    fn klipper_flavor_omits_m200_and_uses_native_heaters() {
        let job = JobProfile {
            flavor: Flavor::Klipper,
            ..JobProfile::default()
        };
        let gcode = emit_job(&[square()], &job).unwrap();
        assert!(!gcode.contains("M200"));
        assert!(!gcode.contains("M109"));
        assert!(gcode.contains("SET_HEATER_TEMPERATURE HEATER=extruder TARGET=210"));
        assert!(gcode.contains("TEMPERATURE_WAIT SENSOR=heater_bed MINIMUM=60"));
        assert!(gcode.contains("SET_PRINT_STATS_INFO TOTAL_LAYER=1 CURRENT_LAYER=1"));
        assert!(gcode.ends_with("TURN_OFF_HEATERS\nM84\n"));
    }

    #[test]
    fn reprapfirmware_flavor_uses_g10_tool_temperatures() {
        let job = JobProfile {
            flavor: Flavor::RepRapFirmware,
            ..JobProfile::default()
        };
        let gcode = emit_job(&[square()], &job).unwrap();
        assert!(gcode.contains("G10 P0 S210 R210\nT0\nM190 S60\nM116\n"));
        assert!(gcode.contains("M200 D0"));
        parse_job(&gcode).unwrap();
    }

    #[test]
    fn flavor_header_is_optional_for_marlin_and_unknown_names_fail() {
        let gcode = emit_job(&[square()], &JobProfile::default()).unwrap();
        let legacy = gcode.replacen(";FLAVOR:Marlin\n", "", 1);
        assert!(!legacy.contains("FLAVOR"));
        assert_eq!(parse_job(&legacy).unwrap().layers, 1);
        let unknown = gcode.replacen(";FLAVOR:Marlin", ";FLAVOR:Sailfish", 1);
        assert_eq!(parse_job(&unknown).unwrap_err().code, "GCODE_FLAVOR");
        assert_eq!(Flavor::from_name("RRF").unwrap(), Flavor::RepRapFirmware);
        assert_eq!(Flavor::from_name("marlin2").unwrap(), Flavor::Marlin);
    }

    #[test]
    fn preview_parser_rejects_job_dialect() {
        let gcode = emit_job(&[square()], &JobProfile::default()).unwrap();
        assert_eq!(crate::parse(&gcode).unwrap_err().code, "GCODE_DIALECT");
    }
}
