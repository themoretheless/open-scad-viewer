use crate::{Result, encode, field, input};
use planar_geometry::sampled_corner::{self, Kind};
use value_codec::Value;
pub fn dispatch(v: Value) -> Result<Value> {
    let sketch = &v["sketch"];
    let points: Vec<[f64; 2]> = field(sketch, "points")?;
    if sketch["closed"].as_bool() != Some(true) {
        return Err(input(
            "Select a corner of a closed contour or close the contour before revolving.",
        ));
    }
    if v["op"].as_str() == Some("cad_sampled_corner") {
        let kind = match v["kind"].as_str() {
            Some("fillet") => Kind::Fillet,
            Some("dogear") => Kind::Dogear,
            _ => return Err(input("Unknown corner kind")),
        };
        return encode(sampled_corner::corner(
            &points,
            field(&v, "vertex")?,
            field(&v, "radius")?,
            kind,
        )?);
    }
    use polygon_core::profile_tools::{Axis, RevolveOptions, SketchPlane};
    let options = &v["options"];
    let axis = match options["axis"].as_str() {
        Some("x") => Axis::X,
        Some("y") => Axis::Y,
        _ => return Err(input("Use a nonzero angle up to 360° and 8–128 segments.")),
    };
    let plane = if let Some(p) = sketch.get("plane") {
        SketchPlane {
            origin: field(p, "origin")?,
            u: field(p, "u")?,
            v: field(p, "v")?,
        }
    } else {
        SketchPlane::default()
    };
    encode(polygon_core::profile_tools::revolve(
        &points,
        RevolveOptions {
            axis,
            offset: field(options, "offset")?,
            angle: field(options, "angle")?,
            segments: field(options, "segments")?,
        },
        plane,
    )?)
}
