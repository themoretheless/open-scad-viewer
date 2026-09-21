//! Effects operations at the JSON boundary. Algorithms live in the geometry crates.
use super::codec::{
    decode_path, decode_paths, decode_rings, encode_path, encode_paths, fill_rule, offset_options,
    optional_holes,
};
use crate::Result;
use crate::encode;
use crate::field;
use planar_geometry::effects;
use planar_geometry::effects::ArcMode;
use planar_geometry::effects::StippleKind;
use value_codec::Value;

pub(super) fn dispatch(action: &str, v: &Value) -> Result<Option<Value>> {
    let output = match action {
        "concentric_offset" => encode(effects::concentric_offset_with_options(
            &field(v, "rings")?,
            field(v, "count")?,
            &offset_options(v, field(v, "step")?)?,
        )?)?,
        "spiral" => encode_path(&effects::spiral(
            field(v, "min")?,
            field(v, "max")?,
            field(v, "turns")?,
            v["innerRatio"].as_f64().unwrap_or(0.0),
            v["clockwise"].as_bool().unwrap_or(true),
        )?),
        "polar_grid" => encode_paths(&effects::polar_grid(
            field(v, "min")?,
            field(v, "max")?,
            field(v, "circles")?,
            field(v, "spokes")?,
            v["innerRatio"].as_f64().unwrap_or(0.0),
            v["fullSpokes"].as_bool().unwrap_or(true),
        )?),
        "step_and_repeat" => encode_paths(&effects::step_and_repeat(
            &decode_path(&v["path"])?,
            field(v, "count")?,
            field(v, "delta")?,
        )?),
        "radial_repeat" => encode_paths(&effects::radial_repeat(
            &decode_path(&v["path"])?,
            field(v, "count")?,
            field(v, "center")?,
        )?),
        "grid_array" => encode_paths(&effects::grid_array(
            &decode_path(&v["path"])?,
            field(v, "rows")?,
            field(v, "cols")?,
            field(v, "spacing")?,
        )?),
        "step_and_repeat_paths" => encode_paths(&effects::step_and_repeat_paths(
            &decode_paths(&v["paths"])?,
            field(v, "count")?,
            field(v, "delta")?,
        )?),
        "radial_repeat_paths" => encode_paths(&effects::radial_repeat_paths(
            &decode_paths(&v["paths"])?,
            field(v, "count")?,
            field(v, "center")?,
            field(v, "angleStep")?,
            v["rotateCopies"].as_bool().unwrap_or(true),
        )?),
        "grid_array_paths" => encode_paths(&effects::grid_array_paths(
            &decode_paths(&v["paths"])?,
            field(v, "rows")?,
            field(v, "cols")?,
            field(v, "spacing")?,
        )?),
        "hatch" => {
            let rule = fill_rule(v)?;
            let spacing = field(v, "spacing")?;
            let angle = v["angle"].as_f64().unwrap_or(0.0);
            let cross = v["cross"].as_bool().unwrap_or(false);
            if v.get("path").is_some_and(|p| !p.is_null()) {
                let holes = optional_holes(v)?;
                encode_paths(&effects::hatch_path(
                    &decode_path(&v["path"])?,
                    &holes,
                    rule,
                    spacing,
                    angle,
                    cross,
                    v["tolerance"].as_f64().unwrap_or(0.25),
                )?)
            } else if v.get("rings").is_some_and(|r| !r.is_null()) {
                encode_paths(&effects::hatch_rings(
                    &decode_rings(&v["rings"])?,
                    rule,
                    spacing,
                    angle,
                    cross,
                )?)
            } else {
                encode_paths(&effects::hatch_rings(
                    &[field::<Vec<[f64; 2]>>(v, "ring")?],
                    rule,
                    spacing,
                    angle,
                    cross,
                )?)
            }
        }
        "stipple" => {
            let kind = match v["kind"].as_str().unwrap_or("dot") {
                "ring" => StippleKind::Ring,
                "cross" => StippleKind::Cross,
                "square" => StippleKind::Square,
                _ => StippleKind::Dot,
            };
            let opts = effects::StippleOptions {
                spacing: field(v, "spacing")?,
                size: field(v, "size")?,
                kind,
                jitter: v["jitter"].as_f64().unwrap_or(0.0),
                seed: v["seed"].as_u64().unwrap_or(1),
                fill_rule: fill_rule(v)?,
            };
            if v.get("path").is_some_and(|p| !p.is_null()) {
                let holes = optional_holes(v)?;
                encode_paths(&effects::stipple_path(
                    &decode_path(&v["path"])?,
                    &holes,
                    &opts,
                    v["tolerance"].as_f64().unwrap_or(0.25),
                )?)
            } else if v.get("rings").is_some_and(|r| !r.is_null()) {
                encode_paths(&effects::stipple_rings(&decode_rings(&v["rings"])?, &opts)?)
            } else {
                encode_paths(&effects::stipple_rings(
                    &[field::<Vec<[f64; 2]>>(v, "ring")?],
                    &opts,
                )?)
            }
        }
        "zig_zag" => {
            let path = decode_path(&v["path"])?;
            encode_path(&if v.get("ridges").is_some_and(|r| !r.is_null()) {
                effects::zig_zag_with_options(
                    &path,
                    &effects::ZigZagOptions {
                        amplitude: field(v, "amplitude")?,
                        ridges: field(v, "ridges")?,
                        smooth: v["smooth"].as_bool().unwrap_or(false),
                        tolerance: v["tolerance"].as_f64().unwrap_or(0.25),
                    },
                )?
            } else {
                effects::zig_zag(&path, field(v, "amplitude")?, field(v, "wavelength")?)?
            })
        }
        "pucker_bloat" => encode_path(&effects::pucker_bloat(
            &decode_path(&v["path"])?,
            field(v, "amount")?,
        )?),
        "roughen" => encode_path(&effects::roughen_with_options(
            &decode_path(&v["path"])?,
            &effects::RoughenOptions {
                amplitude: field(v, "amount")?,
                detail: v["detail"].as_u64().unwrap_or(4) as usize,
                smooth: v["smooth"].as_bool().unwrap_or(false),
                seed: v["seed"].as_u64().unwrap_or(1),
            },
        )?),
        "twist" => encode_path(&effects::twist(
            &decode_path(&v["path"])?,
            field(v, "radians")?,
        )?),
        "scatter" => encode_paths(&effects::scatter(
            &decode_path(&v["path"])?,
            field(v, "count")?,
            field(v, "radius")?,
            v["seed"].as_u64().unwrap_or(1),
        )?),
        "scatter_paths" => encode_paths(&effects::scatter_paths(
            &decode_paths(&v["paths"])?,
            &effects::ScatterOptions {
                count: field(v, "count")?,
                position: v["position"].as_f64().unwrap_or(0.),
                rotation_radians: v["rotationRadians"].as_f64().unwrap_or(0.),
                scale_min: v["scaleMin"].as_f64().unwrap_or(1.),
                scale_max: v["scaleMax"].as_f64().unwrap_or(1.),
                seed: v["seed"].as_u64().unwrap_or(0),
            },
        )?),
        "blend" => encode_paths(&effects::blend(
            &decode_path(&v["a"])?,
            &decode_path(&v["b"])?,
            field(v, "steps")?,
        )?),
        "free_distort" => encode_path(&effects::free_distort(
            &decode_path(&v["path"])?,
            field(v, "quad")?,
        )?),
        "arc" => {
            let mode = match v["mode"].as_str().unwrap_or("arc") {
                "pie" => ArcMode::Pie,
                "segment" => ArcMode::Segment,
                _ => ArcMode::Arc,
            };
            encode_path(&effects::arc_path(
                field(v, "min")?,
                field(v, "max")?,
                field(v, "start")?,
                field(v, "sweep")?,
                mode,
            )?)
        }
        "star" => encode_path(&effects::star(
            field(v, "min")?,
            field(v, "max")?,
            field(v, "points")?,
            v["innerRatio"].as_f64().unwrap_or(0.4),
        )?),
        _ => return Ok(None),
    };
    Ok(Some(output))
}
