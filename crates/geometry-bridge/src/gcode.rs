//! Bounded host adapter for mesh toolpaths, preview G-code, and print jobs.
use crate::{Error, Mesh, Result, field, input};
use base64::Engine;
use slicer_core::{
    Flavor, GcodePreview, JobProfile, MeshBody, OptimizeSettings, ToolpathLayer, ToolpathSettings,
};
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

fn optional_non_negative(v: &Value, key: &str, default: f64) -> Result<f64> {
    match v.get(key) {
        None => Ok(default),
        Some(value) => value
            .as_f64()
            .filter(|n| n.is_finite() && *n >= 0.0)
            .ok_or_else(|| input(format!("{key} must be a finite non-negative number"))),
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

fn optimize_settings(v: &Value) -> Result<OptimizeSettings> {
    let defaults = OptimizeSettings::default();
    Ok(OptimizeSettings {
        simplify_tolerance_mm: optional_non_negative(
            v,
            "simplifyToleranceMm",
            defaults.simplify_tolerance_mm,
        )?,
        retract_min_travel_mm: optional_non_negative(
            v,
            "retractMinTravelMm",
            defaults.retract_min_travel_mm,
        )?,
        max_2opt_swaps: match v.get("max2optSwaps") {
            None => defaults.max_2opt_swaps,
            Some(value) => {
                let n = value
                    .as_f64()
                    .filter(|n| n.is_finite() && *n >= 0.0 && n.fract() == 0.0 && *n <= 10_000.0)
                    .ok_or_else(|| input("max2optSwaps must be an integer 0..10000"))?;
                n as usize
            }
        },
        ..defaults
    })
}

fn job_profile(v: &Value, toolpath: &ToolpathSettings) -> Result<JobProfile> {
    let defaults = JobProfile::default();
    let fan = match v.get("fanSpeed") {
        None => defaults.fan_speed,
        Some(value) => {
            let n = value
                .as_f64()
                .filter(|n| n.is_finite() && (0.0..=255.0).contains(n) && n.fract() == 0.0)
                .ok_or_else(|| input("fanSpeed must be an integer 0..255"))?;
            n as u8
        }
    };
    let home_axes = match v.get("homeAxes") {
        None => defaults.home_axes,
        Some(Value::Bool(flag)) => *flag,
        Some(_) => return Err(input("homeAxes must be a boolean")),
    };
    let flavor = match v.get("flavor") {
        None => defaults.flavor,
        Some(value) => {
            let name = value
                .as_str()
                .ok_or_else(|| input("flavor must be a string"))?;
            Flavor::from_name(name)
                .map_err(|_| input("flavor must be marlin, klipper or reprapfirmware"))?
        }
    };
    slicer_core::job_profile(
        toolpath,
        JobProfile {
            machine: Default::default(),
            nozzle_temp_c: setting(v, "nozzleTempC", defaults.nozzle_temp_c)?,
            bed_temp_c: setting(v, "bedTempC", defaults.bed_temp_c)?,
            retract_length_mm: setting(v, "retractLengthMm", defaults.retract_length_mm)?,
            retract_feedrate_mm_s: setting(
                v,
                "retractFeedrateMmS",
                defaults.retract_feedrate_mm_s,
            )?,
            unretract_feedrate_mm_s: setting(
                v,
                "unretractFeedrateMmS",
                defaults.unretract_feedrate_mm_s,
            )?,
            retract_min_travel_mm: optional_non_negative(
                v,
                "retractMinTravelMm",
                defaults.retract_min_travel_mm,
            )?,
            fan_speed: fan,
            home_axes,
            flavor,
        },
    )
}

fn plan(v: &Value) -> Result<(Vec<ToolpathLayer>, ToolpathSettings, Mesh)> {
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
    Ok((layers, settings, mesh))
}

fn require_extrusion(layers: &[ToolpathLayer]) -> Result<()> {
    if !layers.iter().any(|layer| !layer.paths.is_empty()) {
        return Err(Error::new(
            "GCODE_EMPTY_PLAN",
            "No toolpaths in the selected height range; check the model, range and line width",
        ));
    }
    Ok(())
}

pub(crate) fn toolpaths(v: &Value) -> Result<Value> {
    let (layers, _, _) = plan(v)?;
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

fn packed_moves(v: &Value) -> Result<bool> {
    match v.get("packedMoves") {
        None => Ok(false),
        Some(value) => value
            .as_bool()
            .ok_or_else(|| input("packedMoves must be boolean")),
    }
}

fn preview_value(preview: GcodePreview, packed: bool) -> Value {
    let mut value = json!({
        "layers": preview.layers,
        "extrusionMm": preview.extrusion_mm,
        "depositedVolumeMm3": preview.deposited_volume_mm3,
        "bounds": preview.bounds.map(|b| json!({"min": b.min, "max": b.max})),
        "travelDistanceMm": preview.travel_distance_mm,
        "printDistanceMm": preview.print_distance_mm,
        "estimatedTimeS": preview.estimated_time_s,
    });
    if packed {
        value["moveRows"] = json!(
            preview
                .moves
                .iter()
                .flat_map(|m| [
                    m.x,
                    m.y,
                    m.z,
                    m.e,
                    m.feedrate_mm_s,
                    m.layer_index as f64,
                    if m.extruded { 1.0 } else { 0.0 },
                ])
                .collect::<Vec<_>>()
        );
    } else {
        value["moves"] = json!(
            preview
                .moves
                .iter()
                .map(|m| json!({
                    "x": m.x, "y": m.y, "z": m.z, "e": m.e,
                    "extruded": m.extruded, "feedrateMmS": m.feedrate_mm_s,
                    "layerIndex": m.layer_index,
                }))
                .collect::<Vec<_>>()
        );
    }
    value
}

fn require_positive_extrusion(preview: &GcodePreview) -> Result<()> {
    if preview.print_distance_mm <= 0.0 || preview.extrusion_mm <= 0.0 {
        return Err(Error::new(
            "GCODE_EMPTY_PLAN",
            "No extrusion remains at G-code output precision",
        ));
    }
    Ok(())
}

/// Preview dialect after `slicer-core` → `gcode-optimize` → `emit`.
pub(crate) fn export(v: &Value) -> Result<Value> {
    let packed = packed_moves(v)?;
    let (layers, settings, _) = plan(v)?;
    require_extrusion(&layers)?;
    let optimize = optimize_settings(v)?;
    let gcode = slicer_core::emit_optimized_gcode(&layers, &settings, &optimize)?;
    let preview = slicer_core::parse_gcode_preview(&gcode)?;
    require_positive_extrusion(&preview)?;
    Ok(json!({
        "gcode": gcode,
        "dialect": slicer_core::GCODE_DIALECT,
        "layerCount": layers.len(),
        "preview": preview_value(preview, packed),
    }))
}

fn mesh_body(mesh: &Mesh) -> MeshBody {
    MeshBody {
        positions: mesh.positions.clone(),
        indices: mesh.indices.clone(),
    }
}

/// Job dialect + thick `.gcode.3mf` after optimize.
pub(crate) fn export_job(v: &Value) -> Result<Value> {
    let packed = packed_moves(v)?;
    let (layers, settings, mesh) = plan(v)?;
    require_extrusion(&layers)?;
    let optimize = optimize_settings(v)?;
    let job = job_profile(v, &settings)?;
    let gcode = slicer_core::emit_job_gcode(&layers, &job, &optimize)?;
    let preview = slicer_core::parse_gcode_job(&gcode)?;
    require_positive_extrusion(&preview)?;
    let packaged =
        slicer_core::emit_job_gcode_3mf(&layers, &job, &optimize, Some(&mesh_body(&mesh)))?;
    Ok(json!({
        "gcode": gcode,
        "gcode3mfBase64": base64::engine::general_purpose::STANDARD.encode(packaged),
        "dialect": slicer_core::GCODE_JOB_DIALECT,
        "flavor": job.flavor.name(),
        "layerCount": layers.len(),
        "preview": preview_value(preview, packed),
    }))
}

fn gcode_text(v: &Value) -> Result<&str> {
    v.get("gcode")
        .and_then(Value::as_str)
        .ok_or_else(|| input("gcode must be a string"))
}

/// Flat preview: strict for native dialects, tolerant for other slicers' files.
pub(crate) fn parse(v: &Value) -> Result<Value> {
    let packed = packed_moves(v)?;
    let (preview, _) = slicer_core::parse_gcode_any(gcode_text(v)?)?;
    Ok(preview_value(preview, packed))
}

/// Preview plus detected dialect, generator and flavor for opened files.
pub(crate) fn inspect(v: &Value) -> Result<Value> {
    let packed = packed_moves(v)?;
    let text = gcode_text(v)?;
    let (preview, info) = slicer_core::parse_gcode_any(text)?;
    let first = text.lines().next().unwrap_or("").trim();
    let dialect = if first == format!("; {}", slicer_core::GCODE_JOB_DIALECT) {
        slicer_core::GCODE_JOB_DIALECT.to_string()
    } else if first == format!("; {}", slicer_core::GCODE_DIALECT) {
        slicer_core::GCODE_DIALECT.to_string()
    } else {
        format!("{} G-code (tolerant preview)", info.generator.name())
    };
    Ok(json!({
        "preview": preview_value(preview, packed),
        "dialect": dialect,
        "native": info.native,
        "generator": info.generator.name(),
        "flavor": info.flavor,
        "filamentDiameterMm": info.filament_diameter_mm,
    }))
}
