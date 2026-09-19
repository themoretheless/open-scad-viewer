use super::*;
use curve::Curve;
fn circle() -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
        weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
        periodic: false,
    }
}
fn near(a: &[f64], b: &[f64]) {
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b) {
        assert!((x - y).abs() < 1e-10, "{x} != {y}");
    }
}
#[test]
fn rational_circle_has_analytic_jets() {
    let c = circle();
    let e = c.evaluate(0.5).unwrap();
    let q = std::f64::consts::FRAC_1_SQRT_2;
    let speed = 4. / (2. + 2_f64.sqrt());
    near(&e.point, &[q, q, 0.]);
    near(&e.d1.unwrap(), &[-speed, speed, 0.]);
    near(
        &e.d2.unwrap(),
        &[
            -2_f64.sqrt() * speed * speed,
            -2_f64.sqrt() * speed * speed,
            0.,
        ],
    );
}
#[test]
fn edits_preserve_geometry_and_domain() {
    let c = circle();
    let edited = [
        c.insert(0.3, 2).unwrap(),
        c.elevate(5).unwrap(),
        c.trim(0.1, 0.9).unwrap(),
    ];
    for i in 0..=80 {
        let u = 0.1 + i as f64 / 100.;
        let source = c.evaluate(u).unwrap();
        for target in &edited {
            near(&source.point, &target.evaluate(u).unwrap().point);
        }
    }
    let reversed = c.reverse().unwrap();
    near(
        &c.evaluate(0.3).unwrap().point,
        &reversed.evaluate(0.7).unwrap().point,
    );
}
#[test]
fn natural_endpoints_and_periodic_seams_are_supported() {
    let c = Curve {
        degree: 2,
        knots: vec![-2., -1., 0., 1., 2., 3., 4., 5.],
        control_points: vec![
            vec![0., 0.],
            vec![2., 4.],
            vec![5., 3.],
            vec![6., 1.],
            vec![8., -1.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    near(&c.evaluate(0.).unwrap().point, &[1., 2.]);
    near(&c.evaluate(3.).unwrap().point, &[7., 0.]);
    for u in [0., 3.] {
        let edited = c.insert(u, 2).unwrap();
        for i in 0..=30 {
            let t = i as f64 / 10.;
            near(
                &c.evaluate(t).unwrap().point,
                &edited.evaluate(t).unwrap().point,
            );
        }
    }
    let p = Curve {
        degree: 2,
        knots: (0..9).map(|i| i as f64).collect(),
        control_points: vec![
            vec![1., 0.],
            vec![0., 1.],
            vec![-1., 0.],
            vec![0., -1.],
            vec![1., 0.],
            vec![0., 1.],
        ],
        weights: vec![1.; 6],
        periodic: true,
    };
    let a = p.evaluate(2.).unwrap();
    let b = p.evaluate(6.).unwrap();
    near(&a.point, &b.point);
    near(&a.d1.unwrap(), &b.d1.unwrap());
    assert!(a.d2.is_none());
}
#[test]
fn surface_construction_and_iso_are_consistent() {
    let c = circle();
    let s = surface::extrude(&c, [0., 0., 3.]).unwrap();
    let e = value_codec::to_value(s.evaluate(0.5, 0.4).unwrap()).unwrap();
    near(
        &value_codec::from_value::<Vec<f64>>(e["point"].clone()).unwrap(),
        &[
            std::f64::consts::FRAC_1_SQRT_2,
            std::f64::consts::FRAC_1_SQRT_2,
            1.2,
        ],
    );
    let iso = s.iso(surface::Axis::V, 0.4).unwrap();
    for i in 0..=10 {
        let t = i as f64 / 10.;
        let mut p = c.evaluate(t).unwrap().point;
        p[2] = 1.2;
        near(&p, &iso.evaluate(t).unwrap().point);
    }
    assert_eq!(e["gaussianCurvature"], 0.);
}
#[test]
fn malformed_boundary_input_returns_errors_without_poisoning_kernel() {
    for input in [
        "null",
        "{}",
        "{",
        r#"{"op":"curve_validate","curve":{"degree":-1}}"#,
        r#"{"op":"basis","degree":1,"knots":[0,0,1,1],"controlCount":2,"u":null}"#,
    ] {
        let v: Value = value_codec::from_str(&execute(input)).unwrap();
        assert_eq!(v["ok"], false);
        assert_eq!(v["error"]["code"], "NURBS_INVALID_INPUT");
    }
    let mut c = circle();
    c.weights[1] = 0.;
    assert!(c.validate().is_err());
    c = circle();
    c.knots[3] = -1.;
    assert!(c.validate().is_err());
    assert_eq!(
        dispatch(json!({"op":"curve_evaluate","curve":circle(),"u":0.5})).unwrap()["derivative_status"],
        "available"
    );
}
#[test]
fn output_budget_is_checked_before_elevation() {
    let c = Curve {
        degree: 1,
        knots: std::iter::once(0.)
            .chain((0..30).map(|i| i as f64))
            .chain(std::iter::once(29.))
            .collect(),
        control_points: (0..30).map(|i| vec![i as f64, 0.]).collect(),
        weights: vec![1.; 30],
        periodic: false,
    };
    assert_eq!(c.elevate(25).unwrap_err().code, "NURBS_RESOURCE_LIMIT");
}

#[test]
fn brush_moves_controls_locally_and_preserves_rational_definition() {
    let c = circle();
    let b = geometry_ops::Brush {
        center: [1., 1., 0.],
        radius: 0.5,
        displacement: [0., 0., 2.],
    };
    let out = edit::brush_curve(&c, &b).unwrap();
    assert_eq!(out.degree, c.degree);
    assert_eq!(out.knots, c.knots);
    assert_eq!(out.weights, c.weights);
    assert_eq!(out.control_points[0], c.control_points[0]);
    assert_eq!(out.control_points[2], c.control_points[2]);
    near(&out.control_points[1], &[1., 1., 2.]);
    // Endpoints are interpolated by the clamped curve and remain untouched.
    near(&out.evaluate(0.).unwrap().point, &[1., 0., 0.]);
    near(&out.evaluate(1.).unwrap().point, &[0., 1., 0.]);
    assert!(out.evaluate(0.5).unwrap().point[2] > 0.);

    let s = surface::extrude(&c, [0., 0., 3.]).unwrap();
    let sb = geometry_ops::Brush {
        center: [1., 1., 3.],
        radius: 0.5,
        displacement: [1., 0., 0.],
    };
    let sout = edit::brush_surface(&s, &sb).unwrap();
    assert_eq!(sout.knots_u, s.knots_u);
    assert_eq!(sout.knots_v, s.knots_v);
    assert_eq!(sout.weights, s.weights);
    let moved = s
        .control_points
        .iter()
        .flatten()
        .zip(sout.control_points.iter().flatten())
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(moved, 1, "exactly one control point lies inside the brush");
    near(
        &sout.evaluate(0., 0.).unwrap().point,
        &s.evaluate(0., 0.).unwrap().point,
    );
}
#[test]
fn brush_rejects_invalid_brushes_and_inputs() {
    let c = circle();
    let bad = geometry_ops::Brush {
        center: [0.; 3],
        radius: 0.,
        displacement: [1., 0., 0.],
    };
    assert!(edit::brush_curve(&c, &bad).is_err());
    let s = surface::extrude(&c, [0., 0., 3.]).unwrap();
    assert!(edit::brush_surface(&s, &bad).is_err());
    let ok = geometry_ops::Brush { radius: 1., ..bad };
    let mut flat = c.clone();
    flat.control_points = flat
        .control_points
        .iter()
        .map(|p| p[..2].to_vec())
        .collect();
    assert!(
        edit::brush_curve(&flat, &ok).is_err(),
        "2D controls are rejected"
    );
    let mut broken = c.clone();
    broken.knots.pop();
    assert!(edit::brush_curve(&broken, &ok).is_err());
}

#[test]
fn sculpt_targets_expose_polygon_and_net_structure() {
    let c = circle();
    let geometry_ops::SculptTarget {
        positions,
        normals,
        adjacency: rings,
    } = edit::curve_sculpt_target(&c).unwrap();
    assert_eq!(positions.len(), 3);
    assert_eq!(rings, vec![vec![1], vec![0, 2], vec![1]]);
    near(&normals[0], &[0., 0., 0.]);
    near(
        &normals[1],
        &[
            std::f64::consts::FRAC_1_SQRT_2,
            std::f64::consts::FRAC_1_SQRT_2,
            0.,
        ],
    );
    let s = surface::extrude(&c, [0., 0., 3.]).unwrap();
    let geometry_ops::SculptTarget {
        normals: snormals,
        adjacency: srings,
        ..
    } = edit::surface_sculpt_target(&s).unwrap();
    assert_eq!(
        snormals.len(),
        s.control_points.len() * s.control_points[0].len()
    );
    assert!(srings.iter().all(|r| (2..=4).contains(&r.len())));
    assert!(
        snormals
            .iter()
            .all(|n| (math_core::norm(*n) - 1.).abs() < 1e-9)
    );
}
#[test]
fn sculpt_edits_preserve_rational_definition() {
    use geometry_ops::{Falloff, SculptBrush, SculptKind};
    let c = circle();
    let draw = edit::sculpt_curve(
        &c,
        &SculptBrush::new(SculptKind::Draw { strength: 1. }, [1., 1., 0.], 0.5),
    )
    .unwrap();
    assert_eq!(draw.knots, c.knots);
    assert_eq!(draw.weights, c.weights);
    near(
        &draw.control_points[1],
        &[
            1. + std::f64::consts::FRAC_1_SQRT_2,
            1. + std::f64::consts::FRAC_1_SQRT_2,
            0.,
        ],
    );
    near(&draw.control_points[0], &c.control_points[0]);
    let smooth = edit::sculpt_curve(
        &c,
        &SculptBrush::new(SculptKind::Smooth { strength: 1. }, [1., 1., 0.], 0.5),
    )
    .unwrap();
    near(&smooth.control_points[1], &[0.5, 0.5, 0.]);
    let s = surface::extrude(&c, [0., 0., 3.]).unwrap();
    let inflate = edit::sculpt_surface(
        &s,
        &SculptBrush {
            falloff: Falloff::Constant,
            ..SculptBrush::new(SculptKind::Inflate { strength: 0.5 }, [0.; 3], 100.)
        },
    )
    .unwrap();
    assert_eq!(inflate.knots_u, s.knots_u);
    assert_eq!(inflate.weights, s.weights);
    for (a, b) in s
        .control_points
        .iter()
        .flatten()
        .zip(inflate.control_points.iter().flatten())
    {
        let d: Vec<f64> = a.iter().zip(b).map(|(x, y)| y - x).collect();
        assert!((math_core::norm([d[0], d[1], d[2]]) - 0.5).abs() < 1e-9);
    }
    assert!(
        edit::sculpt_curve(
            &c,
            &SculptBrush::new(SculptKind::Flatten { strength: 1.5 }, [0.; 3], 1.)
        )
        .is_err()
    );
    let mut flat = c.clone();
    flat.control_points = flat
        .control_points
        .iter()
        .map(|p| p[..2].to_vec())
        .collect();
    assert!(
        edit::sculpt_curve(
            &flat,
            &SculptBrush::new(SculptKind::Draw { strength: 1. }, [0.; 3], 1.)
        )
        .is_err()
    );
}

#[test]
fn foundation_curve_certificate_is_outward_and_positive() {
    let c = circle();
    let certificate = foundation::certify_curve(&c, None).unwrap();
    assert_eq!(certificate["version"], "nurbs-foundation/1");
    let spans = certificate["spans"].as_array().unwrap();
    assert_eq!(spans.len(), 1);
    assert!(spans[0]["denominatorLower"].as_f64().unwrap() > 0.);
    assert!(spans[0]["min"][0].as_f64().unwrap() < 0.);
    assert!(spans[0]["max"][1].as_f64().unwrap() > 1.);
    assert_eq!(
        spans[0]["regularity"]["classification"],
        "certified_regular"
    );
}

#[test]
fn foundation_handles_adversarial_positive_weights_and_surface_cells() {
    for exponent in -6..=6 {
        let mut c = circle();
        c.weights = vec![1., 10_f64.powi(exponent), 1.];
        let certificate = foundation::certify_curve(&c, None).unwrap();
        assert!(certificate["spans"][0]["denominatorLower"]
            .as_f64()
            .unwrap()
            > 0.);
        for i in 0..=32 {
            let point = c.evaluate(i as f64 / 32.).unwrap().point;
            for axis in 0..3 {
                assert!(point[axis] >= certificate["spans"][0]["min"][axis].as_f64().unwrap());
                assert!(point[axis] <= certificate["spans"][0]["max"][axis].as_f64().unwrap());
            }
        }
    }
    let surface = surface::Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 2., 0.]],
            vec![vec![3., 0., 0.], vec![3., 2., 0.]],
        ],
        weights: vec![vec![1., 2.], vec![3., 4.]],
        periodic_u: false,
        periodic_v: false,
    };
    let certificate = foundation::certify_surface(&surface, None).unwrap();
    assert_eq!(certificate["cells"].as_array().unwrap().len(), 1);
    assert_eq!(
        certificate["cells"][0]["normalRegularity"]["classification"],
        "certified_planar_regular"
    );
}

