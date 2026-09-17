//! Read-only native intersection queries; reports never authorize topology edits.
use super::{Result, Value, encode, field, input};
use brep_core::intersections::{self, Options, SurfaceTrace};

pub fn dispatch(v: Value) -> Result<Value> {
    if v["op"].as_str() == Some("brep_intersection_trace_curve_segments") {
        let trace: SurfaceTrace = field(&v, "trace")?;
        return encode(trace.to_curve_segments()?);
    }
    if v["op"].as_str() == Some("brep_intersection_trace_curve") {
        let trace: SurfaceTrace = field(&v, "trace")?;
        return encode(trace.to_curve()?);
    }
    if v["op"].as_str() == Some("brep_intersection_trace_evaluate") {
        let trace: SurfaceTrace = field(&v, "trace")?;
        return encode(trace.evaluate(field(&v, "fraction")?)?);
    }
    let options: Options = match v.get("options") {
        Some(value) => value_codec::from_value(value.clone())
            .map_err(|e| input(format!("Invalid intersection options: {e}")))?,
        None => Options::default(),
    };
    match v["op"].as_str() {
        Some("brep_intersect_surface_surface") => encode(intersections::surface_surface(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_curve_segment") => encode(intersections::curve_segment(
            &field(&v, "curve")?,
            field(&v, "start")?,
            field(&v, "end")?,
            options,
        )?),
        Some("brep_intersect_curve_plane") => encode(intersections::curve_plane(
            &field(&v, "curve")?,
            field(&v, "plane")?,
            options,
        )?),
        Some("brep_intersect_surface_plane") => encode(intersections::surface_plane(
            &field(&v, "surface")?,
            field(&v, "plane")?,
            options,
        )?),
        Some("brep_intersect_curve_curve") => encode(intersections::curve_curve(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_curve_surface") => encode(intersections::curve_surface(
            &field(&v, "curve")?,
            &field(&v, "surface")?,
            options,
        )?),
        Some("brep_intersect_curve_ruled_surface") => encode(intersections::curve_ruled_surface(
            &field(&v, "curve")?,
            &field(&v, "surface")?,
            options,
        )?),
        Some("brep_intersect_sphere_sphere") => encode(intersections::intersect_sphere_sphere(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_sphere_cylinder") => encode(intersections::intersect_sphere_cylinder(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_sphere_cone") => encode(intersections::intersect_sphere_cone(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_cone_cone") => encode(intersections::intersect_cone_cone(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_cylinder_cylinder") => {
            encode(intersections::intersect_cylinder_cylinder(
                &field(&v, "first")?,
                &field(&v, "second")?,
                options,
            )?)
        }
        Some("brep_intersect_plane_sphere") => encode(intersections::intersect_plane_sphere(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_plane_cylinder") => encode(intersections::intersect_plane_cylinder(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_plane_cone") => encode(intersections::intersect_plane_cone(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_plane_torus") => encode(intersections::intersect_plane_torus(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_sphere_torus") => encode(intersections::intersect_sphere_torus(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_cylinder_torus") => encode(intersections::intersect_cylinder_torus(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_cone_torus") => encode(intersections::intersect_cone_torus(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_torus_torus") => encode(intersections::intersect_torus_torus(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        _ => Err(input("Unknown intersection query")),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn serialized_intersection_report_is_explicitly_uncertified() {
        let curve =
            nurbs_core::curve::Curve::from_polyline(vec![vec![0., 0., -1.], vec![0., 0., 1.]])
                .unwrap();
        let result = super::dispatch(value_codec::json!({"op":"brep_intersect_curve_plane","curve":curve,"plane":{"normal":[0.,0.,1.],"offset":0.}})).unwrap();
        assert_eq!(result["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(result["permitsTopologyChange"].as_bool(), Some(false));
        assert_eq!(
            result["components"][0]["curve"]["parameter"].as_f64(),
            Some(0.5)
        );
    }
    #[test]
    fn serialized_curve_curve_report_keeps_paired_parameters_and_uncertified() {
        let first =
            nurbs_core::curve::Curve::from_polyline(vec![vec![0., 0., 0.], vec![2., 0., 0.]])
                .unwrap();
        let second =
            nurbs_core::curve::Curve::from_polyline(vec![vec![1., -1., 0.], vec![1., 1., 0.]])
                .unwrap();
        let result = super::dispatch(
            value_codec::json!({"op":"brep_intersect_curve_curve","first":first,"second":second}),
        )
        .unwrap();
        assert_eq!(result["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(result["permitsTopologyChange"].as_bool(), Some(false));
        assert_eq!(result["evidence"].as_str(), Some("numerical_uncertified"));
        let hit = &result["components"][0];
        assert_eq!(hit["kind"].as_str(), Some("point"));
        assert!((hit["first"].as_f64().unwrap() - 0.5).abs() <= 1e-10);
        assert!((hit["second"].as_f64().unwrap() - 0.5).abs() <= 1e-10);
        assert_eq!(hit["contact"].as_str(), Some("transverse"));
        let overlap = super::dispatch(value_codec::json!({"op":"brep_intersect_curve_curve",
            "first":nurbs_core::curve::Curve::from_polyline(vec![vec![0.,0.,0.],vec![1.,0.,0.]]).unwrap(),
            "second":nurbs_core::curve::Curve::from_polyline(vec![vec![0.5,0.,0.],vec![1.5,0.,0.]]).unwrap()}))
        .unwrap();
        let component = &overlap["components"][0];
        assert_eq!(component["kind"].as_str(), Some("overlap"));
        assert_eq!(component["firstInterval"][0].as_f64(), Some(0.5));
        assert_eq!(component["secondInterval"][1].as_f64(), Some(0.5));
        assert_eq!(component["reversed"].as_bool(), Some(false));
    }
    #[test]
    fn serialized_curve_ruled_surface_report_keeps_uv_and_stays_uncertified() {
        use nurbs_core::surface::Surface;
        let surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1., 1.]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let curve = nurbs_core::curve::Curve::from_polyline(vec![
            vec![0.25, 0.25, -1.],
            vec![0.25, 0.25, 1.],
        ])
        .unwrap();
        let result = super::dispatch(value_codec::json!({"op":"brep_intersect_curve_ruled_surface","curve":curve,"surface":surface})).unwrap();
        assert_eq!(result["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(result["permitsTopologyChange"].as_bool(), Some(false));
        assert_eq!(result["evidence"].as_str(), Some("numerical_uncertified"));
        let hit = &result["components"][0];
        assert_eq!(hit["kind"].as_str(), Some("point"));
        assert!((hit["t"].as_f64().unwrap() - 0.5).abs() <= 1e-10);
        assert!((hit["uv"][0].as_f64().unwrap() - 0.25).abs() <= 1e-10);
        assert!((hit["uv"][1].as_f64().unwrap() - 0.25).abs() <= 1e-10);
        assert_eq!(hit["contact"].as_str(), Some("transverse"));
        assert_eq!(hit["uvBox"].as_array().map(Vec::len), Some(4));
        // Ruling coincidence serializes the lifted UV path endpoints.
        let ruling =
            nurbs_core::curve::Curve::from_polyline(vec![vec![0.5, 0.25, 0.], vec![0.5, 0.75, 0.]])
                .unwrap();
        let overlap = super::dispatch(value_codec::json!({"op":"brep_intersect_curve_ruled_surface","curve":ruling,"surface":surface})).unwrap();
        let component = &overlap["components"][0];
        assert_eq!(component["kind"].as_str(), Some("overlap"));
        assert_eq!(component["curveInterval"][0].as_f64(), Some(0.));
        assert!((component["uvStart"][0].as_f64().unwrap() - 0.5).abs() <= 1e-12);
        assert!((component["uvEnd"][1].as_f64().unwrap() - 0.75).abs() <= 1e-12);
        // A non-ruled tensor patch is an explicit unsupported region.
        let curved = Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| vec![i as f64, j as f64, (i * j) as f64])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let curve =
            nurbs_core::curve::Curve::from_polyline(vec![vec![1., 1., -1.], vec![1., 1., 2.]])
                .unwrap();
        let refused = super::dispatch(value_codec::json!({"op":"brep_intersect_curve_ruled_surface","curve":curve,"surface":curved})).unwrap();
        assert_eq!(refused["coverage"].as_str(), Some("incomplete"));
        assert_eq!(
            refused["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
    }
    #[test]
    fn serialized_sphere_sphere_report_carries_exact_circle_and_stays_uncertified() {
        let first = brep_core::analytic::sphere(2.).unwrap();
        let second = brep_core::transform::affine(
            &first,
            [
                [1., 0., 0., 2.],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let result = super::dispatch(
            value_codec::json!({"op":"brep_intersect_sphere_sphere","first":first,"second":second}),
        )
        .unwrap();
        assert_eq!(result["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(result["permitsTopologyChange"].as_bool(), Some(false));
        assert_eq!(result["evidence"].as_str(), Some("numerical_uncertified"));
        let circle = &result["components"][0];
        assert_eq!(circle["kind"].as_str(), Some("circle"));
        assert!((circle["radius"].as_f64().unwrap() - 3_f64.sqrt()).abs() <= 1e-12);
        assert_eq!(circle["center"].as_array().map(Vec::len), Some(3));
        assert!((circle["center"][0].as_f64().unwrap() - 1.).abs() <= 1e-12);
        assert_eq!(circle["curve"]["weights"].as_array().map(Vec::len), Some(9));
        assert_eq!(
            circle["curve"]["weights"][1].as_f64(),
            Some(std::f64::consts::FRAC_1_SQRT_2)
        );
        assert_eq!(circle["firstUv"].as_array().map(Vec::len), Some(4));
        assert_eq!(circle["secondUv"].as_array().map(Vec::len), Some(4));
        assert_eq!(circle["firstUv"][0]["patch"].as_f64(), Some(0.));
        assert!(circle["maxSampleResidual"].as_f64().unwrap() <= 1e-12);
    }
    #[test]
    fn serialized_sphere_sphere_classification_regions_are_explicit() {
        let sphere = brep_core::analytic::sphere(2.).unwrap();
        let place = |x: f64| {
            brep_core::transform::affine(
                &sphere,
                [
                    [1., 0., 0., x],
                    [0., 1., 0., 0.],
                    [0., 0., 1., 0.],
                    [0., 0., 0., 1.],
                ],
            )
            .unwrap()
        };
        // Coincident spheres: explicit coincident_trim region, no curve.
        let coincident = super::dispatch(
            value_codec::json!({"op":"brep_intersect_sphere_sphere","first":sphere,"second":sphere}),
        )
        .unwrap();
        assert_eq!(coincident["coverage"].as_str(), Some("incomplete"));
        assert_eq!(coincident["components"].as_array().map(Vec::len), Some(0));
        assert_eq!(
            coincident["unresolved"][0]["reason"].as_str(),
            Some("coincident_trim")
        );
        // Exact external tangency: tangency_or_multiple_root, never a point.
        let tangent = super::dispatch(
            value_codec::json!({"op":"brep_intersect_sphere_sphere","first":sphere,"second":place(4.)}),
        )
        .unwrap();
        assert_eq!(tangent["components"].as_array().map(Vec::len), Some(0));
        assert_eq!(
            tangent["unresolved"][0]["reason"].as_str(),
            Some("tangency_or_multiple_root")
        );
        // Separate pair: empty and numerically resolved.
        let separate = super::dispatch(
            value_codec::json!({"op":"brep_intersect_sphere_sphere","first":sphere,"second":place(5.)}),
        )
        .unwrap();
        assert_eq!(separate["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(separate["components"].as_array().map(Vec::len), Some(0));
        // Non-canonical solid: explicit unsupported region, no fallback.
        let cylinder = brep_core::analytic::cylinder(1., 2.).unwrap();
        let refused = super::dispatch(
            value_codec::json!({"op":"brep_intersect_sphere_sphere","first":sphere,"second":cylinder}),
        )
        .unwrap();
        assert_eq!(refused["coverage"].as_str(), Some("incomplete"));
        assert_eq!(
            refused["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
    }
    #[test]
    fn serialized_sphere_cylinder_report_carries_exact_circles_and_stays_uncertified() {
        let sphere = brep_core::transform::affine(
            &brep_core::analytic::sphere(3.).unwrap(),
            [
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 1., 4.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
        let result = super::dispatch(
            value_codec::json!({"op":"brep_intersect_sphere_cylinder","first":sphere,"second":cylinder}),
        )
        .unwrap();
        assert_eq!(result["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(result["permitsTopologyChange"].as_bool(), Some(false));
        assert_eq!(result["evidence"].as_str(), Some("numerical_uncertified"));
        assert_eq!(result["components"].as_array().map(Vec::len), Some(2));
        let circle = &result["components"][0];
        assert_eq!(circle["kind"].as_str(), Some("circle"));
        assert!((circle["radius"].as_f64().unwrap() - 2.).abs() <= 1e-12);
        assert!((circle["center"][2].as_f64().unwrap() - (4. - 5_f64.sqrt())).abs() <= 1e-12);
        assert_eq!(circle["curve"]["weights"].as_array().map(Vec::len), Some(9));
        assert_eq!(
            circle["curve"]["weights"][1].as_f64(),
            Some(std::f64::consts::FRAC_1_SQRT_2)
        );
        // Side circles lift to iso-v lines on all four side patches.
        assert_eq!(circle["cylinderUv"].as_array().map(Vec::len), Some(4));
        assert!(!circle["sphereUv"].as_array().unwrap().is_empty());
        assert!(circle["maxSampleResidual"].as_f64().unwrap() <= 1e-12);
    }
    #[test]
    fn serialized_sphere_cylinder_classification_regions_are_explicit() {
        let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
        let place = |radius: f64, x: f64, z: f64| {
            brep_core::transform::affine(
                &brep_core::analytic::sphere(radius).unwrap(),
                [
                    [1., 0., 0., x],
                    [0., 1., 0., 0.],
                    [0., 0., 1., z],
                    [0., 0., 0., 1.],
                ],
            )
            .unwrap()
        };
        let run = |first: &brep_core::Model, second: &brep_core::Model| {
            super::dispatch(
                value_codec::json!({"op":"brep_intersect_sphere_cylinder","first":first,"second":second}),
            )
            .unwrap()
        };
        // r == R: the coincident band, never a guessed circle.
        let coincident = run(&place(2., 0., 4.), &cylinder);
        assert_eq!(coincident["coverage"].as_str(), Some("incomplete"));
        assert_eq!(coincident["components"].as_array().map(Vec::len), Some(0));
        assert_eq!(
            coincident["unresolved"][0]["reason"].as_str(),
            Some("coincident_trim")
        );
        // Cap-plane tangency: tangency_or_multiple_root, never a point.
        let tangent = run(&place(1.5, 0., 6.5), &cylinder);
        assert_eq!(tangent["components"].as_array().map(Vec::len), Some(0));
        assert_eq!(
            tangent["unresolved"][0]["reason"].as_str(),
            Some("tangency_or_multiple_root")
        );
        // Small sphere strictly inside: empty and numerically resolved.
        let inside = run(&place(1., 0., 4.), &cylinder);
        assert_eq!(inside["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(inside["components"].as_array().map(Vec::len), Some(0));
        // Clearly off-axis: the general quartic is out of scope.
        let off_axis = run(&place(3., 0.5, 4.), &cylinder);
        assert_eq!(
            off_axis["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
        // A frustum is not the canonical cylinder: explicit refusal.
        let refused = run(
            &place(2., 0., 4.),
            &brep_core::analytic::frustum(1., 2., 3.).unwrap(),
        );
        assert_eq!(refused["coverage"].as_str(), Some("incomplete"));
        assert_eq!(
            refused["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
    }
    #[test]
    fn serialized_sphere_cone_report_carries_exact_circles_and_stays_uncertified() {
        let sphere = brep_core::transform::affine(
            &brep_core::analytic::sphere(2.).unwrap(),
            [
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 1., 3.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let cone = brep_core::analytic::frustum(1., 3., 6.).unwrap();
        let result = super::dispatch(
            value_codec::json!({"op":"brep_intersect_sphere_cone","first":sphere,"second":cone}),
        )
        .unwrap();
        assert_eq!(result["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(result["permitsTopologyChange"].as_bool(), Some(false));
        assert_eq!(result["evidence"].as_str(), Some("numerical_uncertified"));
        assert_eq!(result["components"].as_array().map(Vec::len), Some(2));
        let circle = &result["components"][0];
        assert_eq!(circle["kind"].as_str(), Some("circle"));
        // Oracle: roots q = (-2/3 +- 2/3) * 9/10 of the side quadratic; the
        // lower circle sits at z = 1.8 with the interpolated radius 1.6.
        assert!((circle["radius"].as_f64().unwrap() - 1.6).abs() <= 1e-12);
        assert!((circle["center"][2].as_f64().unwrap() - 1.8).abs() <= 1e-12);
        assert_eq!(circle["curve"]["weights"].as_array().map(Vec::len), Some(9));
        assert_eq!(
            circle["curve"]["weights"][1].as_f64(),
            Some(std::f64::consts::FRAC_1_SQRT_2)
        );
        // Side circles lift to iso-v lines on all four side patches.
        assert_eq!(circle["coneUv"].as_array().map(Vec::len), Some(4));
        assert!(!circle["sphereUv"].as_array().unwrap().is_empty());
        assert!(circle["maxSampleResidual"].as_f64().unwrap() <= 1e-12);
    }
    #[test]
    fn serialized_sphere_cone_classification_regions_are_explicit() {
        let cone = brep_core::analytic::frustum(2., 4., 6.).unwrap();
        let place = |radius: f64, x: f64, z: f64| {
            brep_core::transform::affine(
                &brep_core::analytic::sphere(radius).unwrap(),
                [
                    [1., 0., 0., x],
                    [0., 1., 0., 0.],
                    [0., 0., 1., z],
                    [0., 0., 0., 1.],
                ],
            )
            .unwrap()
        };
        let run = |first: &brep_core::Model, second: &brep_core::Model| {
            super::dispatch(
                value_codec::json!({"op":"brep_intersect_sphere_cone","first":first,"second":second}),
            )
            .unwrap()
        };
        // Side tangency (double root): r == rho_c / sqrt(1+m^2) = 9/sqrt(10)
        // for the sphere centered at z = 3 — never a guessed circle.
        let tangent = run(&place(9. / 10_f64.sqrt(), 0., 3.), &cone);
        assert_eq!(tangent["coverage"].as_str(), Some("incomplete"));
        assert_eq!(tangent["components"].as_array().map(Vec::len), Some(0));
        assert_eq!(
            tangent["unresolved"][0]["reason"].as_str(),
            Some("tangency_or_multiple_root")
        );
        // Rim contact: r == r_bottom on the bottom ring plane.
        let rim = run(&place(2., 0., 0.), &cone);
        assert_eq!(
            rim["unresolved"][0]["reason"].as_str(),
            Some("tangency_or_multiple_root")
        );
        // Small sphere strictly inside: empty and numerically resolved.
        let inside = run(&place(0.5, 0., 3.), &cone);
        assert_eq!(inside["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(inside["components"].as_array().map(Vec::len), Some(0));
        // Clearly off-axis: the general quartic is out of scope.
        let off_axis = run(&place(2.5, 0.5, 0.5), &cone);
        assert_eq!(
            off_axis["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
        // An equal-radius frustum is a cylinder: explicit refusal as the
        // cone operand, never a numerical fallback.
        let refused = run(
            &place(2., 0., 3.),
            &brep_core::analytic::cylinder(1., 3.).unwrap(),
        );
        assert_eq!(refused["coverage"].as_str(), Some("incomplete"));
        assert_eq!(
            refused["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
    }
    #[test]
    fn serialized_cone_cone_report_carries_exact_circles_and_stays_uncertified() {
        let first = brep_core::analytic::frustum(1., 3., 6.).unwrap();
        let second = brep_core::transform::affine(
            &brep_core::analytic::frustum(4., 2., 4.).unwrap(),
            [
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 1., 1.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let result = super::dispatch(
            value_codec::json!({"op":"brep_intersect_cone_cone","first":first,"second":second}),
        )
        .unwrap();
        assert_eq!(result["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(result["permitsTopologyChange"].as_bool(), Some(false));
        assert_eq!(result["evidence"].as_str(), Some("numerical_uncertified"));
        assert_eq!(result["components"].as_array().map(Vec::len), Some(1));
        let circle = &result["components"][0];
        assert_eq!(circle["kind"].as_str(), Some("circle"));
        // Oracle: rho_1 = 1 + s/3 meets rho_2 = 4.5 - s/2 at s* = 4.2 with
        // the radius 2.4, strictly inside both height ranges.
        assert!((circle["radius"].as_f64().unwrap() - 2.4).abs() <= 1e-12);
        assert!((circle["center"][2].as_f64().unwrap() - 4.2).abs() <= 1e-12);
        assert_eq!(circle["curve"]["weights"].as_array().map(Vec::len), Some(9));
        assert_eq!(
            circle["curve"]["weights"][1].as_f64(),
            Some(std::f64::consts::FRAC_1_SQRT_2)
        );
        // Side circles lift to iso-v lines on all four side patches of both
        // cones (v = 0.7 on the first, v = 0.8 on the second).
        assert_eq!(circle["firstUv"].as_array().map(Vec::len), Some(4));
        assert_eq!(circle["secondUv"].as_array().map(Vec::len), Some(4));
        assert!(
            (circle["firstUv"][0]["arcs"][0]["controlPoints"][0][1]
                .as_f64()
                .unwrap()
                - 0.7)
                .abs()
                <= 1e-12
        );
        assert!(
            (circle["secondUv"][0]["arcs"][0]["controlPoints"][0][1]
                .as_f64()
                .unwrap()
                - 0.8)
                .abs()
                <= 1e-12
        );
        assert!(circle["maxSampleResidual"].as_f64().unwrap() <= 1e-12);
    }
    #[test]
    fn serialized_cone_cone_classification_regions_are_explicit() {
        let first = brep_core::analytic::frustum(1., 3., 6.).unwrap();
        let place = |r_bottom: f64, r_top: f64, height: f64, z: f64| {
            brep_core::transform::affine(
                &brep_core::analytic::frustum(r_bottom, r_top, height).unwrap(),
                [
                    [1., 0., 0., 0.],
                    [0., 1., 0., 0.],
                    [0., 0., 1., z],
                    [0., 0., 0., 1.],
                ],
            )
            .unwrap()
        };
        let run = |a: &brep_core::Model, b: &brep_core::Model| {
            super::dispatch(
                value_codec::json!({"op":"brep_intersect_cone_cone","first":a,"second":b}),
            )
            .unwrap()
        };
        // Equal-taper coincident profiles: coincident_trim, never a surface.
        let coincident = run(&first, &place(5. / 3., 11. / 3., 6., 2.));
        assert_eq!(coincident["coverage"].as_str(), Some("incomplete"));
        assert_eq!(coincident["components"].as_array().map(Vec::len), Some(0));
        assert_eq!(
            coincident["unresolved"][0]["reason"].as_str(),
            Some("coincident_trim")
        );
        // Stacked rim/rim contact at the shared ring plane z = 6.
        let rim = run(&first, &place(3., 5., 6., 6.));
        assert_eq!(
            rim["unresolved"][0]["reason"].as_str(),
            Some("tangency_or_multiple_root")
        );
        // Equal-taper distinct profiles: empty and numerically resolved.
        let distinct = run(&first, &place(2., 4., 6., 2.));
        assert_eq!(distinct["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(distinct["components"].as_array().map(Vec::len), Some(0));
        assert_eq!(distinct["unresolved"].as_array().map(Vec::len), Some(0));
        // Clipped root and clear axial separation: empty and resolved.
        let clipped = run(
            &brep_core::analytic::frustum(3., 1., 6.).unwrap(),
            &place(4., 2., 2., 2.),
        );
        assert_eq!(clipped["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(clipped["components"].as_array().map(Vec::len), Some(0));
        let separated = run(&first, &place(1., 2., 3., 6.5));
        assert_eq!(separated["components"].as_array().map(Vec::len), Some(0));
        assert_eq!(separated["unresolved"].as_array().map(Vec::len), Some(0));
        // Clearly off-axis: the general quartic is out of scope.
        let off_axis = brep_core::transform::affine(
            &brep_core::analytic::frustum(4., 2., 4.).unwrap(),
            [
                [1., 0., 0., 0.5],
                [0., 1., 0., 0.],
                [0., 0., 1., 1.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let off_axis = run(&first, &off_axis);
        assert_eq!(
            off_axis["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
        // An equal-radius frustum is a cylinder: explicit refusal, never a
        // numerical fallback.
        let refused = run(&first, &brep_core::analytic::cylinder(1., 3.).unwrap());
        assert_eq!(refused["coverage"].as_str(), Some("incomplete"));
        assert_eq!(
            refused["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
    }
    #[test]
    fn serialized_cylinder_cylinder_report_carries_exact_lines_and_stays_uncertified() {
        let first = brep_core::analytic::cylinder(2., 8.).unwrap();
        let second = brep_core::transform::affine(
            &brep_core::analytic::cylinder(3., 8.).unwrap(),
            [
                [1., 0., 0., 3.5],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let result = super::dispatch(
            value_codec::json!({"op":"brep_intersect_cylinder_cylinder","first":first,"second":second}),
        )
        .unwrap();
        assert_eq!(result["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(result["permitsTopologyChange"].as_bool(), Some(false));
        assert_eq!(result["evidence"].as_str(), Some("numerical_uncertified"));
        assert_eq!(result["components"].as_array().map(Vec::len), Some(2));
        // Oracle: x = (d^2 + r1^2 - r2^2)/(2d), y = sqrt(r1^2 - x^2).
        let x: f64 = (3.5 * 3.5 + 4. - 9.) / 7.;
        let y = (4. - x * x).sqrt();
        let line = &result["components"][0];
        assert_eq!(line["kind"].as_str(), Some("line"));
        assert_eq!(line["contact"].as_str(), Some("boundary"));
        assert_eq!(line["curve"]["degree"].as_f64(), Some(1.));
        assert!((line["start"][0].as_f64().unwrap() - x).abs() <= 1e-12);
        assert!((line["start"][1].as_f64().unwrap() + y).abs() <= 1e-12);
        assert!((line["start"][2].as_f64().unwrap()).abs() <= 1e-12);
        assert!((line["end"][2].as_f64().unwrap() - 8.).abs() <= 1e-12);
        assert!((line["direction"][2].as_f64().unwrap() - 1.).abs() <= 1e-12);
        // Iso-u lifts: one side patch per cylinder, v sweeping the height.
        assert_eq!(line["firstUv"].as_array().map(Vec::len), Some(1));
        assert_eq!(line["secondUv"].as_array().map(Vec::len), Some(1));
        let lift = &line["firstUv"][0]["arcs"][0];
        assert_eq!(lift["degree"].as_f64(), Some(1.));
        assert_eq!(lift["controlPoints"][0][1].as_f64(), Some(0.));
        assert_eq!(lift["controlPoints"][1][1].as_f64(), Some(1.));
        assert_eq!(
            lift["controlPoints"][0][0].as_f64(),
            lift["controlPoints"][1][0].as_f64()
        );
        assert!(line["maxSampleResidual"].as_f64().unwrap() <= 1e-12);
    }
    #[test]
    fn serialized_cylinder_cylinder_classification_regions_are_explicit() {
        let first = brep_core::analytic::cylinder(2., 8.).unwrap();
        let place = |radius: f64, height: f64, x: f64, z: f64| {
            brep_core::transform::affine(
                &brep_core::analytic::cylinder(radius, height).unwrap(),
                [
                    [1., 0., 0., x],
                    [0., 1., 0., 0.],
                    [0., 0., 1., z],
                    [0., 0., 0., 1.],
                ],
            )
            .unwrap()
        };
        let run = |a: &brep_core::Model, b: &brep_core::Model| {
            super::dispatch(
                value_codec::json!({"op":"brep_intersect_cylinder_cylinder","first":a,"second":b}),
            )
            .unwrap()
        };
        // Coaxial equal radii: the coincident side band, never geometry.
        let coincident = run(&first, &brep_core::analytic::cylinder(2., 8.).unwrap());
        assert_eq!(coincident["coverage"].as_str(), Some("incomplete"));
        assert_eq!(coincident["components"].as_array().map(Vec::len), Some(0));
        assert_eq!(
            coincident["unresolved"][0]["reason"].as_str(),
            Some("coincident_trim")
        );
        // Stacked equal radii sharing a cap plane: rim tangency, unresolved.
        let stacked = run(&first, &place(2., 4., 0., 8.));
        assert_eq!(stacked["components"].as_array().map(Vec::len), Some(0));
        assert_eq!(
            stacked["unresolved"][0]["reason"].as_str(),
            Some("tangency_or_multiple_root")
        );
        // Coaxial unequal radii, no coincident cap plane: empty, resolved.
        let nested = run(&first, &place(1., 4., 0., 2.));
        assert_eq!(nested["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(nested["components"].as_array().map(Vec::len), Some(0));
        // External tangency d == r1 + r2: tangency region, never a line.
        let tangent = run(&first, &place(3., 8., 5., 0.));
        assert_eq!(tangent["components"].as_array().map(Vec::len), Some(0));
        assert_eq!(
            tangent["unresolved"][0]["reason"].as_str(),
            Some("tangency_or_multiple_root")
        );
        // Clearly non-parallel: the general quartic is out of scope.
        let (sin, cos) = 0.3_f64.sin_cos();
        let skew = brep_core::transform::affine(
            &brep_core::analytic::cylinder(3., 8.).unwrap(),
            [
                [1., 0., 0., 3.5],
                [0., cos, -sin, 0.],
                [0., sin, cos, 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let refused = run(&first, &skew);
        assert_eq!(refused["coverage"].as_str(), Some("incomplete"));
        assert_eq!(
            refused["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
        // A frustum is not the canonical cylinder: explicit refusal.
        let refused = run(&first, &brep_core::analytic::frustum(1., 2., 3.).unwrap());
        assert_eq!(
            refused["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
    }
    /// The canonical planar patch operand: one-face open model, exact
    /// bilinear affine surface over the unit square, boundary trims, four
    /// corner vertices (mirrors brep-core's plane_patch fixture).
    fn plane_patch(origin: [f64; 3], u: [f64; 3], v: [f64; 3]) -> brep_core::Model {
        use brep_topology::{Coedge, Edge, Face, FaceUse, Loop, Shell, Vertex};
        let line = |a: Vec<f64>, b: Vec<f64>| nurbs_core::curve::Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![a, b],
            weights: vec![1., 1.],
            periodic: false,
        };
        let add = |a: [f64; 3], b: [f64; 3]| [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
        let corners = [
            origin,
            add(origin, u),
            add(add(origin, u), v),
            add(origin, v),
        ];
        let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        let mut edges = Vec::new();
        let mut coedges = Vec::new();
        for i in 0..4 {
            let j = (i + 1) % 4;
            edges.push(Edge {
                degenerate: false,
                vertices: [i, j],
                curve: line(corners[i].to_vec(), corners[j].to_vec()),
            });
            coedges.push(Coedge {
                edge: i,
                reversed: false,
                pcurve: line(uv[i].to_vec(), uv[j].to_vec()),
            });
        }
        let mut model = brep_core::Model(
            brep_topology::Model {
                vertices: corners.map(|point| Vertex { point }).to_vec(),
                edges,
                loops: vec![Loop { coedges }],
                faces: vec![Face {
                    surface: nurbs_core::surface::Surface {
                        degree_u: 1,
                        degree_v: 1,
                        knots_u: vec![0., 0., 1., 1.],
                        knots_v: vec![0., 0., 1., 1.],
                        control_points: vec![
                            vec![corners[0].to_vec(), corners[3].to_vec()],
                            vec![corners[1].to_vec(), corners[2].to_vec()],
                        ],
                        weights: vec![vec![1., 1.], vec![1., 1.]],
                        periodic_u: false,
                        periodic_v: false,
                    },
                    outer: 0,
                    holes: vec![],
                }],
                shells: vec![Shell {
                    faces: vec![FaceUse {
                        face: 0,
                        reversed: false,
                    }],
                    closed: false,
                }],
                bodies: vec![],
                tolerance_mm: 1e-7,
            },
            brep_core::TopologyIds::default(),
        );
        model.rebuild_topology_ids();
        model.validate().unwrap();
        model
    }
    #[test]
    fn serialized_plane_sphere_report_carries_exact_circle_and_stays_uncertified() {
        let plane = plane_patch([-3., -3., 1.], [6., 0., 0.], [0., 6., 0.]);
        let sphere = brep_core::analytic::sphere(2.).unwrap();
        let result = super::dispatch(
            value_codec::json!({"op":"brep_intersect_plane_sphere","first":plane,"second":sphere}),
        )
        .unwrap();
        assert_eq!(result["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(result["permitsTopologyChange"].as_bool(), Some(false));
        assert_eq!(result["evidence"].as_str(), Some("numerical_uncertified"));
        assert_eq!(result["components"].as_array().map(Vec::len), Some(1));
        let circle = &result["components"][0];
        assert_eq!(circle["kind"].as_str(), Some("circle"));
        assert!((circle["radius"].as_f64().unwrap() - 3_f64.sqrt()).abs() <= 1e-12);
        assert!((circle["center"][2].as_f64().unwrap() - 1.).abs() <= 1e-12);
        assert_eq!(circle["full"].as_bool(), Some(true));
        assert_eq!(circle["curve"]["weights"].as_array().map(Vec::len), Some(9));
        assert_eq!(
            circle["curve"]["weights"][1].as_f64(),
            Some(std::f64::consts::FRAC_1_SQRT_2)
        );
        assert_eq!(
            circle["planeUv"][0]["arcs"].as_array().map(Vec::len),
            Some(4)
        );
        assert!(!circle["sphereUv"].as_array().unwrap().is_empty());
        assert!(circle["maxSampleResidual"].as_f64().unwrap() <= 1e-12);
    }
    #[test]
    fn serialized_plane_sphere_classification_regions_are_explicit() {
        let sphere = brep_core::analytic::sphere(2.).unwrap();
        let run = |plane: &brep_core::Model, second: &brep_core::Model| {
            super::dispatch(
                value_codec::json!({"op":"brep_intersect_plane_sphere","first":plane,"second":second}),
            )
            .unwrap()
        };
        let at = |z: f64| plane_patch([-1., -1., z], [2., 0., 0.], [0., 2., 0.]);
        // Tangency: never a guessed point.
        let tangent = run(&at(2.), &sphere);
        assert_eq!(tangent["coverage"].as_str(), Some("incomplete"));
        assert_eq!(tangent["components"].as_array().map(Vec::len), Some(0));
        assert_eq!(
            tangent["unresolved"][0]["reason"].as_str(),
            Some("tangency_or_multiple_root")
        );
        // Clear miss: empty and resolved.
        let miss = run(&at(5.), &sphere);
        assert_eq!(miss["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(miss["components"].as_array().map(Vec::len), Some(0));
        // A solid operand is not the planar patch: explicit refusal.
        let refused = run(
            &brep_core::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap(),
            &sphere,
        );
        assert_eq!(
            refused["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
    }
    #[test]
    fn serialized_plane_cylinder_report_carries_exact_rulings_and_stays_uncertified() {
        // Plane x = 1 parallel to the axis: the two exact rulings at
        // (1, +-sqrt(3), z), z in [0, 8].
        let plane = plane_patch([1., -3., 0.], [0., 6., 0.], [0., 0., 10.]);
        let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
        let result = super::dispatch(
            value_codec::json!({"op":"brep_intersect_plane_cylinder","first":plane,"second":cylinder}),
        )
        .unwrap();
        assert_eq!(result["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(result["permitsTopologyChange"].as_bool(), Some(false));
        assert_eq!(result["evidence"].as_str(), Some("numerical_uncertified"));
        assert_eq!(result["components"].as_array().map(Vec::len), Some(2));
        let h = 3_f64.sqrt();
        for line in result["components"].as_array().unwrap() {
            assert_eq!(line["kind"].as_str(), Some("line"));
            assert_eq!(line["contact"].as_str(), Some("boundary"));
            assert_eq!(line["curve"]["degree"].as_f64(), Some(1.));
            assert!((line["start"][0].as_f64().unwrap() - 1.).abs() <= 1e-12);
            assert!((line["start"][1].as_f64().unwrap().abs() - h).abs() <= 1e-12);
            assert!(line["start"][2].as_f64().unwrap().abs() <= 1e-12);
            assert!((line["end"][2].as_f64().unwrap() - 8.).abs() <= 1e-12);
            assert!((line["direction"][2].as_f64().unwrap() - 1.).abs() <= 1e-12);
            assert!(!line["planeUv"][0]["arcs"].as_array().unwrap().is_empty());
            assert!(!line["cylinderUv"].as_array().unwrap().is_empty());
            assert!(line["maxSampleResidual"].as_f64().unwrap() <= 1e-12);
        }
    }
    #[test]
    fn serialized_plane_cylinder_classification_regions_are_explicit() {
        let cylinder = brep_core::analytic::cylinder(2., 8.).unwrap();
        let run = |plane: &brep_core::Model, second: &brep_core::Model| {
            super::dispatch(
                value_codec::json!({"op":"brep_intersect_plane_cylinder","first":plane,"second":second}),
            )
            .unwrap()
        };
        // Cap-plane coincidence: the coincident cap/rim region, never a
        // guessed circle.
        let coincident = run(
            &plane_patch([-3., -3., 8.], [6., 0., 0.], [0., 6., 0.]),
            &cylinder,
        );
        assert_eq!(coincident["coverage"].as_str(), Some("incomplete"));
        assert_eq!(coincident["components"].as_array().map(Vec::len), Some(0));
        assert_eq!(
            coincident["unresolved"][0]["reason"].as_str(),
            Some("coincident_trim")
        );
        // Tangent parallel plane: tangency region, never a guessed line.
        let tangent = run(
            &plane_patch([2., -3., 0.], [0., 6., 0.], [0., 0., 10.]),
            &cylinder,
        );
        assert_eq!(tangent["components"].as_array().map(Vec::len), Some(0));
        assert_eq!(
            tangent["unresolved"][0]["reason"].as_str(),
            Some("tangency_or_multiple_root")
        );
        // Clear miss: empty and resolved.
        let miss = run(
            &plane_patch([3., -3., 0.], [0., 6., 0.], [0., 0., 10.]),
            &cylinder,
        );
        assert_eq!(miss["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(miss["components"].as_array().map(Vec::len), Some(0));
        // A skewed patch is not canonical; a frustum is not the cylinder.
        let skewed = plane_patch([-3., -3., 4.], [6., 0., 0.], [3., 6., 0.]);
        let refused = run(&skewed, &cylinder);
        assert_eq!(
            refused["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
        let refused = run(
            &plane_patch([-3., -3., 4.], [6., 0., 0.], [0., 6., 0.]),
            &brep_core::analytic::frustum(1., 2., 3.).unwrap(),
        );
        assert_eq!(
            refused["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
    }
    #[test]
    fn serialized_plane_cone_report_carries_exact_conics_and_stays_uncertified() {
        // Frustum r 3 -> 1 over z 0..5, plane z = 2.5: the exact circle of
        // the linearly interpolated radius 2; plane y = 0 through the axis:
        // the two exact rulings (+-3,0,0) -> (+-1,0,5).
        let cone = brep_core::analytic::frustum(3., 1., 5.).unwrap();
        let run = |plane: &brep_core::Model| {
            super::dispatch(
                value_codec::json!({"op":"brep_intersect_plane_cone","first":plane,"second":cone}),
            )
            .unwrap()
        };
        let result = run(&plane_patch([-4., -4., 2.5], [8., 0., 0.], [0., 8., 0.]));
        assert_eq!(result["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(result["permitsTopologyChange"].as_bool(), Some(false));
        assert_eq!(result["evidence"].as_str(), Some("numerical_uncertified"));
        assert_eq!(result["components"].as_array().map(Vec::len), Some(1));
        let circle = &result["components"][0];
        assert_eq!(circle["kind"].as_str(), Some("circle"));
        assert!((circle["radius"].as_f64().unwrap() - 2.).abs() <= 1e-12);
        assert!((circle["center"][2].as_f64().unwrap() - 2.5).abs() <= 1e-12);
        assert_eq!(circle["full"].as_bool(), Some(true));
        assert_eq!(circle["coneUv"].as_array().map(Vec::len), Some(4));
        assert!(circle["maxSampleResidual"].as_f64().unwrap() <= 1e-12);
        let result = run(&plane_patch([-5., 0., -2.], [10., 0., 0.], [0., 0., 9.]));
        assert_eq!(result["components"].as_array().map(Vec::len), Some(2));
        for line in result["components"].as_array().unwrap() {
            assert_eq!(line["kind"].as_str(), Some("line"));
            assert_eq!(line["contact"].as_str(), Some("boundary"));
            assert_eq!(line["curve"]["degree"].as_f64(), Some(1.));
            assert!((line["start"][0].as_f64().unwrap().abs() - 3.).abs() <= 1e-12);
            assert!((line["end"][0].as_f64().unwrap().abs() - 1.).abs() <= 1e-12);
            assert!((line["end"][2].as_f64().unwrap() - 5.).abs() <= 1e-12);
            assert!(!line["coneUv"].as_array().unwrap().is_empty());
            assert!(line["maxSampleResidual"].as_f64().unwrap() <= 1e-12);
        }
    }
    #[test]
    fn serialized_plane_cone_classification_regions_are_explicit() {
        let cone = brep_core::analytic::frustum(3., 1., 5.).unwrap();
        let cylinder = brep_core::analytic::cylinder(2., 5.).unwrap();
        let run = |plane: &brep_core::Model, second: &brep_core::Model| {
            super::dispatch(
                value_codec::json!({"op":"brep_intersect_plane_cone","first":plane,"second":second}),
            )
            .unwrap()
        };
        // Ring-plane coincidence: the coincident cap/rim region, never a
        // guessed circle.
        let coincident = run(
            &plane_patch([-4., -4., 5.], [8., 0., 0.], [0., 8., 0.]),
            &cone,
        );
        assert_eq!(coincident["coverage"].as_str(), Some("incomplete"));
        assert_eq!(coincident["components"].as_array().map(Vec::len), Some(0));
        assert_eq!(
            coincident["unresolved"][0]["reason"].as_str(),
            Some("coincident_trim")
        );
        // Oblique plane through the apex: a multiple-root contact.
        let apex = [0., 0., 7.5];
        let apex_plane = plane_patch(
            [apex[0] - 6., apex[1] - 6.4, apex[2] + 4.8],
            [12., 0., 0.],
            [0., 12.8, -9.6],
        );
        let apex_hit = run(&apex_plane, &cone);
        assert_eq!(apex_hit["components"].as_array().map(Vec::len), Some(0));
        assert_eq!(
            apex_hit["unresolved"][0]["reason"].as_str(),
            Some("tangency_or_multiple_root")
        );
        // Clear miss: empty and resolved.
        let miss = run(
            &plane_patch([-4., -4., 7.], [8., 0., 0.], [0., 8., 0.]),
            &cone,
        );
        assert_eq!(miss["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(miss["components"].as_array().map(Vec::len), Some(0));
        // An equal-radius frustum is a cylinder: refused here.
        let refused = run(
            &plane_patch([-4., -4., 2.5], [8., 0., 0.], [0., 8., 0.]),
            &cylinder,
        );
        assert_eq!(
            refused["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
    }
    #[test]
    fn serialized_plane_torus_report_carries_exact_circles_and_stays_uncertified() {
        let torus = brep_core::analytic::torus(3., 1.).unwrap();
        let run = |plane: &brep_core::Model| {
            super::dispatch(
                value_codec::json!({"op":"brep_intersect_plane_torus","first":plane,"second":torus}),
            )
            .unwrap()
        };
        // Perpendicular plane z = 0.4: the exact parallel pair 3 +- sqrt(0.84).
        let result = run(&plane_patch([-5., -5., 0.4], [10., 0., 0.], [0., 10., 0.]));
        assert_eq!(result["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(result["permitsTopologyChange"].as_bool(), Some(false));
        assert_eq!(result["evidence"].as_str(), Some("numerical_uncertified"));
        assert_eq!(result["components"].as_array().map(Vec::len), Some(2));
        let s = (1_f64 - 0.16).sqrt();
        for (k, circle) in result["components"].as_array().unwrap().iter().enumerate() {
            assert_eq!(circle["kind"].as_str(), Some("circle"));
            let oracle = 3. + if k == 0 { s } else { -s };
            assert!((circle["radius"].as_f64().unwrap() - oracle).abs() <= 1e-12);
            assert!((circle["center"][2].as_f64().unwrap() - 0.4).abs() <= 1e-12);
            assert_eq!(circle["full"].as_bool(), Some(true));
            assert_eq!(circle["curve"]["weights"].as_array().map(Vec::len), Some(9));
            assert_eq!(
                circle["planeUv"][0]["arcs"].as_array().map(Vec::len),
                Some(4)
            );
            // Iso-v parallel lifts: one degree-1 line per revolution quadrant.
            assert_eq!(circle["torusUv"].as_array().map(Vec::len), Some(4));
            for lift in circle["torusUv"].as_array().unwrap() {
                assert_eq!(lift["arcs"][0]["degree"].as_f64(), Some(1.));
                assert_eq!(
                    lift["arcs"][0]["controlPoints"][0][1],
                    lift["arcs"][0]["controlPoints"][1][1]
                );
            }
            assert!(circle["maxSampleResidual"].as_f64().unwrap() <= 1e-10);
        }
        // Through-axis plane y = 0: the meridian pair of radius 1 at (+-3,0,0).
        let result = run(&plane_patch([-5., 0., -5.], [10., 0., 0.], [0., 0., 10.]));
        assert_eq!(result["components"].as_array().map(Vec::len), Some(2));
        for circle in result["components"].as_array().unwrap() {
            assert!((circle["radius"].as_f64().unwrap() - 1.).abs() <= 1e-12);
            assert!((circle["center"][0].as_f64().unwrap().abs() - 3.).abs() <= 1e-12);
            // Iso-u meridian lifts: one degree-1 line per profile quadrant.
            assert_eq!(circle["torusUv"].as_array().map(Vec::len), Some(4));
            for lift in circle["torusUv"].as_array().unwrap() {
                assert_eq!(
                    lift["arcs"][0]["controlPoints"][0][0],
                    lift["arcs"][0]["controlPoints"][1][0]
                );
            }
            assert!(circle["maxSampleResidual"].as_f64().unwrap() <= 1e-10);
        }
    }
    #[test]
    fn serialized_plane_torus_classification_regions_are_explicit() {
        let torus = brep_core::analytic::torus(3., 1.).unwrap();
        let run = |plane: &brep_core::Model, second: &brep_core::Model| {
            super::dispatch(
                value_codec::json!({"op":"brep_intersect_plane_torus","first":plane,"second":second}),
            )
            .unwrap()
        };
        let at = |z: f64| plane_patch([-5., -5., z], [10., 0., 0.], [0., 10., 0.]);
        // Tangency |h| == r: never a guessed circle.
        let tangent = run(&at(1.), &torus);
        assert_eq!(tangent["coverage"].as_str(), Some("incomplete"));
        assert_eq!(tangent["components"].as_array().map(Vec::len), Some(0));
        assert_eq!(
            tangent["unresolved"][0]["reason"].as_str(),
            Some("tangency_or_multiple_root")
        );
        // Clear miss: empty and resolved.
        let miss = run(&at(2.), &torus);
        assert_eq!(miss["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(miss["components"].as_array().map(Vec::len), Some(0));
        // Offset parallel-to-axis plane (Cassini oval): unsupported.
        let cassini = run(
            &plane_patch([-5., 0.5, -5.], [10., 0., 0.], [0., 0., 10.]),
            &torus,
        );
        assert_eq!(
            cassini["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
        // Oblique plane: the quartic section is refused honestly.
        let s = std::f64::consts::FRAC_1_SQRT_2;
        let oblique = run(
            &plane_patch(
                [-5., -5. * s, -5. * s],
                [10., 0., 0.],
                [0., 10. * s, 10. * s],
            ),
            &torus,
        );
        assert_eq!(
            oblique["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
        // Non-canonical operands: a sphere is not the torus; a solid is not
        // the planar patch (fixed operand order).
        let refused = run(&at(0.4), &brep_core::analytic::sphere(2.).unwrap());
        assert_eq!(
            refused["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
        let refused = run(&brep_core::analytic::sphere(2.).unwrap(), &torus);
        assert_eq!(
            refused["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
    }
    #[test]
    fn serialized_sphere_torus_report_carries_exact_circles_and_stays_uncertified() {
        let torus = brep_core::analytic::torus(3., 1.).unwrap();
        let run = |sphere: &brep_core::Model| {
            super::dispatch(
                value_codec::json!({"op":"brep_intersect_sphere_torus","first":sphere,"second":torus}),
            )
            .unwrap()
        };
        // Sphere r = sqrt(5.2) centered at the torus center: the exact circle
        // pair of radius 2.2 at z = +-0.6.
        let result = run(&brep_core::analytic::sphere(5.2_f64.sqrt()).unwrap());
        assert_eq!(result["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(result["permitsTopologyChange"].as_bool(), Some(false));
        assert_eq!(result["evidence"].as_str(), Some("numerical_uncertified"));
        assert_eq!(result["components"].as_array().map(Vec::len), Some(2));
        for (k, circle) in result["components"].as_array().unwrap().iter().enumerate() {
            assert_eq!(circle["kind"].as_str(), Some("circle"));
            assert!((circle["radius"].as_f64().unwrap() - 2.2).abs() <= 1e-12);
            let z = if k == 0 { -0.6 } else { 0.6 };
            assert!((circle["center"][2].as_f64().unwrap() - z).abs() <= 1e-12);
            assert_eq!(circle["curve"]["weights"].as_array().map(Vec::len), Some(9));
            // Iso-v parallel lifts: one degree-1 line per revolution quadrant.
            assert_eq!(circle["torusUv"].as_array().map(Vec::len), Some(4));
            for lift in circle["torusUv"].as_array().unwrap() {
                assert_eq!(lift["arcs"][0]["degree"].as_f64(), Some(1.));
                assert_eq!(
                    lift["arcs"][0]["controlPoints"][0][1],
                    lift["arcs"][0]["controlPoints"][1][1]
                );
            }
            assert!(!circle["sphereUv"].as_array().unwrap().is_empty());
            assert!(circle["maxSampleResidual"].as_f64().unwrap() <= 1e-10);
        }
    }
    #[test]
    fn serialized_sphere_torus_classification_regions_are_explicit() {
        let torus = brep_core::analytic::torus(3., 1.).unwrap();
        let run = |sphere: &brep_core::Model, second: &brep_core::Model| {
            super::dispatch(
                value_codec::json!({"op":"brep_intersect_sphere_torus","first":sphere,"second":second}),
            )
            .unwrap()
        };
        // Tangency r == R - r_t at the inner equator: never a guessed circle.
        let tangent = run(&brep_core::analytic::sphere(2.).unwrap(), &torus);
        assert_eq!(tangent["coverage"].as_str(), Some("incomplete"));
        assert_eq!(tangent["components"].as_array().map(Vec::len), Some(0));
        assert_eq!(
            tangent["unresolved"][0]["reason"].as_str(),
            Some("tangency_or_multiple_root")
        );
        // Clear miss: small sphere in the hole, empty and resolved.
        let miss = run(
            &brep_core::transform::affine(
                &brep_core::analytic::sphere(1.).unwrap(),
                [
                    [1., 0., 0., 0.],
                    [0., 1., 0., 0.],
                    [0., 0., 1., 0.5],
                    [0., 0., 0., 1.],
                ],
            )
            .unwrap(),
            &torus,
        );
        assert_eq!(miss["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(miss["components"].as_array().map(Vec::len), Some(0));
        // Near-axial offset within the recognition band: near_coincidence.
        let near = run(
            &brep_core::transform::affine(
                &brep_core::analytic::sphere(5.2_f64.sqrt()).unwrap(),
                [
                    [1., 0., 0., 1e-10],
                    [0., 1., 0., 0.],
                    [0., 0., 1., 0.],
                    [0., 0., 0., 1.],
                ],
            )
            .unwrap(),
            &torus,
        );
        assert_eq!(
            near["unresolved"][0]["reason"].as_str(),
            Some("near_coincidence")
        );
        // Clearly off-axis: the general quartic is unsupported, no fallback.
        let off = run(
            &brep_core::transform::affine(
                &brep_core::analytic::sphere(5.2_f64.sqrt()).unwrap(),
                [
                    [1., 0., 0., 0.5],
                    [0., 1., 0., 0.],
                    [0., 0., 1., 0.],
                    [0., 0., 0., 1.],
                ],
            )
            .unwrap(),
            &torus,
        );
        assert_eq!(
            off["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
        // Non-canonical operands and swapped order: unsupported regions.
        let refused = run(&torus, &brep_core::analytic::sphere(2.).unwrap());
        assert_eq!(
            refused["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
        let refused = run(
            &brep_core::analytic::sphere(2.).unwrap(),
            &brep_core::analytic::cylinder(1., 3.).unwrap(),
        );
        assert_eq!(
            refused["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
    }
    #[test]
    fn serialized_cylinder_torus_report_carries_exact_circles_and_stays_uncertified() {
        let torus = brep_core::analytic::torus(3., 1.).unwrap();
        let run = |cylinder: &brep_core::Model| {
            super::dispatch(
                value_codec::json!({"op":"brep_intersect_cylinder_torus","first":cylinder,"second":torus}),
            )
            .unwrap()
        };
        // Cylinder R_c = 2.2 spanning z in -4..4: the exact side circle pair
        // of radius 2.2 at z = +-sqrt(1 - 0.64) = +-0.6.
        let cylinder = brep_core::transform::affine(
            &brep_core::analytic::cylinder(2.2, 8.).unwrap(),
            [
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 1., -4.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let result = run(&cylinder);
        assert_eq!(result["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(result["permitsTopologyChange"].as_bool(), Some(false));
        assert_eq!(result["evidence"].as_str(), Some("numerical_uncertified"));
        assert_eq!(result["components"].as_array().map(Vec::len), Some(2));
        for (k, circle) in result["components"].as_array().unwrap().iter().enumerate() {
            assert_eq!(circle["kind"].as_str(), Some("circle"));
            assert!((circle["radius"].as_f64().unwrap() - 2.2).abs() <= 1e-12);
            let z = if k == 0 { -0.6 } else { 0.6 };
            assert!((circle["center"][2].as_f64().unwrap() - z).abs() <= 1e-12);
            assert_eq!(circle["curve"]["weights"].as_array().map(Vec::len), Some(9));
            // Side lifts: one degree-1 iso-v line per side patch.
            assert_eq!(circle["cylinderUv"].as_array().map(Vec::len), Some(4));
            for lift in circle["cylinderUv"].as_array().unwrap() {
                assert_eq!(lift["arcs"][0]["degree"].as_f64(), Some(1.));
                assert_eq!(
                    lift["arcs"][0]["controlPoints"][0][1],
                    lift["arcs"][0]["controlPoints"][1][1]
                );
            }
            // Torus lifts: one degree-1 iso-v line per revolution quadrant.
            assert_eq!(circle["torusUv"].as_array().map(Vec::len), Some(4));
            assert!(circle["maxSampleResidual"].as_f64().unwrap() <= 1e-10);
        }
        // Short wide cylinder R_c = 4.5 spanning -0.25..0.25: four cap
        // circles, radii 3 +- sqrt(1 - 0.0625) at h_c = +-0.25.
        let cap = brep_core::transform::affine(
            &brep_core::analytic::cylinder(4.5, 0.5).unwrap(),
            [
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 1., -0.25],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let result = run(&cap);
        assert_eq!(result["components"].as_array().map(Vec::len), Some(4));
        let s = (1_f64 - 0.0625).sqrt();
        for (k, circle) in result["components"].as_array().unwrap().iter().enumerate() {
            let (z, rho) = [
                (-0.25, 3. - s),
                (-0.25, 3. + s),
                (0.25, 3. - s),
                (0.25, 3. + s),
            ][k];
            assert!((circle["radius"].as_f64().unwrap() - rho).abs() <= 1e-12);
            assert!((circle["center"][2].as_f64().unwrap() - z).abs() <= 1e-12);
            // Cap lift: four exact 90-degree UV arcs on one cap face.
            assert_eq!(circle["cylinderUv"].as_array().map(Vec::len), Some(1));
            assert_eq!(
                circle["cylinderUv"][0]["arcs"].as_array().map(Vec::len),
                Some(4)
            );
            assert!(circle["maxSampleResidual"].as_f64().unwrap() <= 1e-10);
        }
    }
    #[test]
    fn serialized_cylinder_torus_classification_regions_are_explicit() {
        let torus = brep_core::analytic::torus(3., 1.).unwrap();
        let run = |first: &brep_core::Model, second: &brep_core::Model| {
            super::dispatch(
                value_codec::json!({"op":"brep_intersect_cylinder_torus","first":first,"second":second}),
            )
            .unwrap()
        };
        let at = |r: f64, x: f64| {
            brep_core::transform::affine(
                &brep_core::analytic::cylinder(r, 8.).unwrap(),
                [
                    [1., 0., 0., x],
                    [0., 1., 0., 0.],
                    [0., 0., 1., -4.],
                    [0., 0., 0., 1.],
                ],
            )
            .unwrap()
        };
        // Meridian tangency R_c == R - r = 2: never a guessed circle.
        let tangent = run(&at(2., 0.), &torus);
        assert_eq!(tangent["coverage"].as_str(), Some("incomplete"));
        assert_eq!(tangent["components"].as_array().map(Vec::len), Some(0));
        assert_eq!(
            tangent["unresolved"][0]["reason"].as_str(),
            Some("tangency_or_multiple_root")
        );
        // Clear miss: the line rho = 2 - 1e-9 misses the meridian circle.
        let miss = run(&at(2. - 1e-9, 0.), &torus);
        assert_eq!(miss["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(miss["components"].as_array().map(Vec::len), Some(0));
        // Near-coaxial offset within the recognition band: near_coincidence.
        let near = run(&at(2.2, 1e-10), &torus);
        assert_eq!(
            near["unresolved"][0]["reason"].as_str(),
            Some("near_coincidence")
        );
        // Clearly off-axis: the general quartic is unsupported, no fallback.
        let off = run(&at(2.2, 0.5), &torus);
        assert_eq!(
            off["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
        // Non-canonical operands and swapped order: unsupported regions.
        let refused = run(&torus, &brep_core::analytic::cylinder(2.2, 8.).unwrap());
        assert_eq!(
            refused["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
        let refused = run(
            &brep_core::analytic::cylinder(2.2, 8.).unwrap(),
            &brep_core::analytic::sphere(2.).unwrap(),
        );
        assert_eq!(
            refused["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
    }
    #[test]
    fn serialized_cone_torus_report_carries_exact_circles_and_stays_uncertified() {
        let torus = brep_core::analytic::torus(3., 1.).unwrap();
        let run = |cone: &brep_core::Model| {
            super::dispatch(
                value_codec::json!({"op":"brep_intersect_cone_torus","first":cone,"second":torus}),
            )
            .unwrap()
        };
        // Frustum r 1 -> 3 over z 0..6 with its bottom ring at z=-3: the
        // exact side circle pair (rho=2, z=0) and (rho=2.2, z=0.6) — the
        // meridian quadratic 5 t^2 - 33 t + 54 = 0 has roots t = 3, 3.6.
        let cone = brep_core::transform::affine(
            &brep_core::analytic::frustum(1., 3., 6.).unwrap(),
            [
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 1., -3.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let result = run(&cone);
        assert_eq!(result["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(result["permitsTopologyChange"].as_bool(), Some(false));
        assert_eq!(result["evidence"].as_str(), Some("numerical_uncertified"));
        assert_eq!(result["components"].as_array().map(Vec::len), Some(2));
        for (k, circle) in result["components"].as_array().unwrap().iter().enumerate() {
            assert_eq!(circle["kind"].as_str(), Some("circle"));
            let (rho, z) = [(2., 0.), (2.2, 0.6)][k];
            assert!((circle["radius"].as_f64().unwrap() - rho).abs() <= 1e-12);
            assert!((circle["center"][2].as_f64().unwrap() - z).abs() <= 1e-12);
            assert_eq!(circle["curve"]["weights"].as_array().map(Vec::len), Some(9));
            // Cone side lifts: one degree-1 iso-v line per side patch.
            assert_eq!(circle["coneUv"].as_array().map(Vec::len), Some(4));
            for lift in circle["coneUv"].as_array().unwrap() {
                assert_eq!(lift["arcs"][0]["degree"].as_f64(), Some(1.));
                assert_eq!(
                    lift["arcs"][0]["controlPoints"][0][1],
                    lift["arcs"][0]["controlPoints"][1][1]
                );
            }
            // Torus lifts: one degree-1 iso-v line per revolution quadrant.
            assert_eq!(circle["torusUv"].as_array().map(Vec::len), Some(4));
            assert!(circle["maxSampleResidual"].as_f64().unwrap() <= 1e-10);
        }
        // Frustum r 2 -> 4 over z 0..6 against the torus lifted to z=6.5:
        // the side line misses the meridian circle and the top cap plane at
        // h_c = -0.5 cuts the cap circle pair 3 +- sqrt(0.75), both inside
        // the r_top=4 disk.
        let cap = brep_core::transform::affine(
            &brep_core::analytic::torus(3., 1.).unwrap(),
            [
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 1., 6.5],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let run2 = |cone: &brep_core::Model| {
            super::dispatch(
                value_codec::json!({"op":"brep_intersect_cone_torus","first":cone,"second":cap}),
            )
            .unwrap()
        };
        let result = run2(&brep_core::analytic::frustum(2., 4., 6.).unwrap());
        assert_eq!(result["components"].as_array().map(Vec::len), Some(2));
        let s = 0.75_f64.sqrt();
        for (k, circle) in result["components"].as_array().unwrap().iter().enumerate() {
            let rho = [3. - s, 3. + s][k];
            assert!((circle["radius"].as_f64().unwrap() - rho).abs() <= 1e-12);
            assert!((circle["center"][2].as_f64().unwrap() - 6.).abs() <= 1e-12);
            // Cap lift: four exact 90-degree UV arcs on one cap face.
            assert_eq!(circle["coneUv"].as_array().map(Vec::len), Some(1));
            assert_eq!(
                circle["coneUv"][0]["arcs"].as_array().map(Vec::len),
                Some(4)
            );
            assert!(circle["maxSampleResidual"].as_f64().unwrap() <= 1e-10);
        }
    }
    #[test]
    fn serialized_cone_torus_classification_regions_are_explicit() {
        let torus = brep_core::analytic::torus(3., 1.).unwrap();
        let run = |first: &brep_core::Model, second: &brep_core::Model| {
            super::dispatch(
                value_codec::json!({"op":"brep_intersect_cone_torus","first":first,"second":second}),
            )
            .unwrap()
        };
        let at = |x: f64| {
            brep_core::transform::affine(
                &brep_core::analytic::frustum(1., 3., 6.).unwrap(),
                [
                    [1., 0., 0., x],
                    [0., 1., 0., 0.],
                    [0., 0., 1., -3.],
                    [0., 0., 0., 1.],
                ],
            )
            .unwrap()
        };
        // Meridian tangency: against the frustum r 1 -> 3 with its bottom
        // ring at z=0, the torus at z = 6 - sqrt(10) makes the side line
        // tangent to the meridian circle — never a guessed circle.
        let base = brep_core::analytic::frustum(1., 3., 6.).unwrap();
        let tangent_torus = brep_core::transform::affine(
            &torus,
            [
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 1., 6. - 10_f64.sqrt()],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let tangent = run(&base, &tangent_torus);
        assert_eq!(tangent["coverage"].as_str(), Some("incomplete"));
        assert_eq!(tangent["components"].as_array().map(Vec::len), Some(0));
        assert_eq!(
            tangent["unresolved"][0]["reason"].as_str(),
            Some("tangency_or_multiple_root")
        );
        // Near-coaxial offset within the recognition band: near_coincidence.
        let near = run(&at(1e-10), &torus);
        assert_eq!(
            near["unresolved"][0]["reason"].as_str(),
            Some("near_coincidence")
        );
        // Clearly off-axis: the general quartic is unsupported, no fallback.
        let off = run(&at(0.5), &torus);
        assert_eq!(
            off["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
        // Non-canonical operands and swapped order: unsupported regions.
        let refused = run(&torus, &brep_core::analytic::frustum(1., 3., 6.).unwrap());
        assert_eq!(
            refused["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
        let refused = run(
            &brep_core::analytic::frustum(1., 3., 6.).unwrap(),
            &brep_core::analytic::sphere(2.).unwrap(),
        );
        assert_eq!(
            refused["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
    }
    #[test]
    fn serialized_torus_torus_report_carries_exact_circles_and_stays_uncertified() {
        let first = brep_core::analytic::torus(3., 1.).unwrap();
        let run = |second: &brep_core::Model| {
            super::dispatch(
                value_codec::json!({"op":"brep_intersect_torus_torus","first":first,"second":second}),
            )
            .unwrap()
        };
        // Torus R=3, r=1 at the origin against torus R=3, r=sqrt(5.2)
        // lifted to z=3: the meridian centers (3,0) and (3,3) sit at d=3,
        // a=0.8, l=0.6 — the exact circle pair of radii 2.4 and 3.6, both
        // at z=0.8, sorted by height then radius.
        let second = brep_core::transform::affine(
            &brep_core::analytic::torus(3., 5.2_f64.sqrt()).unwrap(),
            [
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 1., 3.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let result = run(&second);
        assert_eq!(result["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(result["permitsTopologyChange"].as_bool(), Some(false));
        assert_eq!(result["evidence"].as_str(), Some("numerical_uncertified"));
        assert_eq!(result["components"].as_array().map(Vec::len), Some(2));
        for (k, circle) in result["components"].as_array().unwrap().iter().enumerate() {
            assert_eq!(circle["kind"].as_str(), Some("circle"));
            let rho = [2.4, 3.6][k];
            assert!((circle["radius"].as_f64().unwrap() - rho).abs() <= 1e-12);
            assert!((circle["center"][2].as_f64().unwrap() - 0.8).abs() <= 1e-12);
            assert_eq!(circle["curve"]["weights"].as_array().map(Vec::len), Some(9));
            // Both torus lifts: one degree-1 iso-v line per revolution
            // quadrant patch of the profile row.
            for key in ["firstUv", "secondUv"] {
                assert_eq!(circle[key].as_array().map(Vec::len), Some(4));
                for lift in circle[key].as_array().unwrap() {
                    assert_eq!(lift["arcs"][0]["degree"].as_f64(), Some(1.));
                    assert_eq!(
                        lift["arcs"][0]["controlPoints"][0][1],
                        lift["arcs"][0]["controlPoints"][1][1]
                    );
                }
            }
            assert!(circle["maxSampleResidual"].as_f64().unwrap() <= 1e-10);
        }
    }
    #[test]
    fn serialized_torus_torus_classification_regions_are_explicit() {
        let first = brep_core::analytic::torus(3., 1.).unwrap();
        let run = |a: &brep_core::Model, b: &brep_core::Model| {
            super::dispatch(
                value_codec::json!({"op":"brep_intersect_torus_torus","first":a,"second":b}),
            )
            .unwrap()
        };
        let at = |z: f64| {
            brep_core::transform::affine(
                &brep_core::analytic::torus(3., 1.).unwrap(),
                [
                    [1., 0., 0., 0.],
                    [0., 1., 0., 0.],
                    [0., 0., 1., z],
                    [0., 0., 0., 1.],
                ],
            )
            .unwrap()
        };
        // External meridian tangency: torus R=3, r=1 at z=2 touches the
        // first tube (d = 2 = r_1 + r_2) — never a guessed circle.
        let tangent = run(&first, &at(2.));
        assert_eq!(tangent["coverage"].as_str(), Some("incomplete"));
        assert_eq!(tangent["components"].as_array().map(Vec::len), Some(0));
        assert_eq!(
            tangent["unresolved"][0]["reason"].as_str(),
            Some("tangency_or_multiple_root")
        );
        // Coincident tori: coincident_trim, never a guessed curve.
        let coincident = run(&first, &brep_core::analytic::torus(3., 1.).unwrap());
        assert_eq!(coincident["coverage"].as_str(), Some("incomplete"));
        assert_eq!(
            coincident["unresolved"][0]["reason"].as_str(),
            Some("coincident_trim")
        );
        // Near-coaxial offset within the recognition band: near_coincidence.
        let near = brep_core::transform::affine(
            &at(3.),
            [
                [1., 0., 0., 1e-10],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let near = run(&first, &near);
        assert_eq!(
            near["unresolved"][0]["reason"].as_str(),
            Some("near_coincidence")
        );
        // Clearly off-axis: the general quartic is unsupported, no fallback.
        let off = brep_core::transform::affine(
            &at(3.),
            [
                [1., 0., 0., 0.5],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let off = run(&first, &off);
        assert_eq!(
            off["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
        // Non-canonical operands on either side: unsupported regions.
        let refused = run(&first, &brep_core::analytic::sphere(2.).unwrap());
        assert_eq!(
            refused["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
        let refused = run(&brep_core::analytic::cylinder(2.2, 8.).unwrap(), &first);
        assert_eq!(
            refused["unresolved"][0]["reason"].as_str(),
            Some("unsupported_surface")
        );
    }
}
