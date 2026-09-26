//! Bounded, model-space G-code previews, not executable printer jobs.
//! XYZ is millimeters; E is absolute filament length; F is mm/min on disk.

use std::fmt::{self, Write};

pub const DIALECT: &str = "open-scad-viewer/print-preview 2";
pub const MAX_LAYERS: usize = 2_048;
pub const MAX_MOVES: usize = 100_000;
pub const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_LINE_BYTES: usize = 1_024;
pub const MAX_COORDINATE_MM: f64 = 1_000_000.0;
pub const COORDINATE_RESOLUTION_MM: f64 = 0.00001;
const MIN_FEEDRATE_MM_S: f64 = 0.001 / 60.0;
const PROLOGUE: [&str; 5] = ["G21", "G90", "M82", "M200 D0", "G92 E0"];

mod fixed7;
mod flavor;
mod foreign;
mod foreign_extrusion;
mod foreign_feedrate;
mod foreign_words;
mod job;
mod package_3mf;
mod parser;
mod writer;
pub use writer::{emit_job_to, emit_to};

pub use flavor::Flavor;
pub use foreign::{
    ASSUMED_FILAMENT_DIAMETER_MM, DialectInfo, Generator, detect, parse_any, parse_foreign,
    parse_foreign_with,
};
pub use job::{JOB_DIALECT, JobProfile, emit_job, job_flavor, parse_job};
pub use math_core::{Error, Result};
pub(crate) use parser::MotionParser;
pub use package_3mf::{
    MeshBody, PLATE_JSON_PATH, emit_3mf, emit_gcode_3mf_job, extract_gcode_3mf, extract_member_3mf,
    package_gcode_3mf, package_job_3mf, parse_3mf,
};

#[derive(Clone, Debug, PartialEq)]
pub struct MachineProfile {
    pub layer_height_mm: f64,
    pub line_width_mm: f64,
    pub print_feedrate_mm_s: f64,
    pub travel_feedrate_mm_s: f64,
    pub filament_diameter_mm: f64,
}

impl Default for MachineProfile {
    fn default() -> Self {
        Self {
            layer_height_mm: 0.2,
            line_width_mm: 0.4,
            print_feedrate_mm_s: 50.0,
            travel_feedrate_mm_s: 120.0,
            filament_diameter_mm: 1.75,
        }
    }
}

impl MachineProfile {
    pub fn validate(&self) -> Result<()> {
        let dimensions = [
            self.layer_height_mm,
            self.line_width_mm,
            self.filament_diameter_mm,
        ];
        let speeds = [self.print_feedrate_mm_s, self.travel_feedrate_mm_s];
        if !dimensions.iter().all(|v| valid_dimension(*v))
            || !speeds
                .iter()
                .all(|v| v.is_finite() && (MIN_FEEDRATE_MM_S..=MAX_COORDINATE_MM).contains(v))
            || ![
                self.bead_area_mm2(),
                self.filament_area_mm2(),
                self.bead_area_mm2() / self.filament_area_mm2(),
            ]
            .iter()
            .all(|v| v.is_finite() && *v > 0.0)
        {
            return Err(invalid(
                "GCODE_INVALID_SETTINGS",
                "Dimensions must be 0.00001..1000000 mm and speeds 0.001/60..1000000 mm/s, with finite positive derived values",
            ));
        }
        Ok(())
    }

    pub fn bead_area_mm2(&self) -> f64 {
        self.line_width_mm * self.layer_height_mm
    }