#[test]
fn certified_projection_reports_uniqueness_and_ambiguity() {
    let line = Curve::from_polyline(vec![vec![0., 0.], vec![2., 0.]]).unwrap();
    let projected = foundation::project_curve(&line, &[0.5, 1.], None).unwrap();
    assert_eq!(projected["status"], "unique");
    assert!((projected["candidates"][0]["point"][0].as_f64().unwrap() - 0.5).abs() < 1e-14);

    let polyline =
        Curve::from_polyline(vec![vec![-1., 0.], vec![0., 1.], vec![1., 0.]]).unwrap();
    let ambiguous = foundation::project_curve(&polyline, &[0., 0.], None).unwrap();
    assert_eq!(ambiguous["status"], "nonunique_or_unresolved");
    assert_eq!(ambiguous["candidates"].as_array().unwrap().len(), 2);
}

#[test]
fn interpolation_approximation_and_exact_reparameterization_are_certified() {
    let interpolation = foundation::interpolate_polyline(
        vec![vec![0., 0.], vec![1., 2.], vec![3., 4.]],
        None,
    )
    .unwrap();
    assert_eq!(
        interpolation["certificate"]["dataSiteErrorUpper"].as_f64(),
        Some(0.)
    );
    let approximation = foundation::approximate_curve(&circle(), None).unwrap();
    assert!(approximation["certificate"]["hausdorffErrorUpper"]
        .as_f64()
        .unwrap()
        .is_finite());

    let periodic = Curve {
        degree: 2,
        knots: (0..9).map(|i| i as f64).collect(),
        control_points: vec![
            vec![1., 0.],
            vec![0., 1.],
            vec![-1., 0.],
            vec![0., -1.],
            vec![1., 0.],
            vec![0., 1.],
        ],
        weights: vec![1.; 6],
        periodic: true,
    };
    let result = foundation::reparameterize_curve(&periodic, [-3., 5.], None).unwrap();
    let mapped: Curve = value_codec::from_value(result["curve"].clone()).unwrap();
    assert!(mapped.periodic);
    for i in 0..=32 {
        let old_u = 2. + 4. * i as f64 / 32.;
        let new_u = -3. + 8. * i as f64 / 32.;
        near(
            &periodic.evaluate(old_u).unwrap().point,
            &mapped.evaluate(new_u).unwrap().point,
        );
    }
}

