//! Appearance operations at the JSON boundary. Algorithms live in the geometry crates.
use super::codec::{
    decode_gradient, decode_path, decode_region, encode_colored_mesh, fill_rule, stroke_options,
};
use crate::Result;
use crate::encode;
use crate::field;
use value_codec::Value;

pub(super) fn dispatch(action: &str, v: &Value) -> Result<Option<Value>> {
    let output = match action {
        "recolor" => {
            use polygon_core::appearance::{Color, shift_color};
            let c = field::<[u8; 4]>(v, "color")?;
            let out = shift_color(
                Color::from_rgba(c),
                field(v, "hue")?,
                field(v, "sat")?,
                field(v, "light")?,
            );
            encode(out.to_rgba())?
        }
        "gradient_fill_mesh" => {
            let mesh = polygon_core::gradient_mesh::gradient_fill_rings(
                &decode_region(v)?,
                &decode_gradient(v)?,
                fill_rule(v)? == planar_geometry::tessellation::FillRule::EvenOdd,
            )?;
            encode_colored_mesh(&mesh)
        }
        "gradient_stroke_mesh" => {
            use polygon_core::gradient_mesh::{GradientSampleMode, gradient_stroke_mesh};
            let opts = stroke_options(v)?;
            let mode = match v["mode"].as_str().unwrap_or("spatial") {
                "along" | "alongPath" | "AlongPath" => GradientSampleMode::AlongPath,
                _ => GradientSampleMode::Spatial,
            };
            encode_colored_mesh(&gradient_stroke_mesh(
                &decode_path(&v["path"])?,
                &opts,
                &decode_gradient(v)?,
                mode,
            )?)
        }
        _ => return Ok(None),
    };
    Ok(Some(output))
}
