//! Retained 2D rational profiles. No temporary solids or mesh construction.
use super::{Result, Value, encode, field, input};
use brep_core::planar_trim;
use nurbs_core::curve::Curve;
use value_codec::json;

pub(crate) fn profile(loops: Vec<Vec<Curve>>, tolerance: f64) -> Result<Value> {
    if planar_trim::validate(&loops, tolerance).is_ok() {
        let area = loops.iter().try_fold(0., |sum, wire| {
            Ok::<_, super::Error>(sum + planar_trim::signed_area(wire, tolerance)?)
        })?;
        return Ok(json!({"kind":"brep-profile", "loops":loops, "areaMm2":area,
            "toleranceMm":tolerance, "geometryStatus":"numerical_uncertified"}));
    }
    let report = brep_core::profile_region::inspect(&loops, tolerance, true)?;
    let mut interval = [0_f64, 0_f64];
    for area in report.areas_mm2 {
        interval[0] = (interval[0] + area[0]).next_down();
        interval[1] = (interval[1] + area[1]).next_up();
    }
    if interval[0] <= 0. {
        return Err(input("Profile material area is unproven"));
    }
    Ok(json!({"kind":"brep-profile", "loops":loops,
        "areaMm2":interval[0]*0.5+interval[1]*0.5,"areaIntervalMm2":interval,
        "toleranceMm":tolerance, "geometryStatus":"numerical_uncertified"}))
}

pub fn dispatch(v: Value) -> Result<Value> {
    let tolerance = match v.get("toleranceMm") {
        Some(_) => field::<f64>(&v, "toleranceMm")?,
        None => 1e-7,
    };
    match v["op"].as_str() {
        Some("brep_profile_intersections") => brep_core::profile_intersections::inspect(
            &field::<Vec<Vec<Curve>>>(&v, "loops")?,
            tolerance,
            if v.get("maxPairs").is_some() {
                field(&v, "maxPairs")?
            } else {
                128
            },
            if v.get("maxBoxes").is_some() {
                field(&v, "maxBoxes")?
            } else {
                32768
            },
        ),
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
                    brep_core::profile_region::orient_even_odd(&loops, tolerance)?,
                    tolerance,
                ),
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
    fn retained_intersection_query_reports_original_segments_and_work_bounds() {
        let loops = vec![
            vec![Curve::from_polyline(vec![vec![0., 0.], vec![2., 2.]]).unwrap()],
            vec![Curve::from_polyline(vec![vec![0., 2.], vec![2., 0.]]).unwrap()],
        ];
        let r = dispatch(
            json!({"op":"brep_profile_intersections","loops":loops,"maxPairs":1,"maxBoxes":8192}),
        )
        .unwrap();
        assert_eq!(r["scope"].as_str(), Some("distinct-profile-segment-pairs"));
        assert_eq!(r["complete"].as_bool(), Some(true), "{r:?}");
        assert!(r["boxesVisited"].as_u64().unwrap() <= 8192);
        assert_eq!(
            r["pairs"][0]["report"]["components"][0]["kind"].as_str(),
            Some("point")
        );
    }
    #[test]
    fn general_profile_validation_roundtrip_transform_and_area_enclosure() {
        let mut wire = rectangle([0., 0.], [2., 2.]);
        wire[0] = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., -1.], vec![2., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let loops = vec![wire];
        let original = value_codec::to_string(&loops).unwrap();
        let profile = profile(loops.clone(), 1e-7).unwrap();
        let bounds: [f64; 2] = field(&profile, "areaIntervalMm2").unwrap();
        assert!(bounds[0] <= 14. / 3. && bounds[1] >= 14. / 3.);
        assert_eq!(
            field::<Vec<Vec<Curve>>>(&profile, "loops").unwrap()[0][0].control_points,
            loops[0][0].control_points
        );
        let reloaded: Value =
            value_codec::from_str(&value_codec::to_string(&profile).unwrap()).unwrap();
        let checked =
            dispatch(json!({"op":"brep_profile_validate","loops":reloaded["loops"]})).unwrap();
        assert_eq!(profile, checked);
        let matrix = [
            -2., 0., 0., 0., 0., 3., 0., 0., 0., 0., 1., 0., 10., 20., 0., 1.,
        ];
        let moved =
            dispatch(json!({"op":"brep_profile_transform","loops":loops,"matrix":matrix})).unwrap();
        let area: [f64; 2] = field(&moved, "areaIntervalMm2").unwrap();
        assert!(area[0] <= 28. && area[1] >= 28.);
        assert_eq!(value_codec::to_string(&loops).unwrap(), original);
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