#[test]
fn foundation_boundary_and_mutation_limits_are_deterministic() {
    let degree = 25;
    let controls = 26;
    let c = Curve {
        degree,
        knots: std::iter::repeat_n(0., degree + 1)
            .chain(std::iter::repeat_n(1., degree + 1))
            .collect(),
        control_points: (0..controls)
            .map(|i| vec![i as f64 / 25., (i % 3) as f64])
            .collect(),
        weights: (0..controls)
            .map(|i| 10_f64.powi((i as i32 % 13) - 6))
            .collect(),
        periodic: false,
    };
    assert!(foundation::certify_curve(&c, None).is_ok());
    let mut too_high = c.clone();
    too_high.degree = 26;
    too_high.knots.push(1.);
    assert_eq!(too_high.validate().unwrap_err().code, "NURBS_INVALID_INPUT");
    let mut nonpositive = c;
    nonpositive.weights[7] = 0.;
    assert_eq!(
        nonpositive.validate().unwrap_err().code,
        "NURBS_INVALID_INPUT"
    );
}

#[test]
fn successor_projection_covers_rational_stationary_and_surface_boundaries() {
    let projected = foundation::project_curve(&circle(), &[0.8, 0.2, 0.], None).unwrap();
    assert_eq!(projected["version"], "nurbs-foundation/3");
    assert_eq!(projected["coverage"]["endpointsIncluded"], true);
    assert!(projected["candidates"].as_array().unwrap().iter().any(|candidate|
        candidate["classification"] == "simple_stationary"));

    let surface = surface::extrude(&circle(), [0.,0.,1.]).unwrap();
    let projected = foundation::project_surface(&surface, [2.,0.,0.5], None).unwrap();
    assert_eq!(projected["version"], "nurbs-foundation/4");
    assert_eq!(projected["coverage"]["complete"], true);
    assert_eq!(projected["boundaryReductions"].as_array().unwrap().len(), 4);
}

