//! G-code encoding of a planned toolpath. Not a print process and not the
//! only possible machine dialect. No mesh/NURBS/WASM dependency.
//! Coordinates are millimeters; feedrate is mm/s internally and F is mm/min.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]

pub const DIALECT: &str = "open-scad-viewer/print-preview 1";
pub const MAX_LAYERS: usize = 2_048;
pub const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;

pub use math_core::{Error, Result};

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
        if ![
            self.layer_height_mm,
            self.line_width_mm,
            self.print_feedrate_mm_s,
            self.travel_feedrate_mm_s,
            self.filament_diameter_mm,
        ]
        .iter()
        .all(|value| value.is_finite() && *value > 0.0)
        {
            return Err(Error::new(
                "GCODE_INVALID_SETTINGS",
                "Machine profile values must be finite and positive",
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

#[derive(Clone, Debug, PartialEq)]
pub struct GcodeMove {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub extruded: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GcodePreview {
    pub layers: usize,
    pub extrusion_mm: f64,
    pub deposited_volume_mm3: f64,
    pub moves: Vec<GcodeMove>,
}

impl GcodePreview {
    pub fn with_volume(mut self, volume_mm3: f64) -> Self {
        self.deposited_volume_mm3 = volume_mm3;
        self
    }
}

fn invalid(code: &'static str, message: &str) -> Error {
    Error::new(code, message)
}

fn require_layers(layers: &[PlannedLayer]) -> Result<()> {
    if layers.len() > MAX_LAYERS {
        return Err(invalid(
            "GCODE_LAYER_LIMIT",
            "G-code preview exceeded 2048 layers",
        ));
    }
    if layers.iter().any(|layer| !layer.z_mm.is_finite()) {
        return Err(invalid(
            "GCODE_INVALID_HEIGHT",
            "Layer height must be finite",
        ));
    }
    Ok(())
}

fn path_vertices(path: &PlannedPath) -> Vec<[f64; 2]> {
    if path.closed && path.points.len() >= 3 {
        let mut points = path.points.clone();
        points.push(path.points[0]);
        points
    } else {
        path.points.clone()
    }
}

pub fn path_length_mm(path: &PlannedPath) -> f64 {
    let points = path_vertices(path);
    if points.len() < 2 {
        return 0.0;
    }
    points
        .array_windows()
        .map(|[a, b]| (b[0] - a[0]).hypot(b[1] - a[1]))
        .sum()
}

pub fn deposited_volume_mm3(layers: &[PlannedLayer], machine: &MachineProfile) -> f64 {
    let area = machine.bead_area_mm2();
    layers
        .iter()
        .flat_map(|layer| layer.paths.iter())
        .map(|path| path_length_mm(path) * area)
        .sum()
}

/// Serialize a completed print plan. Units are millimeters; E is filament length.
pub fn emit(layers: &[PlannedLayer], machine: &MachineProfile) -> Result<String> {
    machine.validate()?;
    require_layers(layers)?;
    let filament = machine.filament_area_mm2();
    if filament <= 0.0 {
        return Err(invalid(
            "GCODE_INVALID_SETTINGS",
            "Filament diameter must be positive",
        ));
    }
    let area = machine.bead_area_mm2();
    let print_f = machine.print_feedrate_mm_s * 60.0;
    let travel_f = machine.travel_feedrate_mm_s * 60.0;
    let mut out = format!("; {DIALECT}\nG21\nG90\nM82\nG92 E0\n");
    let mut e = 0.0;
    for (index, layer) in layers.iter().enumerate() {
        out.push_str(&format!(
            ";LAYER:{index}\n;Z:{:.5}\nG1 Z{:.5} F{travel_f:.3}\n",
            layer.z_mm, layer.z_mm
        ));
        for path in &layer.paths {
            if path.points.is_empty() {
                continue;
            }
            let start = path.points[0];
            out.push_str(&format!(
                "G0 X{:.5} Y{:.5} F{travel_f:.3}\n",
                start[0], start[1]
            ));
            for [a, b] in path_vertices(path).array_windows() {
                let length = (b[0] - a[0]).hypot(b[1] - a[1]);
                e += length * area / filament;
                out.push_str(&format!(
                    "G1 X{:.5} Y{:.5} E{:.7} F{print_f:.3}\n",
                    b[0], b[1], e
                ));
            }
        }
        if out.len() > MAX_OUTPUT_BYTES {
            return Err(invalid(
                "GCODE_OUTPUT_LIMIT",
                "G-code preview exceeds 4 MiB",
            ));
        }
    }
    out.push_str("M104 S0\nM140 S0\nG28\n");
    Ok(out)
}

/// Independent parse of exported preview G-code. Physical print is out of scope.
pub fn parse(gcode: &str) -> Result<GcodePreview> {
    if !gcode.starts_with(&format!("; {DIALECT}")) {
        return Err(invalid(
            "GCODE_DIALECT",
            "Not an open-scad-viewer print preview",
        ));
    }
    let mut layers = 0_usize;
    let mut extrusion_mm = 0.0;
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;
    let mut moves = Vec::new();
    for line in gcode.lines() {
        if let Some(rest) = line.strip_prefix(";LAYER:")
            && rest.parse::<usize>().is_ok()
        {
            layers += 1;
        }
        let command = line.split_whitespace().next().unwrap_or("");
        if command != "G0" && command != "G1" {
            continue;
        }
        let extruded = line.contains(" E");
        for token in line.split_whitespace().skip(1) {
            if token.len() < 2 {
                continue;
            }
            let (axis, value) = token.split_at(1);
            if let Ok(number) = value.parse::<f64>() {
                match axis {
                    "X" => x = number,
                    "Y" => y = number,
                    "Z" => z = number,
                    "E" => extrusion_mm = number,
                    _ => {}
                }
            }
        }
        moves.push(GcodeMove { x, y, z, extruded });
    }
    Ok(GcodePreview {
        layers,
        extrusion_mm,
        deposited_volume_mm3: 0.0,
        moves,
    })
}
