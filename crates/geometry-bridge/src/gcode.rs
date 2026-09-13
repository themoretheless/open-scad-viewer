//! Bounded host adapter for mesh toolpaths and the versioned G-code preview.
use crate::{Error, Mesh, Result, field, input};
use slicer_core::{GcodePreview, ToolpathLayer, ToolpathSettings};
use value_codec::{Value, json};

const MAX_SLICE_TRIANGLES: usize = polygon_core::MAX_MESH_TRIANGLES;
const MAX_SLICE_TRIANGLE_VISITS: usize = 10_000_000;

fn setting(v: &Value, key: &str, default: f64) -> Result<f64> {
    match v.get(key) {
        None => Ok(default),
        Some(value) => value
            .as_f64()
            .filter(|n| n.is_finite() && *n > 0.0)
            .ok_or_else(|| input(format!("{key} must be a finite positive number"))),
    }
}

fn settings(v: &Value) -> Result<ToolpathSettings> {
    let defaults = ToolpathSettings::default();
    let walls = setting(v, "wallCount", defaults.wall_count as f64)?;
    if walls.fract() != 0.0 || walls > 8.0 {
        return Err(input("wallCount must be an integer between 1 and 8"));
    }
    Ok(ToolpathSettings {
        layer_height_mm: setting(v, "layerHeightMm", defaults.layer_height_mm)?,
        line_width_mm: setting(v, "lineWidthMm", defaults.line_width_mm)?,
        wall_count: walls as usize,
        infill_spacing_mm: setting(v, "infillSpacingMm", defaults.infill_spacing_mm)?,
        feedrate_mm_s: setting(v, "feedrateMmS", defaults.feedrate_mm_s)?,
        travel_feedrate_mm_s: setting(v, "travelFeedrateMmS", defaults.travel_feedrate_mm_s)?,
        filament_diameter_mm: setting(v, "filamentDiameterMm", defaults.filament_diameter_mm)?,
    })
}

fn plan(v: &Value) -> Result<(Vec<ToolpathLayer>, ToolpathSettings)> {
    let settings = settings(v)?;
    let z_min = field(v, "zMin")?;
    let z_max = field(v, "zMax")?;
    let mesh: Mesh = field(v, "mesh")?;
    if mesh.indices.len() / 3 > MAX_SLICE_TRIANGLES {
        return Err(Error::new(
            "TOOLPATH_MESH_LIMIT",
            "Slicing supports at most 100000 triangles",
        ));
    }
    let index = polygon_core::solid::section::MeshSectionIndex::new(&mesh)?;
    let mut visits = 0usize;
    let layers = slicer_core::schedule_layers(
        |z| {
            let section = index.section(z)?;
            visits += section.candidate_triangles;
            if visits > MAX_SLICE_TRIANGLE_VISITS {
                return Err(Error::new(
                    "TOOLPATH_WORK_LIMIT",
                    "Slicing exceeded 10000000 triangle visits",
                ));
            }
            Ok(planar_geometry::LayerSection {
                z_mm: section.z_mm,
                contours: section
                    .contours
                    .into_iter()
                    .map(|contour| contour.points)
                    .collect(),
            })
        },
        z_min,
        z_max,
        &settings,
    )?;
    Ok((layers, settings))
}

pub(crate) fn toolpaths(v: &Value) -> Result<Value> {
    let (layers, _) = plan(v)?;
    Ok(json!({
        "layers": layers.iter().map(|layer| json!({
            "z_mm": layer.z_mm,
            "paths": layer.paths.iter().map(|path| json!({
                "role": match path.role {
                    slicer_core::PathRole::Outline => "outline",
                    slicer_core::PathRole::Inset => "inset",
                    slicer_core::PathRole::Hatch => "hatch",
                },
                "closed": path.closed,
                "points": path.points,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    }))
}

fn preview_value(preview: GcodePreview) -> Value {
    json!({
        "layers": preview.layers,
        "extrusionMm": preview.extrusion_mm,
        "depositedVolumeMm3": preview.deposited_volume_mm3,
        "bounds": preview.bounds.map(|b| json!({"min": b.min, "max": b.max})),
        "travelDistanceMm": preview.travel_distance_mm,
        "printDistanceMm": preview.print_distance_mm,
        "estimatedTimeS": preview.estimated_time_s,
        "moves": preview.moves.iter().map(|m| json!({
            "x": m.x, "y": m.y, "z": m.z, "e": m.e,
            "extruded": m.extruded, "feedrateMmS": m.feedrate_mm_s,
            "layerIndex": m.layer_index,
        })).collect::<Vec<_>>(),
    })
}

pub(crate) fn export(v: &Value) -> Result<Value> {
    let (layers, settings) = plan(v)?;
    if !layers.iter().any(|layer| !layer.paths.is_empty()) {
        return Err(Error::new(
            "GCODE_EMPTY_PLAN",
            "No toolpaths in the selected height range; check the model, range and line width",
        ));
    }
    let gcode = slicer_core::emit_gcode(&layers, &settings)?;
    // Validate the actual file before publishing it or displaying its statistics.
    let preview = slicer_core::parse_gcode_preview(&gcode)?;
    if preview.print_distance_mm <= 0.0 || preview.extrusion_mm <= 0.0 {
        return Err(Error::new(
            "GCODE_EMPTY_PLAN",
            "No extrusion remains at G-code output precision",
        ));
    }
    Ok(json!({
        "gcode": gcode,
        "dialect": slicer_core::GCODE_DIALECT,
        "layerCount": layers.len(),
        "preview": preview_value(preview),
    }))
}

pub(crate) fn parse(v: &Value) -> Result<Value> {
    let text = v
        .get("gcode")
        .and_then(Value::as_str)
        .ok_or_else(|| input("gcode must be a string"))?;
    Ok(preview_value(slicer_core::parse_gcode_preview(text)?))
}