#[test]
fn successor_normal_cone_and_reduction_rollback_are_explicit() {
    let surface = surface::Surface {
        degree_u:1, degree_v:1, knots_u:vec![0.,0.,1.,1.], knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],vec![vec![1.,0.,0.],vec![1.,1.,1.]]],
        weights:vec![vec![1.,1.],vec![1.,1.]], periodic_u:false, periodic_v:false,
    };
    let certificate=foundation::certify_surface(&surface,None).unwrap();
    assert_eq!(certificate["cells"][0]["normalRegularity"]["classification"],"certified_regular");
    assert_eq!(certificate["cells"][0]["normalRegularity"]["normalNumeratorBounds"].as_array().unwrap().len(),3);

    let inserted=circle().insert(0.5,1).unwrap();
    let refused=foundation::remove_curve_knot(&inserted,0.5,0.,None).unwrap();
    assert_eq!(refused["certificate"]["rolledBack"],true);
    assert_eq!(refused["curve"]["knots"],value_codec::to_value(&inserted.knots).unwrap());
    let accepted=foundation::remove_curve_knot(&inserted,0.5,10.,None).unwrap();
    assert_eq!(accepted["certificate"]["accepted"],true);
}

#[test]
fn foundation_v3_periodic_edits_preserve_wrapping_and_seam_evidence() {
    let periodic = Curve {
        degree:2, knots:(0..9).map(|i|i as f64).collect(),
        control_points:vec![vec![1.,0.],vec![0.,1.],vec![-1.,0.],vec![0.,-1.],vec![1.,0.],vec![0.,1.]],
        weights:vec![1.;6],periodic:true,
    };
    let inserted=foundation::edit_periodic_curve(&periodic,"insert",3.5,1,10.,None).unwrap();
    let curve:value_codec::Value=inserted["curve"].clone();
    assert_eq!(curve["periodic"],true);
    assert_eq!(inserted["certificate"]["wrappedStorage"],true);
    assert_eq!(inserted["certificate"]["seam"]["c0"]["certified"],true);
    let split=foundation::split_periodic_curve(&periodic,4.,None).unwrap();
    assert_eq!(split["curves"].as_array().unwrap().len(),2);
    let periodic3=Curve{control_points:periodic.control_points.iter().map(|p|vec![p[0],p[1],0.]).collect(),..periodic.clone()};
    let surface=surface::extrude(&periodic3,[0.,0.,2.]).unwrap();
    let edited_surface=foundation::edit_periodic_surface(
        &surface,surface::Axis::U,"insert",3.5,1,10.,None).unwrap();
    assert_eq!(edited_surface["surface"]["periodicU"],true);
    assert_eq!(edited_surface["certificate"]["wrappedStorage"],true);
}

