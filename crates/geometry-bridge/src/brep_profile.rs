//! Retained 2D analytic profiles. No temporary solids or mesh construction.
use super::{Result, Value, encode, field, input};
use brep_core::planar_trim;
use nurbs_core::curve::Curve;
use value_codec::json;

pub(crate) fn profile(loops: Vec<Vec<Curve>>, tolerance: f64) -> Result<Value> {
    planar_trim::validate(&loops, tolerance)?;
    let area = loops.iter().try_fold(0., |sum, wire| {
        Ok::<_, super::Error>(sum + planar_trim::signed_area(wire, tolerance)?)
    })?;
    Ok(json!({
        "kind":"brep-profile", "loops":loops, "areaMm2":area,
        "toleranceMm":tolerance, "geometryStatus":"numerical_uncertified"
    }))
}

pub fn dispatch(v: Value) -> Result<Value> {
    let tolerance = match v.get("toleranceMm") {
        Some(_) => field::<f64>(&v, "toleranceMm")?,
        None => 1e-7,
    };
    match v["op"].as_str() {
        Some("brep_profile_transform") => profile(
            brep_core::transform::profile(
                &field::<Vec<Vec<Curve>>>(&v, "loops")?,
                field(&v, "matrix")?,
                tolerance,
            )?,
            tolerance,
        ),
        Some("brep_profile_author") => {
            let loops = match field::<String>(&v, "kind")?.as_str() {
                "circle" => vec![brep_core::sketch::circle_wire(field(&v, "radius")?)?],
                "rectangle" => {
                    let [w, h]: [f64; 2] = field(&v, "size")?;
                    if !w.is_finite() || !h.is_finite() || w <= 0. || h <= 0. {
                        return Err(input("Invalid profile rectangle size"));
                    }
                    let [x, y] = if field::<bool>(&v, "center")? {
                        [-w / 2., -h / 2.]
                    } else {
                        [0., 0.]
                    };
                    vec![brep_core::sketch::polygon_wire(vec![
                        [x, y],
                        [x + w, y],
                        [x + w, y + h],
                        [x, y + h],
                    ])?]
                }
                "polygon" => {
                    let rings: Vec<Vec<[f64; 2]>> = field(&v, "rings")?;
                    let loops = rings
                        .into_iter()
                        .map(brep_core::sketch::polygon_wire)
                        .collect::<Result<Vec<_>>>()?;
                    planar_trim::orient_even_odd(&loops, tolerance)?
                }
                _ => return Err(input("Unsupported authored profile kind")),
            };
            profile(loops, tolerance)
        }
        Some("brep_profile_validate") => {
            let loops: Vec<Vec<Curve>> = field(&v, "loops")?;
            let fill_rule = match v.get("fillRule") {
                Some(_) => field::<String>(&v, "fillRule")?,
                None => "material-left".into(),
            };
            match fill_rule.as_str() {
                "material-left" => profile(loops, tolerance),
                "even-odd" => profile(planar_trim::orient_even_odd(&loops, tolerance)?, tolerance),
                _ => Err(input("Profile fillRule must be material-left or even-odd")),
            }
        }
        Some("brep_profile_boolean") => profile(
            planar_trim::boolean(
                &field::<Vec<Vec<Curve>>>(&v, "a")?,
                &field::<Vec<Vec<Curve>>>(&v, "b")?,
                &field::<String>(&v, "operation")?,
                tolerance,
            )?,
            tolerance,
        ),
        Some("brep_profile_signed_area") => encode(planar_trim::signed_area(
            &field::<Vec<Curve>>(&v, "loop")?,
            tolerance,
        )?),
        _ => Err(input("Unknown retained profile request")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rectangle(a: [f64; 2], b: [f64; 2]) -> Vec<Curve> {
        let points = [a, [b[0], a[1]], b, [a[0], b[1]]];
        (0..4)
            .map(|i| {
                Curve::from_polyline(vec![points[i].to_vec(), points[(i + 1) % 4].to_vec()])
                    .unwrap()
            })
            .collect()
    }
    fn reverse(wire: Vec<Curve>) -> Vec<Curve> {
        wire.into_iter()
            .rev()
            .map(|c| c.reverse().unwrap())
            .collect()
    }
    #[test]
    fn even_odd_normalizes_unordered_original_rings_and_retains_holes() {
        let loops = vec![
            rectangle([2., 2.], [8., 8.]),
            reverse(rectangle([0., 0.], [10., 10.])),
            reverse(rectangle([4., 4.], [6., 6.])),
        ];
        let report =
            dispatch(json!({"op":"brep_profile_validate","loops":loops,"fillRule":"even-odd"}))
                .unwrap();
        assert_eq!(report["areaMm2"].as_f64(), Some(68.));
        assert_eq!(
            report["geometryStatus"].as_str(),
            Some("numerical_uncertified")
        );
        let normalized: Vec<Vec<Curve>> = field(&report, "loops").unwrap();
        assert_eq!(
            normalized
                .iter()
                .map(|w| planar_trim::signed_area(w, 1e-7).unwrap())
                .collect::<Vec<_>>(),
            vec![-36., 100., 4.]
        );
        assert!(dispatch(json!({"op":"brep_profile_validate","loops":loops})).is_err());
    }
    #[test]
    fn empty_and_boolean_profiles_have_no_solid_or_mesh_placeholder() {
        let a = vec![rectangle([0., 0.], [2., 2.])];
        let report =
            dispatch(json!({"op":"brep_profile_boolean","a":a,"b":a,"operation":"difference"}))
                .unwrap();
        assert_eq!(report["areaMm2"].as_f64(), Some(0.));
        assert!(report["loops"].as_array().unwrap().is_empty());
        assert!(report.get("mesh").is_none());
        assert!(report.get("vertices").is_none());
        let empty =
            dispatch(json!({"op":"brep_profile_validate","loops":[],"fillRule":"even-odd"}))
                .unwrap();
        assert_eq!(report, empty);
        assert!(dispatch(json!({"op":"brep_profile_signed_area","loop":[]})).is_err());
    }
    #[test]
    fn even_odd_refuses_crossings_and_contacting_holes() {
        let points = [[0., 2.], [1., 1.], [2., 2.], [1., 3.]];
        let touching_hole = (0..4)
            .map(|i| {
                Curve::from_polyline(vec![points[i].to_vec(), points[(i + 1) % 4].to_vec()])
                    .unwrap()
            })
            .collect::<Vec<_>>();
        for loops in [
            vec![rectangle([0., 0.], [2., 2.]), rectangle([1., 1.], [3., 3.])],
            vec![rectangle([0., 0.], [4., 4.]), rectangle([0., 1.], [2., 3.])],
            vec![rectangle([0., 0.], [4., 4.]), touching_hole],
        ] {
            assert!(
                dispatch(json!({"op":"brep_profile_validate","loops":loops,"fillRule":"even-odd"}))
                    .is_err()
            );
        }
    }
}