    pub fn filament_area_mm2(&self) -> f64 {
        std::f64::consts::PI * (self.filament_diameter_mm * 0.5).powi(2)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlannedPath {
    pub points: Vec<[f64; 2]>,
    pub closed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlannedLayer {
    pub z_mm: f64,
    pub paths: Vec<PlannedPath>,
}

/// Borrowed counterpart of [`PlannedPath`]: zero-copy emitter input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlannedPathRef<'a> {
    pub points: &'a [[f64; 2]],
    pub closed: bool,
}

/// Borrowed counterpart of [`PlannedLayer`]. Building one allocates only the
/// small per-path views; the points themselves are never copied.
#[derive(Clone, Debug, PartialEq)]
pub struct PlannedLayerRef<'a> {
    pub z_mm: f64,
    pub paths: Vec<PlannedPathRef<'a>>,
}

pub(crate) trait LayerAccess {
    fn z_mm(&self) -> f64;
    fn for_each_path(&self, visit: &mut dyn FnMut(&[[f64; 2]], bool));
}

impl LayerAccess for PlannedLayer {
    fn z_mm(&self) -> f64 {
        self.z_mm
    }
    fn for_each_path(&self, visit: &mut dyn FnMut(&[[f64; 2]], bool)) {
        for path in &self.paths {
            visit(&path.points, path.closed);
        }
    }
}

impl LayerAccess for PlannedLayerRef<'_> {
    fn z_mm(&self) -> f64 {
        self.z_mm
    }
    fn for_each_path(&self, visit: &mut dyn FnMut(&[[f64; 2]], bool)) {
        for path in &self.paths {
            visit(path.points, path.closed);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GcodeMove {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// Absolute filament position in millimeters.
    pub e: f64,
    pub feedrate_mm_s: f64,
    pub layer_index: usize,
    /// True only when absolute E increases on this movement.
    pub extruded: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GcodeBounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct GcodePreview {
    pub layers: usize,
    pub extrusion_mm: f64,
    pub deposited_volume_mm3: f64,
    pub moves: Vec<GcodeMove>,
    /// Bounds of fully known positions, without an assumed machine origin.
    pub bounds: Option<GcodeBounds>,
    pub travel_distance_mm: f64,
    pub print_distance_mm: f64,
    /// Nominal distance / commanded speed; excludes acceleration and startup.
    pub estimated_time_s: f64,
}

impl GcodePreview {
    pub fn with_volume(mut self, volume_mm3: f64) -> Self {
        self.deposited_volume_mm3 = volume_mm3;
        self
    }
}

pub(crate) fn invalid(code: &'static str, message: &str) -> Error {
    Error::new(code, message)
}

pub(crate) fn valid_dimension(value: f64) -> bool {
    value.is_finite() && (COORDINATE_RESOLUTION_MM..=MAX_COORDINATE_MM).contains(&value)
}

pub(crate) fn valid_coordinate(value: f64) -> bool {
    value.is_finite() && value.abs() <= MAX_COORDINATE_MM
}

pub(crate) fn rounded_coordinate(value: f64) -> f64 {
    (value * 100_000.0).round() / 100_000.0
}

pub(crate) fn require_layers<L: LayerAccess>(layers: &[L]) -> Result<()> {
    if layers.len() > MAX_LAYERS {
        return Err(invalid(
            "GCODE_LAYER_LIMIT",
            "G-code preview exceeded 2048 layers",
        ));
    }
    let mut previous_z = None;
    let mut moves = 0usize;
    let mut paths = 0usize;
    for layer in layers {
        if !valid_coordinate(layer.z_mm()) {
            return Err(invalid(
                "GCODE_INVALID_HEIGHT",
                "Layer Z must be finite and within +/-1000000 mm",
            ));
        }
        let z = rounded_coordinate(layer.z_mm());
        if previous_z.is_some_and(|previous| z <= previous) {
            return Err(invalid(
                "GCODE_INVALID_HEIGHT",
                "Layer Z must increase at the exported 0.00001 mm resolution",
            ));
        }
        previous_z = Some(z);
        moves += 1;
        if moves > MAX_MOVES {
            return Err(move_limit());
        }
        let mut result = Ok(());
        layer.for_each_path(&mut |points, closed| {
            if result.is_err() {
                return;
            }
            paths += 1;
            // Bound work before visiting vertices or allocating output, even for one huge layer.
            let additional = points.len().checked_add(usize::from(closed));
            moves = match additional.and_then(|n| moves.checked_add(n)) {
                Some(m) => m,
                None => {
                    result = Err(move_limit());
                    return;
                }
            };
            if paths > MAX_MOVES || moves > MAX_MOVES {
                result = Err(move_limit());
                return;
            }
            if closed && points.len() < 3 {
                result = Err(invalid(
                    "GCODE_INVALID_PATH",
                    "A closed path needs at least three points",
                ));
                return;
            }
            if points
                .iter()
                .flatten()
                .any(|value| !valid_coordinate(*value))
            {
                result = Err(invalid(
                    "GCODE_INVALID_COORDINATE",
                    "Path coordinates must be finite and within +/-1000000 mm",
                ));
            }
        });
        result?;
    }
    Ok(())
}

fn move_limit() -> Error {
    invalid(
        "GCODE_MOVE_LIMIT",
        "G-code preview exceeded 100000 moves or paths",
    )
}

fn path_length(points: &[[f64; 2]], closed: bool) -> f64 {
    let open: f64 = points
        .windows(2)
        .map(|pair| (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1]))
        .sum();
    if closed && points.len() >= 3 {
        let first = points[0];
        let last = points[points.len() - 1];
        open + (last[0] - first[0]).hypot(last[1] - first[1])
    } else {
        open
    }
}

pub fn path_length_mm(path: &PlannedPath) -> f64 {
    path_length(&path.points, path.closed)
}

/// Analytic volume of an unquantized plan. Callers must validate their plan/profile.
pub fn deposited_volume_mm3(layers: &[PlannedLayer], machine: &MachineProfile) -> f64 {
    deposited_volume(layers, machine)
}

/// Borrowed-input variant of [`deposited_volume_mm3`]; same analytic sum
/// without cloning any path points.
pub fn deposited_volume_mm3_ref(layers: &[PlannedLayerRef], machine: &MachineProfile) -> f64 {
    deposited_volume(layers, machine)
}

fn deposited_volume<L: LayerAccess>(layers: &[L], machine: &MachineProfile) -> f64 {
    let area = machine.bead_area_mm2();
    let mut total = 0.0;
    for layer in layers {
        layer.for_each_path(&mut |points, closed| {
            total += path_length(points, closed) * area;
        });
    }
    total
}

/// Fails before an append would take the string length past the public output limit.
pub(crate) struct BoundedOutput(pub(crate) String);

impl Write for BoundedOutput {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if text.len() > MAX_OUTPUT_BYTES.saturating_sub(self.0.len()) {
            return Err(fmt::Error);
        }
        self.0.push_str(text);
        Ok(())
    }
}

pub(crate) fn output_limit(_: fmt::Error) -> Error {
    invalid("GCODE_OUTPUT_LIMIT", "G-code preview exceeds 4 MiB")
}

/// Serialize a preview plan. This contains no homing/heating or printer setup.
pub fn emit(layers: &[PlannedLayer], machine: &MachineProfile) -> Result<String> {
    let mut out = BoundedOutput(String::with_capacity(estimated_output_bytes(layers)));
    emit_body(layers, machine, &mut out)?;
    Ok(out.0)
}

/// Borrowed-input variant of [`emit`]: identical bytes, no point copies.
pub fn emit_ref(layers: &[PlannedLayerRef], machine: &MachineProfile) -> Result<String> {
    let mut out = BoundedOutput(String::with_capacity(estimated_output_bytes(layers)));
    emit_body(layers, machine, &mut out)?;
    Ok(out.0)
}

/// Rough upper-bound sizing for the serialization buffer, capped at the limit.
pub(crate) fn estimated_output_bytes<L: LayerAccess>(layers: &[L]) -> usize {
    let moves: usize = layers
        .iter()
        .map(|layer| {
            let mut sum = 2;
            layer.for_each_path(&mut |points, _| {
                sum += points.len() + 2;
            });
            sum
        })
        .sum();
    moves
        .saturating_mul(64)
        .saturating_add(1024)
        .min(MAX_OUTPUT_BYTES)
}

fn emit_body<L: LayerAccess>(
    layers: &[L],
    machine: &MachineProfile,
    out: &mut impl Write,
) -> Result<()> {
    machine.validate()?;
    require_layers(layers)?;
    let ratio = machine.bead_area_mm2() / machine.filament_area_mm2();
    let print_f = machine.print_feedrate_mm_s * 60.0;
    let travel_f = machine.travel_feedrate_mm_s * 60.0;
    writeln!(out, "; {DIALECT}\n; Preview only: model coordinates, no printer startup or shutdown\n;FILAMENT_DIAMETER_MM:{}", machine.filament_diameter_mm).map_err(output_limit)?;
    for command in PROLOGUE {
        writeln!(out, "{command}").map_err(output_limit)?;
    }
    let mut e = 0.0;
    let mut written_units = 0i64;
    let mut ebuf = String::with_capacity(24);
    for (index, layer) in layers.iter().enumerate() {
        let z = rounded_coordinate(layer.z_mm());
        writeln!(out, ";LAYER:{index}\n;Z:{z:.5}\nG1 Z{z:.5} F{travel_f:.3}")
            .map_err(output_limit)?;
        let mut result = Ok(());
        layer.for_each_path(&mut |points, closed| {
            if result.is_err() {
                return;
            }
            result = emit_path(
                out,
                points,
                closed,
                ratio,
                print_f,
                travel_f,
                &mut e,
                &mut written_units,
                &mut ebuf,
            );
        });
        result?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn emit_path(
    out: &mut impl Write,
    points: &[[f64; 2]],
    closed: bool,
    ratio: f64,
    print_f: f64,
    travel_f: f64,
    e: &mut f64,
    written_units: &mut i64,
    ebuf: &mut String,
) -> Result<()> {
    let Some(first) = points.first() else {
        return Ok(());
    };
    let start = first.map(rounded_coordinate);
    writeln!(out, "G0 X{:.5} Y{:.5} F{travel_f:.3}", start[0], start[1])
        .map_err(output_limit)?;
    let mut previous = start;
    for point in points.iter().skip(1).chain(closed.then_some(first)) {
        let next = point.map(rounded_coordinate);
        let length = (next[0] - previous[0]).hypot(next[1] - previous[1]);
        if length == 0.0 {
            continue;
        }
        let next_e = *e + length * ratio;
        if !next_e.is_finite() || next_e <= *e {
            return Err(invalid(
                "GCODE_NUMERIC",
                "Extrusion accumulation exceeds numeric precision",
            ));
        }
        ebuf.clear();
        let next_units = fixed7::push_fixed7(ebuf, next_e)?;
        if next_units <= *written_units {
            return Err(invalid(
                "GCODE_NUMERIC",
                "A segment's extrusion cannot be represented at 0.0000001 mm precision",
            ));
        }
        *e = next_e;
        *written_units = next_units;
        writeln!(
            out,
            "G1 X{:.5} Y{:.5} E{ebuf} F{print_f:.3}",
            next[0], next[1]
        )
        .map_err(output_limit)?;
        previous = next;
    }
    Ok(())
}

#[derive(Default)]
pub(crate) struct MoveWords {
    pub(crate) x: Option<f64>,
    pub(crate) y: Option<f64>,
    pub(crate) z: Option<f64>,
    pub(crate) e: Option<f64>,
    pub(crate) f: Option<f64>,
}

pub(crate) fn number(text: &str) -> Result<f64> {
    text.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| {
            invalid(
                "GCODE_INVALID_NUMBER",
                "G-code numbers must be finite decimal values",
            )
        })
}

pub(crate) fn words<'a>(tokens: impl Iterator<Item = &'a str>) -> Result<MoveWords> {
    let mut words = MoveWords::default();
    let mut count = 0;
    for token in tokens {
        // Check ASCII before splitting: malformed Unicode must never panic.
        if token.len() < 2 || !token.is_ascii() {
            return Err(invalid(
                "GCODE_SYNTAX",
                "Expected whitespace-separated axis/value words",
            ));
        }
        let (axis, value) = token.split_at(1);
        let slot = match axis {
            "X" => &mut words.x,
            "Y" => &mut words.y,
            "Z" => &mut words.z,
            "E" => &mut words.e,
            "F" => &mut words.f,
            _ => return Err(invalid("GCODE_SYNTAX", "Unsupported G-code move word")),
        };
        if slot.is_some() {
            return Err(invalid("GCODE_SYNTAX", "Duplicate G-code move word"));
        }
        *slot = Some(number(value)?);
        count += 1;
    }
    if count == 0 {
        return Err(invalid(
            "GCODE_SYNTAX",
            "A move needs at least one axis or feedrate",
        ));
    }
    Ok(words)
}

/// Strict parser for this crate's preview dialect, not a general printer parser.
/// Initial incomplete XYZ setup establishes position without inventing an origin.
pub fn parse(gcode: &str) -> Result<GcodePreview> {
    if gcode.len() > MAX_OUTPUT_BYTES {
        return Err(invalid(
            "GCODE_OUTPUT_LIMIT",
            "G-code preview exceeds 4 MiB",
        ));
    }
    if gcode.lines().next() == Some("; open-scad-viewer/print-preview 1") {
        return Err(invalid(
            "GCODE_DIALECT",
            "Legacy print-preview 1 is unsupported; regenerate the preview with the current exporter",
        ));
    }
    if gcode.lines().next() != Some(format!("; {DIALECT}").as_str()) {
        return Err(invalid(
            "GCODE_DIALECT",
            "Not an open-scad-viewer print preview",
        ));
    }
    let mut parser = MotionParser::new();
    let mut prologue = 0;
    for (line_index, raw) in gcode.lines().enumerate().skip(1) {
        let line_result: Result<()> = (|| {
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
            if let Some(text) = line.strip_prefix(";LAYER:") {
                return parser.admit_layer(
                    text,
                    prologue == PROLOGUE.len(),
                    "Layers require complete units/modes and filament metadata",
                    "G-code preview exceeded 2048 layers",
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
            if prologue < PROLOGUE.len() {
                if command_text
                    .split_whitespace()
                    .ne(PROLOGUE[prologue].split_whitespace())
                {
                    return Err(invalid(
                        "GCODE_PROLOGUE",
                        "Expected G21, G90, M82, M200 D0, then G92 E0",
                    ));
                }
                prologue += 1;
                return Ok(());
            }
            if command != "G0" && command != "G1" {
                return Err(invalid(
                    "GCODE_UNSUPPORTED_COMMAND",
                    "Only absolute G0/G1 moves are allowed after preview setup",
                ));
            }
            if parser.result.layers == 0 {
                return Err(invalid("GCODE_LAYER", "Motion must follow a layer marker"));
            }
            parser.count_move("G-code preview exceeded 100000 moves or paths")?;
            let words = words(tokens)?;
            let old_position = parser.position();
            let was_known = parser.all_known();
            parser.apply_position_words(&words)?;
            parser.enforce_layer_z(&words)?;
            let feedrate = parser.admit_feedrate(&words)?;
            let e = words.e.unwrap_or(parser.result.extrusion_mm);
            if e < parser.result.extrusion_mm {
                return Err(invalid(
                    "GCODE_UNSUPPORTED_EXTRUSION",
                    "Preview extrusion cannot retract or decrease",
                ));
            }
            let extruded = e > parser.result.extrusion_mm;
            let is_known = parser.all_known();
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
            if !is_known {
                return Ok(());
            }
            // A pure F command updates modal speed without fabricating a movement.
            if words.x.is_none() && words.y.is_none() && words.z.is_none() && words.e.is_none() {
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
    if prologue != PROLOGUE.len() || parser.diameter.is_none() {
        return Err(invalid(
            "GCODE_PROLOGUE",
            "Preview is missing complete units/modes or filament metadata",
        ));
    }
    let extrusion_mm = parser.result.extrusion_mm;
    parser.finish(extrusion_mm, "Preview totals exceed numeric precision")
}