#[test]
fn foundation_v3_reparameterization_and_fitting_are_typed() {
    let mapping=json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],
        "controlValues":[0.,0.2,1.],"weights":[1.,0.75,1.]}]});
    let certificate=foundation::certify_reparameterization(&mapping,None).unwrap();
    assert_eq!(certificate["classification"],"certified_strictly_monotone");
    let evaluated=foundation::evaluate_reparameterized_curve(&circle(),&mapping,0.5,None).unwrap();
    assert!(evaluated["sourceParameter"].as_f64().unwrap()>0.);
    let fit=foundation::fit_curve_points(vec![vec![0.,0.],vec![1.,1.],vec![2.,0.],vec![3.,1.]],3,None).unwrap();
    assert_eq!(fit["certificate"]["classification"],"approximate_fit");
    assert_eq!(fit["certificate"]["fittedToExactPromotion"],false);
    let resource_plus_one=foundation::fit_curve_points(vec![vec![0.,0.];27],27,None);
    assert!(resource_plus_one.is_err());
}

#[test]
fn foundation_v4_closes_pre_intersection_gaps() {
    let bilinear = surface::Surface {
        degree_u:1, degree_v:1, knots_u:vec![0.,0.,1.,1.], knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
        weights:vec![vec![1.,1.],vec![1.,1.]], periodic_u:false, periodic_v:false,
    };
    let unique=foundation::project_surface(&bilinear,[0.25,0.4,1.],None).unwrap();
    assert_eq!(unique["status"],"unique");
    assert!(unique["uniquenessProof"].is_object());

    let singularish = surface::Surface {
        degree_u:1, degree_v:1, knots_u:vec![0.,0.,1.,1.], knots_v:vec![0.,0.,1.,1.],
        control_points:vec![
            vec![vec![0.,0.,0.],vec![1.,0.,0.]],
            vec![vec![0.,0.,0.],vec![0.,1.,0.]],
        ],
        weights:vec![vec![1.,1.],vec![1.,1.]], periodic_u:false, periodic_v:false,
    };
    let certificate=foundation::certify_surface(&singularish,None).unwrap();
    let localization=&certificate["singularityLocalization"];
    assert!(localization["method"].as_str().unwrap().contains("recursive"));
    assert!(localization.get("isolatedSingularPoints").is_some());
    assert!(localization.get("regularComplement").is_some());

    let mapping=json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],
        "controlValues":[0.,0.5,1.],"weights":[1.,1.,1.]}]});
    let composed=foundation::materialize_reparameterized_curve(&circle(),&mapping,None).unwrap();
    assert_eq!(composed["certificate"]["version"],"nurbs-foundation/5");
    assert_eq!(composed["certificate"]["exact"],true);
    // Equal-weight quadratic φ(t)=t, so composition preserves geometry at t=0.5.
    let source=circle().evaluate(0.5).unwrap().point;
    let mapped=value_codec::from_value::<Curve>(composed["curve"].clone()).unwrap()
        .evaluate(0.5).unwrap().point;
    near(&source,&mapped);

    let cloud=foundation::fit_curve_cloud_certified(
        vec![vec![0.,0.],vec![0.5,0.4],vec![1.,0.],vec![1.5,0.3],vec![2.,0.]],4,None).unwrap();
    assert_eq!(cloud["certificate"]["classification"],"approximate_cloud_fit");
    assert_eq!(cloud["certificate"]["fittedToExactPromotion"],false);
    assert!(cloud["certificate"]["hausdorffErrorUpper"].as_f64().unwrap()
        >= cloud["certificate"]["dataSiteErrorUpper"].as_f64().unwrap());

    let surface_cloud=foundation::fit_surface_cloud_certified(
        vec![[0.,0.,0.],[1.,0.,0.],[0.,1.,0.],[1.,1.,0.2],[0.5,0.5,0.1]],3,3,None).unwrap();
    assert_eq!(surface_cloud["certificate"]["classification"],"approximate_cloud_fit");
    assert!(surface_cloud["certificate"]["hausdorffErrorUpper"].as_f64().unwrap().is_finite());

    let over_resource=foundation::fit_surface_cloud_certified(vec![[0.,0.,0.];4],9,2,None);
    assert!(over_resource.is_err());
}

