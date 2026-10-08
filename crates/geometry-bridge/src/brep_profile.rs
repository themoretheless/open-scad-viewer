//! Retained 2D analytic profiles. No temporary solids or mesh construction.
use super::{Result, Value, encode, field, input};
use brep_core::planar_trim;
use nurbs_core::curve::Curve;
use value_codec::json;

pub(crate) fn profile(loops: Vec<Vec<Curve>>, tolerance: f64) -> Result<Value> {
    let area = if loops.iter().flatten().any(|c| c.degree == 3) {
        brep_core::bezier_profile::validate_profile(&loops, tolerance)?
    } else {
        planar_trim::validate(&loops, tolerance)?;
        loops.iter().try_fold(0., |sum, wire| {
            Ok::<_, super::Error>(sum + planar_trim::signed_area(wire, tolerance)?)
        })?
    };
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
        Some("brep_profile_normalize") => profile(brep_core::bezier_profile::normalize(&field::<Vec<Vec<Curve>>>(&v,"loops")?,tolerance)?,tolerance),
        Some("brep_profile_cleanup") => {
            let loops: Vec<Vec<Curve>> = field(&v, "loops")?;
            let deviation: f64 = field(&v, "deviation")?;
            if !deviation.is_finite()
                || !(0.0001..=10.).contains(&deviation)
                || loops.is_empty()
                || loops.len() > 32
                || loops.iter().map(Vec::len).sum::<usize>() > 1022
            {
                return Err(input("Invalid retained cleanup options"));
            }
            let mut rings = Vec::new();
            let mut total = 0;
            for wire in loops {
                let mut ring = Vec::new();
                let mut endpoint: Option<[f64; 2]> = None;
                for curve in wire {
                    if curve.control_points.iter().any(|p| p.len() != 2) {
                        return Err(input("Cleanup requires planar profile curves"));
                    }
                    let report = nurbs_core::tessellation::curve_tessellation::tessellate(
                        &curve,
                        deviation * 0.5,
                        8192,
                    )?;
                    if !report.within_tolerance {
                        return Err(input(
                            "Profile cleanup tessellation did not meet its tolerance",
                        ));
                    }
                    for segment in report.segments {
                        let a = [segment.points[0][0], segment.points[0][1]];
                        let b = [segment.points[1][0], segment.points[1][1]];
                        if endpoint.is_some_and(|p| (p[0] - a[0]).hypot(p[1] - a[1]) > 1e-7) {
                            return Err(input("Retained cleanup requires connected curves"));
                        }
                        ring.push(a);
                        endpoint = Some(b);
                        total += 1;
                        if total > 20000 {
                            return Err(input("Retained cleanup exceeds 20000 samples"));
                        }
                    }
                }
                if ring.len() < 3
                    || endpoint.is_none_or(|p| (p[0] - ring[0][0]).hypot(p[1] - ring[0][1]) > 1e-7)
                {
                    return Err(input("Retained cleanup requires closed boundaries"));
                }
                rings.push(
                    planar_geometry::path::BezierPath::from_polygon(&ring, true)?
                        .simplify(deviation * 0.5)?
                        .to_ring(deviation * 0.5)?,
                );
            }
            let rings = planar_geometry::rings::normalize(
                &rings,
                planar_geometry::tessellation::FillRule::EvenOdd,
            )?;
            let loops = rings
                .into_iter()
                .map(brep_core::sketch::polygon_wire)
                .collect::<Result<Vec<_>>>()?;
            let loops = planar_trim::orient_even_odd(&loops, 1e-7)?;
            profile(loops, 1e-7)
        }
        Some("brep_profile_offset") => profile(
            planar_trim::offset(
                &field::<Vec<Vec<Curve>>>(&v, "loops")?,
                field(&v, "distance")?,
                tolerance,
            )?,
            tolerance,
        ),
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
                "bezier" => {
                    let path = crate::path2d::authored_path(&v["path"])?;
                    let wire = brep_core::sketch::bezier_wire(path)?;
                    brep_core::bezier_profile::orient_even_odd(&[wire.clone()], tolerance).or_else(|_|brep_core::bezier_profile::normalize(&[wire],tolerance))?
                }
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
                "even-odd" => profile(
                    if loops.iter().flatten().any(|c| c.degree == 3) {
                        brep_core::bezier_profile::orient_even_odd(&loops, tolerance)?
                    } else {
                        planar_trim::orient_even_odd(&loops, tolerance)?
                    },
                    tolerance,
                ),
                _ => Err(input("Profile fillRule must be material-left or even-odd")),
            }
        }
        Some("brep_profile_boolean") => {
            let a: Vec<Vec<Curve>> = field(&v, "a")?;
            let b: Vec<Vec<Curve>> = field(&v, "b")?;
            let operation: String = field(&v, "operation")?;
            let result = if a.iter().chain(&b).flatten().any(|c| c.degree == 3) {
                brep_core::bezier_profile::boolean(&a, &b, &operation, tolerance)?
            } else {
                planar_trim::boolean(&a, &b, &operation, tolerance)?
            };
            profile(result, tolerance)
        }
        Some("brep_profile_signed_area") => {
            let wire: Vec<Curve> = field(&v, "loop")?;
            encode(if wire.iter().any(|c| c.degree == 3) {
                brep_core::bezier_profile::signed_area(&wire, tolerance)?
            } else {
                planar_trim::signed_area(&wire, tolerance)?
            })
        }
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
    fn cleanup_retained_circle_keeps_holes_and_refuses_budget_or_invalid_deviation() {
        let outer = brep_core::sketch::circle_wire(5.).unwrap();
        let inner = brep_core::sketch::circle_wire(2.).unwrap();
        let loops = planar_trim::orient_even_odd(&[outer, inner], 1e-7).unwrap();
        let result =
            dispatch(json!({"op":"brep_profile_cleanup","loops":loops,"deviation":0.05})).unwrap();
        assert_eq!(result["loops"].as_array().unwrap().len(), 2);
        assert!((result["areaMm2"].as_f64().unwrap() - 21. * std::f64::consts::PI).abs() < 2.);
        assert!(
            dispatch(json!({"op":"brep_profile_cleanup","loops":loops,"deviation":0.})).is_err()
        );
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
    #[test]
    fn offsets_retained_regions_and_reports_empty_material() {
        let loops = vec![rectangle([0., 0.], [6., 4.])];
        let expanded =
            dispatch(json!({"op":"brep_profile_offset","loops":loops,"distance":1.})).unwrap();
        assert!(
            (expanded["areaMm2"].as_f64().unwrap() - (44. + std::f64::consts::PI)).abs() < 1e-7
        );
        let empty =
            dispatch(json!({"op":"brep_profile_offset","loops":loops,"distance":-5.})).unwrap();
        assert_eq!(empty["loops"], json!([]));
        assert_eq!(empty["geometryStatus"], json!("numerical_uncertified"));
    }
}
