//! Native workbench section selection, placement, matching and mesh construction.
use super::{Result, Value, cad_path, cad_sketch, encode, field, input};
fn placed(sketch: &Value, close: bool) -> Result<Vec<[f64; 3]>> {
    let mut points: Vec<[f64; 2]> = field(sketch, "points")?;
    if !(2..=512).contains(&points.len()) {
        return Err(input("Section requires 2–512 points."));
    }
    if close {
        points.push(points[0]);
    }
    let plane = sketch
        .get("plane")
        .cloned()
        .unwrap_or_else(|| value_codec::json!({"origin":[0.,0.,0.],"u":[1.,0.,0.],"v":[0.,1.,0.]}));
    let value = cad_sketch::world_points(value_codec::json!({"points":points,"plane":plane}))?;
    value_codec::from_value(value).map_err(|e| input(e.to_string()))
}
pub fn build(v: Value) -> Result<Value> {
    let sketches: Vec<Value> = field(&v, "sketches")?;
    let ids: Vec<String> = field(&v, "profileIds")?;
    let action: String = field(&v, "action")?;
    let mut profiles = Vec::new();
    for id in ids {
        if let Some(s) = sketches
            .iter()
            .find(|s| s.get("id").and_then(Value::as_str) == Some(id.as_str()))
        {
            profiles.push(s);
        }
    }
    let built = match action.as_str() {
        "loft" => {
            if !(2..=64).contains(&profiles.len()) {
                return Err(input("Choose 2–64 closed sections in loft order."));
            }
            let mut count = 0;
            for s in &profiles {
                if !field::<bool>(s, "closed")? {
                    return Err(input("Choose closed sections in loft order."));
                }
                let points: Vec<[f64; 2]> = field(s, "points")?;
                count = count.max(points.len());
            }
            if !(3..=512).contains(&count) {
                return Err(input("Loft requires 3–512 points per section."));
            }
            let fractions: Vec<f64> = (0..count).map(|i| i as f64 / count as f64).collect();
            let mut sections = Vec::with_capacity(profiles.len());
            for s in profiles {
                let path = placed(s, true)?;
                let sampled =
                    cad_path::sample(value_codec::json!({"path":path,"fractions":fractions}))?;
                let section: Vec<[f64; 3]> =
                    value_codec::from_value(sampled).map_err(|e| input(e.to_string()))?;
                sections.push(section);
            }
            polygon_core::solid::modeling::loft(&sections, true)?
        }
        "sweep" => {
            let id: String = field(&v, "pathId")?;
            let profile = profiles
                .first()
                .ok_or_else(|| input("Choose a closed profile and an open path."))?;
            let path = sketches
                .iter()
                .find(|s| s.get("id").and_then(Value::as_str) == Some(id.as_str()))
                .ok_or_else(|| input("Choose a closed profile and an open path."))?;
            if !field::<bool>(profile, "closed")? || field::<bool>(path, "closed")? {
                return Err(input("Choose a closed profile and an open path."));
            }
            let points: Vec<[f64; 2]> = field(profile, "points")?;
            let up: [f64; 3] = match profile.get("plane") {
                Some(p) => field(p, "u")?,
                None => [1., 0., 0.],
            };
            polygon_core::solid::modeling::sweep(&points, &placed(path, false)?, up, true)?
        }
        _ => return Err(input("Invalid section operation.")),
    };
    let report = built.mesh.inspect()?;
    if !report.closed || report.signed_volume_mm3 <= 0. {
        return Err(input("Operation did not produce a closed solid."));
    }
    encode(built.mesh)
}

/// Preserve authored polygon vertices for ruled B-rep correspondence.
pub fn ruled(v: Value) -> Result<Value> {
    let sketches: Vec<Value> = field(&v, "sketches")?;
    let ids: Vec<String> = field(&v, "ids")?;
    if !(2..=64).contains(&ids.len()) {
        return Err(input("Select 2–64 closed polygon sketches in loft order."));
    }
    let mut sections = Vec::with_capacity(ids.len());
    for id in ids {
        let sketch = sketches
            .iter()
            .find(|s| s.get("id").and_then(Value::as_str) == Some(id.as_str()))
            .ok_or_else(|| input("Select only closed polygon sketches for ruled loft."))?;
        if !field::<bool>(sketch, "closed")? || sketch.get("analytic").is_some() {
            return Err(input(
                "Ruled loft requires closed polygon sketches; analytic curves are not supported.",
            ));
        }
        sections.push(placed(sketch, false)?);
    }
    let model = brep_core::ruled_loft(&sections)?;
    let mesh = super::brep::nurbs(&model, 4)?.built.mesh;
    Ok(value_codec::json!({"brep":model,"mesh":mesh}))
}