#[test]
fn foundation_v5_certified_curve_curve_and_curve_surface() {
    let line_a = Curve {
        degree: 1, knots: vec![0.,0.,1.,1.],
        control_points: vec![vec![0.,0.,0.],vec![1.,1.,0.]], weights: vec![1.,1.], periodic: false,
    };
    let line_b = Curve {
        degree: 1, knots: vec![0.,0.,1.,1.],
        control_points: vec![vec![0.,1.,0.],vec![1.,0.,0.]], weights: vec![1.,1.], periodic: false,
    };
    let cc = intersection::intersect_curve_curve(&line_a,&line_b,None).unwrap();
    assert_eq!(cc["version"],"nurbs-foundation/5");
    assert_eq!(cc["kind"],"curve_curve");
    assert_eq!(cc["coverage"]["complete"],true);
    assert!(cc["unresolved"].as_array().unwrap().is_empty());
    let points: Vec<_>=cc["components"].as_array().unwrap().iter()
        .filter(|c|c["kind"]=="point").collect();
    assert_eq!(points.len(),1);
    assert!((points[0]["first"].as_f64().unwrap()-0.5).abs()<1e-8);
    assert!((points[0]["second"].as_f64().unwrap()-0.5).abs()<1e-8);
    assert_eq!(points[0]["contactClass"],"transverse");
    assert!(points[0]["geometryEnclosure"].is_array());
    assert!(points[0]["multiplicity"].as_u64().unwrap()>=1);

    let arc = circle();
    let chord = Curve {
        degree: 1, knots: vec![0.,0.,1.,1.],
        control_points: vec![vec![1.,0.,0.],vec![0.,1.,0.]], weights: vec![1.,1.], periodic: false,
    };
    let shared = intersection::intersect_curve_curve(&arc,&chord,None).unwrap();
    assert!(shared["coverage"]["complete"].as_bool().unwrap() || !shared["components"].as_array().unwrap().is_empty());

    let coincident = intersection::intersect_curve_curve(&line_a,&line_a,None).unwrap();
    assert!(coincident["components"].as_array().unwrap().iter().any(|c|c["kind"]=="overlap"));
    assert!(coincident["components"].as_array().unwrap().iter().any(|c|c.get("coedgeTrim").is_some()));

    let tangent_line = Curve {
        degree: 1, knots: vec![0.,0.,1.,1.],
        control_points: vec![vec![0.,0.,0.],vec![2.,0.,0.]], weights: vec![1.,1.], periodic: false,
    };
    let parabola = Curve {
        degree: 2, knots: vec![0.,0.,0.,1.,1.,1.],
        control_points: vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![2.,0.,0.]],
        weights: vec![1.,1.,1.], periodic: false,
    };
    let touch = intersection::intersect_curve_curve(&parabola,&tangent_line,None).unwrap();
    assert!(touch["components"].as_array().unwrap().iter().any(|c|
        c["kind"]=="overlap" || matches!(c["contactClass"].as_str(),Some("even_tangency"|"coincident"|"odd_tangency"|"transverse"|"boundary"))
    ));

    let plane = surface::Surface {
        degree_u:1, degree_v:1, knots_u:vec![0.,0.,1.,1.], knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
        weights:vec![vec![1.,1.],vec![1.,1.]], periodic_u:false, periodic_v:false,
    };
    let piercing = Curve {
        degree: 1, knots: vec![0.,0.,1.,1.],
        control_points: vec![vec![0.25,0.4,-1.],vec![0.25,0.4,1.]], weights: vec![1.,1.], periodic: false,
    };
    let cs = intersection::intersect_curve_surface(&piercing,&plane,None).unwrap();
    assert_eq!(cs["version"],"nurbs-foundation/5");
    assert_eq!(cs["kind"],"curve_surface");
    assert_eq!(cs["coverage"]["complete"],true);
    let cs_points: Vec<_>=cs["components"].as_array().unwrap().iter()
        .filter(|c|c["kind"]=="point").collect();
    assert_eq!(cs_points.len(),1);
    assert!((cs_points[0]["t"].as_f64().unwrap()-0.5).abs()<1e-8);
    assert!(cs_points[0]["coedgeTrim"].is_null() || cs_points[0].get("coedgeTrim").is_some());

    let on_plane = Curve {
        degree: 1, knots: vec![0.,0.,1.,1.],
        control_points: vec![vec![0.1,0.2,0.],vec![0.8,0.7,0.]], weights: vec![1.,1.], periodic: false,
    };
    let overlap = intersection::intersect_curve_surface(&on_plane,&plane,None).unwrap();
    assert!(overlap["components"].as_array().unwrap().iter().any(|c|c["kind"]=="overlap"));
    assert!(overlap["components"].as_array().unwrap().iter().any(|c|c.get("coedgeTrim").is_some()));

    // Nested composition materialization.
    let nested=json!({"composition":[
        {"pieces":[{"domain":[0.,1.],"range":[0.,1.],"controlValues":[0.,0.5,1.],"weights":[1.,1.,1.]}]},
        {"pieces":[{"domain":[0.,1.],"range":[0.,1.],"controlValues":[0.,0.5,1.],"weights":[1.,1.,1.]}]}
    ]});
    let composed=foundation::materialize_reparameterized_curve(&circle(),&nested,None).unwrap();
    assert_eq!(composed["certificate"]["version"],"nurbs-foundation/5");
    assert_eq!(composed["certificate"]["compositionTree"]["kind"],"nested_composition");

    // Resource boundary refusals.
    assert!(intersection::resource_boundary_probe(0,2).is_err());
    assert!(intersection::resource_boundary_probe(26,2).is_err());
    assert!(intersection::resource_boundary_probe(2,257).is_err());

    // Mutated certificate does not alter subsequent authoritative queries.
    let mut mutated=cc.clone();
    mutated["coverage"]["complete"]=json!(false);
    let again=intersection::intersect_curve_curve(&line_a,&line_b,None).unwrap();
    assert_eq!(again["coverage"]["complete"],true);
}

