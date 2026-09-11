//! Layered toolpaths from already-cut contours. Not a CAD kernel and not a
//! print process.
//!
//! The host (CAD) sections a mesh and passes rings. This crate offsets walls,
//! hatches infill, and can encode the plan through `gcode-core`. Coordinates
//! are millimeters.

use planar_geometry::rings::{self as rings, Rings};

pub use gcode_core::{GcodeMove, GcodePreview};

pub const MAX_LAYERS: usize = 2_048;

pub use math_core::{Error, Result};

/// One horizontal slice: closed rings in millimeters. Not a CAD handle.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerSection {
    pub z_mm: f64,
    pub contours: Vec<Vec<[f64; 2]>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathRole {
    Outline,
    Inset,
    Hatch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Toolpath {
    pub role: PathRole,
    pub points: Vec<[f64; 2]>,
    pub closed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToolpathLayer {
    pub z_mm: f64,
    pub paths: Vec<Toolpath>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToolpathSettings {
    pub layer_height_mm: f64,
    pub line_width_mm: f64,
    pub wall_count: usize,
    pub infill_spacing_mm: f64,
    pub feedrate_mm_s: f64,
    pub travel_feedrate_mm_s: f64,
    /// Used only when emitting extrusion-style G-code (`E`).
    pub filament_diameter_mm: f64,
}

impl Default for ToolpathSettings {
    fn default() -> Self {
        Self {
            layer_height_mm: 0.2,
            line_width_mm: 0.4,
            wall_count: 2,
            infill_spacing_mm: 2.0,
            feedrate_mm_s: 50.0,
            travel_feedrate_mm_s: 120.0,
            filament_diameter_mm: 1.75,
        }
    }
}

fn invalid(code: &'static str, message: &str) -> Error {
    Error::new(code, message)
}

fn require_settings(settings: &ToolpathSettings) -> Result<()> {
    if ![
        settings.layer_height_mm,
        settings.line_width_mm,
        settings.infill_spacing_mm,
        settings.feedrate_mm_s,
        settings.travel_feedrate_mm_s,
        settings.filament_diameter_mm,
    ]
    .iter()
    .all(|value| value.is_finite() && *value > 0.0)
        || settings.wall_count == 0
        || settings.wall_count > 8
    {
        return Err(invalid(
            "TOOLPATH_INVALID_SETTINGS",
            "Toolpath settings are out of range",
        ));
    }
    Ok(())
}

fn rings_of(section: &LayerSection) -> Rings {
    section.contours.clone()
}

fn offset_rings(source: &Rings, distance: f64) -> Result<Rings> {
    rings::offset(source, distance, false, 8)
}

fn path_from_ring(role: PathRole, ring: &[[f64; 2]], closed: bool) -> Toolpath {
    Toolpath {
        role,
        points: ring.to_vec(),
        closed,
    }
}

fn cross2(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

fn winding(point: [f64; 2], source: &Rings) -> i32 {
    let mut winding = 0;
    for ring in source {
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            let c = cross2(
                [b[0] - a[0], b[1] - a[1]],
                [point[0] - a[0], point[1] - a[1]],
            );
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

fn hatch(source: &Rings, spacing: f64) -> Vec<Toolpath> {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for ring in source {
        for point in ring {
            xs.push(point[0]);
            ys.push(point[1]);
        }
    }
    if xs.is_empty() {
        return Vec::new();
    }
    let min_x = xs.iter().copied().fold(f64::INFINITY, f64::min);
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
        for ring in source {
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
            if winding(mid, source) != 0 {
                paths.push(Toolpath {
                    role: PathRole::Hatch,
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

/// Offset contours and hatch one already-computed layer section.
pub fn plan_layer(section: &LayerSection, settings: &ToolpathSettings) -> Result<ToolpathLayer> {
    require_settings(settings)?;
    if !section.z_mm.is_finite() {
        return Err(invalid(
            "TOOLPATH_INVALID_HEIGHT",
            "Layer height must be finite",
        ));
    }
    let mut paths = Vec::new();
    for wall in 0..settings.wall_count {
        let inset = -(wall as f64 + 0.5) * settings.line_width_mm;
        let offset = offset_rings(&rings_of(section), inset)?;
        let role = if wall == 0 {
            PathRole::Outline
        } else {
            PathRole::Inset
        };
        for ring in &offset {
            if ring.len() >= 3 {
                paths.push(path_from_ring(role, ring, true));
            }
        }
    }
    let remaining = offset_rings(
        &rings_of(section),
        -(settings.wall_count as f64) * settings.line_width_mm,
    )?;
    paths.extend(hatch(&remaining, settings.infill_spacing_mm));
    Ok(ToolpathLayer {
        z_mm: section.z_mm,
        paths,
    })
}

/// Walk a height range. `section_at` comes from the host (CAD mesh section).
pub fn schedule_layers<F>(
    mut section_at: F,
    z_min: f64,
    z_max: f64,
    settings: &ToolpathSettings,
) -> Result<Vec<ToolpathLayer>>
where
    F: FnMut(f64) -> Result<LayerSection>,
{
    require_settings(settings)?;
    if !z_min.is_finite() || !z_max.is_finite() || z_max <= z_min {
        return Err(invalid(
            "TOOLPATH_INVALID_RANGE",
            "Layer range must be finite and increasing",
        ));
    }
    let mut layers = Vec::new();
    let mut z = z_min;
    while z < z_max {
        let section = section_at(z)?;
        if !section.contours.is_empty() {
            layers.push(plan_layer(&section, settings)?);
        }
        z += settings.layer_height_mm;
        if layers.len() > MAX_LAYERS {
            return Err(invalid(
                "TOOLPATH_LAYER_LIMIT",
                "Toolpath plan exceeded 2048 layers",
            ));
        }
    }
    Ok(layers)
}

fn machine_profile(settings: &ToolpathSettings) -> gcode_core::MachineProfile {
    gcode_core::MachineProfile {
        layer_height_mm: settings.layer_height_mm,
        line_width_mm: settings.line_width_mm,
        print_feedrate_mm_s: settings.feedrate_mm_s,
        travel_feedrate_mm_s: settings.travel_feedrate_mm_s,
        filament_diameter_mm: settings.filament_diameter_mm,
    }
}

fn planned_layers(layers: &[ToolpathLayer]) -> Vec<gcode_core::PlannedLayer> {
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
    error
}

/// One encoding of a planned toolpath. Not the only possible machine dialect.
pub fn emit_gcode(layers: &[ToolpathLayer], settings: &ToolpathSettings) -> Result<String> {
    require_settings(settings)?;
    gcode_core::emit(&planned_layers(layers), &machine_profile(settings)).map_err(gcode_error)
}

pub fn parse_gcode_preview(gcode: &str) -> Result<GcodePreview> {
    gcode_core::parse(gcode).map_err(gcode_error)
}

/// Extruded volume when the plan carries filament diameter. Zero-meaning for
/// non-extrusion machines.
pub fn deposited_volume_mm3(layers: &[ToolpathLayer], settings: &ToolpathSettings) -> f64 {
    gcode_core::deposited_volume_mm3(&planned_layers(layers), &machine_profile(settings))
}
