//! Mesh-section print regions: walls and sparse infill.
//!
//! Input contours come from `section::MeshSection`. G-code serialization lives
//! in `gcode-core`; this module only plans paths and forwards a completed plan.

use crate::planar::rings::{self as rings, Rings};
use crate::solid::section::{MeshSection, MeshSectionIndex};
use crate::{Error, Result};

pub use gcode_core::{GcodeMove, GcodePreview};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrintRole {
    OuterWall,
    InnerWall,
    SparseInfill,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrintPath {
    pub role: PrintRole,
    pub points: Vec<[f64; 2]>,
    pub closed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrintLayer {
    pub z_mm: f64,
    pub paths: Vec<PrintPath>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrintSettings {
    pub layer_height_mm: f64,
    pub line_width_mm: f64,
    pub wall_count: usize,
    pub infill_spacing_mm: f64,
    pub print_feedrate_mm_s: f64,
    pub travel_feedrate_mm_s: f64,
    pub filament_diameter_mm: f64,
}

impl Default for PrintSettings {
    fn default() -> Self {
        Self {
            layer_height_mm: 0.2,
            line_width_mm: 0.4,
            wall_count: 2,
            infill_spacing_mm: 2.0,
            print_feedrate_mm_s: 50.0,
            travel_feedrate_mm_s: 120.0,
            filament_diameter_mm: 1.75,
        }
    }
}

fn invalid(code: &'static str, message: &str) -> Error {
    Error {
        code,
        message: message.into(),
    }
}

fn require_settings(settings: &PrintSettings) -> Result<()> {
    if ![
        settings.layer_height_mm,
        settings.line_width_mm,
        settings.infill_spacing_mm,
        settings.print_feedrate_mm_s,
        settings.travel_feedrate_mm_s,
        settings.filament_diameter_mm,
    ]
    .iter()
    .all(|value| value.is_finite() && *value > 0.0)
        || settings.wall_count == 0
        || settings.wall_count > 8
    {
        return Err(invalid("PRINT_INVALID_SETTINGS", "Print settings are out of range"));
    }
    Ok(())
}

fn rings_of(section: &MeshSection) -> Rings {
    section.contours.iter().map(|contour| contour.points.clone()).collect()
}

fn offset_rings(rings: &Rings, distance: f64) -> Result<Rings> {
    rings::offset(rings, distance, false, 8)
}

fn path_from_ring(role: PrintRole, ring: &[[f64; 2]], closed: bool) -> PrintPath {
    PrintPath {
        role,
        points: ring.to_vec(),
        closed,
    }
}

fn cross2(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

fn winding(point: [f64; 2], rings: &Rings) -> i32 {
    let mut winding = 0;
    for ring in rings {
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            let c = cross2([b[0] - a[0], b[1] - a[1]], [point[0] - a[0], point[1] - a[1]]);
            if a[1] <= point[1] && b[1] > point[1] && c > 0.0 {
                winding += 1;
            }
            if a[1] > point[1] && b[1] <= point[1] && c < 0.0 {
                winding -= 1;
            }
        }
    }
    winding
}

fn hatch(rings: &Rings, spacing: f64) -> Vec<PrintPath> {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for ring in rings {
        for point in ring {
            xs.push(point[0]);
            ys.push(point[1]);
        }
    }
    if xs.is_empty() {
        return Vec::new();
    }
    let min_x = xs.iter().copied().fold(f64::INFINITY, f64::min);
    let _max_x = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min_y = ys.iter().copied().fold(f64::INFINITY, f64::min);
    let max_y = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !min_x.is_finite() || max_y - min_y < spacing * 0.25 {
        return Vec::new();
    }
    let mut paths = Vec::new();
    let mut y = min_y + spacing * 0.5;
    let mut reverse = false;
    while y <= max_y - spacing * 0.25 {
        let mut hits = Vec::new();
        for ring in rings {
            for i in 0..ring.len() {
                let a = ring[i];
                let b = ring[(i + 1) % ring.len()];
                let (y0, y1) = (a[1], b[1]);
                if (y0 <= y && y < y1) || (y1 <= y && y < y0) {
                    let t = (y - y0) / (y1 - y0);
                    if t.is_finite() {
                        hits.push(a[0] + (b[0] - a[0]) * t);
                    }
                }
            }
        }
        hits.sort_by(|a, b| a.total_cmp(b));
        let mut pair = 0;
        while pair + 1 < hits.len() {
            let mut x0 = hits[pair];
            let mut x1 = hits[pair + 1];
            if reverse {
                std::mem::swap(&mut x0, &mut x1);
            }
            let mid = [(x0 + x1) * 0.5, y];
            if winding(mid, rings) != 0 {
                paths.push(PrintPath {
                    role: PrintRole::SparseInfill,
                    points: vec![[x0, y], [x1, y]],
                    closed: false,
                });
            }
            pair += 2;
        }
        reverse = !reverse;
        y += spacing;
    }
    paths
}

/// Build walls and sparse infill for one already-computed mesh section.
pub fn plan_layer(section: &MeshSection, settings: &PrintSettings) -> Result<PrintLayer> {
    require_settings(settings)?;
    if !section.z_mm.is_finite() {
        return Err(invalid("PRINT_INVALID_HEIGHT", "Layer height must be finite"));
    }
    let mut paths = Vec::new();
    for wall in 0..settings.wall_count {
        let inset = -(wall as f64 + 0.5) * settings.line_width_mm;
        let rings = offset_rings(&rings_of(section), inset)?;
        let role = if wall == 0 {
            PrintRole::OuterWall
        } else {
            PrintRole::InnerWall
        };
        for ring in &rings {
            if ring.len() >= 3 {
                paths.push(path_from_ring(role, ring, true));
            }
        }
    }
    let remaining = offset_rings(&rings_of(section), -(settings.wall_count as f64) * settings.line_width_mm)?;
    paths.extend(hatch(&remaining, settings.infill_spacing_mm));
    Ok(PrintLayer {
        z_mm: section.z_mm,
        paths,
    })
}

pub fn schedule_layers(
    index: &MeshSectionIndex,
    z_min: f64,
    z_max: f64,
    settings: &PrintSettings,
) -> Result<Vec<PrintLayer>> {
    require_settings(settings)?;
    if !z_min.is_finite() || !z_max.is_finite() || z_max <= z_min {
        return Err(invalid("PRINT_INVALID_RANGE", "Layer range must be finite and increasing"));
    }
    let mut layers = Vec::new();
    let mut z = z_min;
    while z < z_max {
        let section = index.section(z)?;
        if !section.contours.is_empty() {
            layers.push(plan_layer(&section, settings)?);
        }
        z += settings.layer_height_mm;
        if layers.len() > 2_048 {
            return Err(invalid("PRINT_LAYER_LIMIT", "Print preview exceeded 2048 layers"));
        }
    }
    Ok(layers)
}

fn machine_profile(settings: &PrintSettings) -> gcode_core::MachineProfile {
    gcode_core::MachineProfile {
        layer_height_mm: settings.layer_height_mm,
        line_width_mm: settings.line_width_mm,
        print_feedrate_mm_s: settings.print_feedrate_mm_s,
        travel_feedrate_mm_s: settings.travel_feedrate_mm_s,
        filament_diameter_mm: settings.filament_diameter_mm,
    }
}

fn planned_layers(layers: &[PrintLayer]) -> Vec<gcode_core::PlannedLayer> {
    layers
        .iter()
        .map(|layer| gcode_core::PlannedLayer {
            z_mm: layer.z_mm,
            paths: layer
                .paths
                .iter()
                .map(|path| gcode_core::PlannedPath {
                    points: path.points.clone(),
                    closed: path.closed,
                })
                .collect(),
        })
        .collect()
}

fn gcode_error(error: gcode_core::Error) -> Error {
    Error {
        code: error.code,
        message: error.message,
    }
}

/// Serialize a completed print plan. Units are millimeters; E is filament length.
pub fn emit_gcode(layers: &[PrintLayer], settings: &PrintSettings) -> Result<String> {
    require_settings(settings)?;
    gcode_core::emit(&planned_layers(layers), &machine_profile(settings)).map_err(gcode_error)
}

/// Independent parse of exported preview G-code. Physical print is out of scope.
pub fn parse_gcode_preview(gcode: &str) -> Result<GcodePreview> {
    gcode_core::parse(gcode).map_err(gcode_error)
}

pub fn deposited_volume_mm3(layers: &[PrintLayer], settings: &PrintSettings) -> f64 {
    gcode_core::deposited_volume_mm3(&planned_layers(layers), &machine_profile(settings))
}