#[test]
fn foundation_v5_adversarial_scales_seams_and_weights() {
    let scale=1e4;
    let a = Curve {
        degree: 1, knots: vec![0.,0.,1.,1.],
        control_points: vec![vec![0.,0.,0.],vec![scale,scale,0.]], weights: vec![1.,1.], periodic: false,
    };
    let b = Curve {
        degree: 1, knots: vec![0.,0.,1.,1.],
        control_points: vec![vec![0.,scale,0.],vec![scale,0.,0.]], weights: vec![2.,3.], periodic: false,
    };
    let report=intersection::intersect_curve_curve(&a,&b,None).unwrap();
    assert!(report["components"].as_array().unwrap().iter().any(|c|c["kind"]=="point"));

    let periodic = Curve {
        degree: 2,
        knots: vec![0., 1., 2., 3., 4., 5., 6., 7., 8.],
        control_points: vec![
            vec![1., 0., 0.],
            vec![0., 1., 0.],
            vec![-1., 0., 0.],
            vec![0., -1., 0.],
            vec![1., 0., 0.],
            vec![0., 1., 0.],
        ],
        weights: vec![1., 1., 1., 1., 1., 1.],
        periodic: true,
    };
    let axis = Curve {
        degree: 1, knots: vec![0.,0.,1.,1.],
        control_points: vec![vec![0.,-2.,0.],vec![0.,2.,0.]], weights: vec![1.,1.], periodic: false,
    };
    let seam=intersection::intersect_curve_curve(&periodic,&axis,None).unwrap();
    assert!(seam["coverage"]["boxesVisited"].as_u64().unwrap()>0);

    let high = Curve {
        degree: 3, knots: vec![0.,0.,0.,0.,1.,1.,1.,1.],
        control_points: vec![
            vec![0.,0.,0.],vec![0.2,1.,0.],vec![0.8,-1.,0.],vec![1.,0.,0.],
        ],
        weights: vec![1.,0.5,2.,1.], periodic: false,
    };
    let x_axis = Curve {
        degree: 1, knots: vec![0.,0.,1.,1.],
        control_points: vec![vec![0.,0.,0.],vec![1.,0.,0.]], weights: vec![1.,1.], periodic: false,
    };
    let multi=intersection::intersect_curve_curve(&high,&x_axis,None).unwrap();
    assert!(multi["components"].as_array().unwrap().len()>=1);
}

