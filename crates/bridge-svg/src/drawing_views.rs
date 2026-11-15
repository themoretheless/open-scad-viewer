//! `drawing_views`: orthographic hidden-line views, hatched section and
//! overhang faces of one triangle mesh (`mesh_section::drawing`).
use crate::{Result, Value};
use mesh_section::MeshView;
use mesh_section::drawing::{View, ViewOptions, hatched_section, overhangs, view_edges};
use value_codec::json;

pub(crate) fn run(v: &Value) -> Result<Value> {
    let positions: Vec<f64> = bridge_codec::field(v, "positions")?;
    let indices: Vec<usize> = bridge_codec::field(v, "indices")?;
    let names: Vec<String> = bridge_codec::field(v, "views")?;
    if names.is_empty() || names.len() > 4 || indices.len() / 3 > 200_000 {
        return Err(bridge_codec::input(
            "Drawing needs 1-4 views and at most 200000 triangles",
        ));
    }
    let mesh = MeshView::new(&positions, &indices);
    let options = ViewOptions {
        feature_angle_deg: v
            .get("featureAngleDeg")
            .and_then(Value::as_f64)
            .unwrap_or(30.),
        ..ViewOptions::default()
    };
    let mut views = Vec::new();
    for name in &names {
        let segments = view_edges(&mesh, View::parse(name)?, options)?;
        let list: Vec<Value> = segments
            .iter()
            .map(|s| json!({"a":s.a,"b":s.b,"hidden":s.hidden,"silhouette":s.silhouette}))
            .collect();
        views.push(json!({"view":name,"segments":list}));
    }
    let section = match v.get("section") {
        Some(s) if !s.is_null() => {
            let view: String = bridge_codec::field(s, "view")?;
            let r = hatched_section(
                &mesh,
                View::parse(&view)?,
                bridge_codec::field(s, "offset")?,
                bridge_codec::field(s, "spacing")?,
            )?;
            json!({"view":view,"rings":r.rings,"hatch":r.hatch,"area":r.area})
        }
        _ => Value::Null,
    };
    let angle = v
        .get("overhangAngleDeg")
        .and_then(Value::as_f64)
        .unwrap_or(45.);
    let o = overhangs(&mesh, angle, 1e-6)?;
    Ok(json!({"views":views,"section":section,
        "overhang":{"thresholdDeg":o.threshold_deg,"area":o.area,"triangles":o.triangles}}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routes_views_section_and_overhang() {
        let positions = vec![0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1.];
        let indices = vec![0, 2, 1, 0, 1, 3, 1, 2, 3, 0, 3, 2];
        let r = run(
            &json!({"positions":positions,"indices":indices,"views":["front","iso"],
            "section":{"view":"top","offset":0.5,"spacing":0.1}}),
        )
        .unwrap();
        assert_eq!(r["views"].as_array().unwrap().len(), 2);
        assert!(r["section"]["area"].as_f64().unwrap() > 0.);
        assert!(r["overhang"]["triangles"].as_array().unwrap().is_empty());
        assert!(run(&json!({"positions":positions,"indices":indices,"views":["side"]})).is_err());
        assert!(run(&json!({"positions":positions,"indices":indices,"views":[]})).is_err());
    }
}