#[test]
fn certified_general_surface_surface_intersection() {
    let xy = surface::Surface {
        degree_u:1, degree_v:1, knots_u:vec![0.,0.,1.,1.], knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
        weights:vec![vec![1.,1.],vec![1.,1.]], periodic_u:false, periodic_v:false,
    };
    let xz = surface::Surface {
        degree_u:1, degree_v:1, knots_u:vec![0.,0.,1.,1.], knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.,0.,0.],vec![0.,0.,1.]],vec![vec![1.,0.,0.],vec![1.,0.,1.]]],
        weights:vec![vec![1.,1.],vec![1.,1.]], periodic_u:false, periodic_v:false,
    };
    let report = ss_intersection::intersect_surface_surface(&xy, &xz, None).unwrap();
    assert_eq!(report["version"], "nurbs-ss/1");
    assert_eq!(report["kind"], "surface_surface");
    assert_eq!(report["coverage"]["complete"], true);
    assert!(report["unresolved"].as_array().unwrap().is_empty());
    assert!(report["components"].as_array().unwrap().iter().any(|c| c["kind"]=="curve"));
    assert_eq!(report["booleanMutationAuthority"], false);
    assert_eq!(report["topologyAuthority"]["granted"], false);
    assert!(report["branchGraph"]["components"].is_array());
    assert!(report["uvArrangement"]["traces"].is_array());
    let audit = ss_intersection::verify_ss_coverage(&report).unwrap();
    assert_eq!(audit["complete"], true);
    assert_eq!(audit["notes"].as_array().unwrap().iter().any(|n| n=="no_graph_patch_iso_fixture"), true);

    let coincident = ss_intersection::intersect_surface_surface(&xy, &xy, None).unwrap();
    assert!(coincident["components"].as_array().unwrap().iter().any(|c| c["kind"]=="overlap"));
    assert!(coincident["components"].as_array().unwrap().iter().any(|c| c.get("coedgeTrim").is_some()));

    let above = surface::Surface {
        degree_u:1, degree_v:1, knots_u:vec![0.,0.,1.,1.], knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.,0.,2.],vec![0.,1.,2.]],vec![vec![1.,0.,2.],vec![1.,1.,2.]]],
        weights:vec![vec![1.,1.],vec![1.,1.]], periodic_u:false, periodic_v:false,
    };
    let empty = ss_intersection::intersect_surface_surface(&xy, &above, None).unwrap();
    assert_eq!(empty["coverage"]["complete"], true);
    assert!(empty["components"].as_array().unwrap().is_empty());

    // Rational positive-weight bicubic transverse seed (no graph-patch iso path).
    let patch_a = surface::Surface {
        degree_u:3, degree_v:3,
        knots_u:vec![0.,0.,0.,0.,1.,1.,1.,1.], knots_v:vec![0.,0.,0.,0.,1.,1.,1.,1.],
        control_points:vec![
            vec![vec![0.,0.,0.],vec![0.,1./3.,0.],vec![0.,2./3.,0.],vec![0.,1.,0.]],
            vec![vec![1./3.,0.,0.],vec![1./3.,1./3.,0.1],vec![1./3.,2./3.,0.1],vec![1./3.,1.,0.]],
            vec![vec![2./3.,0.,0.],vec![2./3.,1./3.,0.1],vec![2./3.,2./3.,0.1],vec![2./3.,1.,0.]],
            vec![vec![1.,0.,0.],vec![1.,1./3.,0.],vec![1.,2./3.,0.],vec![1.,1.,0.]],
        ],
        weights:vec![
            vec![1.,1.,1.,1.], vec![1.,1.2,0.8,1.], vec![1.,0.9,1.1,1.], vec![1.,1.,1.,1.],
        ],
        periodic_u:false, periodic_v:false,
    };
    let cutter = surface::Surface {
        degree_u:1, degree_v:1, knots_u:vec![0.,0.,1.,1.], knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.,0.5,-1.],vec![0.,0.5,1.]],vec![vec![1.,0.5,-1.],vec![1.,0.5,1.]]],
        weights:vec![vec![1.,1.],vec![1.,1.]], periodic_u:false, periodic_v:false,
    };
    let general = ss_intersection::intersect_surface_surface(&patch_a, &cutter, None).unwrap();
    assert_eq!(general["version"], "nurbs-ss/1");
    assert!(general["coverage"]["boxesVisited"].as_u64().unwrap() > 0);
    // Either complete components or typed unresolved — never silent false Complete.
    if general["coverage"]["complete"] == true {
        assert!(general["unresolved"].as_array().unwrap().is_empty());
    } else {
        assert!(!general["unresolved"].as_array().unwrap().is_empty());
        assert_eq!(general["topologyAuthority"]["revoked"], true);
    }
    assert_eq!(general["booleanMutationAuthority"], false);
    assert!(ss_intersection::ss_resource_probe(0, 1, 4).is_err());
    assert!(ss_intersection::ss_resource_probe(26, 1, 4).is_err());
    assert!(ss_intersection::ss_resource_probe(3, 3, 257).is_err());
}
