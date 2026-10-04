//! Original-path tangent and projected-rail frame values for general guided sweep.
//! No contact-width fitting, retained error, arc-length or closed correction proof.
use super::*;

pub fn certify_path_guide_values(
    path: &Curve,
    guide: &Curve,
    twist: &Curve,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<ValuesReport> {
    if traversal[0] == traversal[1] {
        let derivative = if traversal[0] == 0. || traversal[0] == 1. {
            vector_certificate::certify_endpoint_first(path, traversal[0] == 1., max_cells)?
        } else {
            vector_certificate::certify_first_point(path, traversal[0], max_cells)?
        };
        let mut out = ValuesReport {
            status: Status::Unresolved,
            cells: derivative.cells,
            longitudinal: None,
            transverse: None,
            binormal: None,
        };
        let Some(tangent) = derivative.first else {
            return Ok(out);
        };
        let position = vector_certificate::certify_values_traversal(
            path,
            traversal,
            max_cells - out.cells,
            false,
        )?;
        out.cells += position.cells;
        let Some(position) = position.value else {
            return Ok(out);
        };
        let frame = certify_guide_values(
            guide,
            twist,
            traversal,
            tangent,
            position,
            max_cells - out.cells,
        )?;
        out.cells += frame.cells;
        if frame.status == Status::Certified {
            out.status = Status::Certified;
            out.longitudinal = frame.longitudinal;
            out.transverse = frame.transverse;
            out.binormal = frame.binormal;
        }
        return Ok(out);
    }
    let position = vector_certificate::certify_traversal(path, traversal, max_cells, false)?;
    let mut out = ValuesReport {
        status: Status::Unresolved,
        cells: position.cells,
        longitudinal: None,
        transverse: None,
        binormal: None,
    };
    if position.status != Status::Certified {
        return Ok(out);
    }
    // Path domain width is positive, so normalizing its original-parameter
    // derivative gives the same tangent as normalizing traversal velocity.
    let frame = certify_guide_values(
        guide,
        twist,
        traversal,
        position.first.unwrap(),
        position.value.unwrap(),
        max_cells - out.cells,
    )?;
    out.cells += frame.cells;
    if frame.status == Status::Certified {
        out.status = Status::Certified;
        out.longitudinal = frame.longitudinal;
        out.transverse = frame.transverse;
        out.binormal = frame.binormal;
    }
    Ok(out)
}

/// Original guided control image with explicit initial-coordinate premise.
/// No contact fitting, cap, closed correction or retained interpolation proof.
pub fn certify_path_guide_control_value(
    path: &Curve,
    guide: &Curve,
    scale: &Curve,
    twist: &Curve,
    affine: Option<(&Curve, &Curve)>,
    q: [[f64; 2]; 3],
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<ControlValueReport> {
    let frame = certify_path_guide_values(path, guide, twist, traversal, max_cells)?;
    super::trajectory::control_value_from_frame(path, scale, affine, q, traversal, max_cells, frame)
}

#[test]
fn original_path_guide_values_cover_twist_and_shared_budget() {
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let mut guide = crate::primitives::line([1., 0., 0.], [1., 0., 10.]).unwrap();
    guide.knots = vec![31., 31., 41., 41.];
    let mut twist = crate::primitives::line([0.; 3], [0.25, 0., 0.]).unwrap();
    twist.knots = vec![17., 17., 19., 19.];
    let report = certify_path_guide_values(&path, &guide, &twist, [0.25, 0.5], 1000).unwrap();
    assert_eq!(report.status, Status::Certified);
    let n = report.transverse.unwrap();
    let axis = report.longitudinal.unwrap();
    for t in [0.25_f64, 0.375, 0.5] {
        let expected = [(0.25 * t).cos(), (0.25 * t).sin(), 0.];
        for k in 0..3 {
            assert!(n[k][0] <= expected[k] && expected[k] <= n[k][1]);
        }
    }
    assert!(axis[2][0] <= 1. && axis[2][1] >= 1.);
    let exhausted =
        certify_path_guide_values(&path, &guide, &twist, [0.25, 0.5], report.cells - 1).unwrap();
    assert_eq!(exhausted.status, Status::Unresolved);
    assert!(exhausted.transverse.is_none() && exhausted.longitudinal.is_none());
    let parallel = crate::primitives::line([0., 0., 1.], [0., 0., 11.]).unwrap();
    let refused = certify_path_guide_values(&path, &parallel, &twist, [0.25, 0.5], 1000).unwrap();
    assert_eq!(refused.status, Status::Unresolved);
    assert!(refused.transverse.is_none());
}

#[test]
fn curved_original_path_tangent_is_enclosed_without_a_constant_polyline_premise() {
    let path = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 5., 5., 5.],
        control_points: vec![vec![0.; 3], vec![0.5, 0., 0.], vec![1., 0., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let mut guide = path.clone();
    guide.knots = vec![17., 17., 17., 19., 19., 19.];
    for p in &mut guide.control_points {
        p[0] += 1.;
    }
    let mut twist = crate::primitives::line([0.; 3], [1., 0., 0.]).unwrap();
    twist.control_points = vec![vec![0.; 3]; 2];
    let report = certify_path_guide_values(&path, &guide, &twist, [0.25, 0.5], 1000).unwrap();
    assert_eq!(report.status, Status::Certified);
    let axis = report.longitudinal.unwrap();
    let normal = report.transverse.unwrap();
    for t in [0.25_f64, 0.375, 0.5] {
        let length = (1. + 4. * t * t).sqrt();
        let expected_axis = [1. / length, 0., 2. * t / length];
        let expected_normal = [2. * t / length, 0., -1. / length];
        for k in 0..3 {
            assert!(axis[k][0] <= expected_axis[k] && expected_axis[k] <= axis[k][1]);
            assert!(normal[k][0] <= expected_normal[k] && expected_normal[k] <= normal[k][1]);
        }
    }
}

#[test]
fn guided_control_value_composes_uniform_affine_and_center_with_shared_work() {
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let mut guide = crate::primitives::line([1., 0., 0.], [1., 0., 10.]).unwrap();
    guide.knots = vec![31., 31., 41., 41.];
    let mut twist = crate::primitives::line([0.; 3], [0.25, 0., 0.]).unwrap();
    twist.knots = vec![17., 17., 19., 19.];
    let mut scale = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    scale.knots = vec![7., 7., 9., 9.];
    let mut axes = crate::primitives::line([1.; 3], [2., 1., 1.]).unwrap();
    axes.knots = vec![43., 43., 47., 47.];
    let mut center = crate::primitives::line([0.; 3], [0.5, 0., 0.]).unwrap();
    center.knots = vec![59., 59., 61., 61.];
    let q = [[2., 2.], [3., 3.], [4., 4.]];
    let report = certify_path_guide_control_value(
        &path,
        &guide,
        &scale,
        &twist,
        Some((&axes, &center)),
        q,
        [0.25, 0.5],
        1000,
    )
    .unwrap();
    assert_eq!(report.status, Status::Certified);
    let value = report.value.unwrap();
    for t in [0.25_f64, 0.375, 0.5] {
        let a = 2. * (1. + t).powi(2) + 0.5 * t;
        let b = 3. * (1. + t);
        let expected = [
            a * (0.25 * t).cos() - b * (0.25 * t).sin(),
            a * (0.25 * t).sin() + b * (0.25 * t).cos(),
            4. + 14. * t,
        ];
        for k in 0..3 {
            assert!(value[k][0] <= expected[k] && expected[k] <= value[k][1]);
        }
    }
    let refused = certify_path_guide_control_value(
        &path,
        &guide,
        &scale,
        &twist,
        Some((&axes, &center)),
        q,
        [0.25, 0.5],
        report.cells - 1,
    )
    .unwrap();
    assert_eq!(refused.status, Status::Unresolved);
    assert!(refused.value.is_none());
}

#[test]
fn guided_endpoints_use_original_rational_derivatives_and_share_all_work() {
    let mut path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    path.knots = vec![2., 2., 5., 5.];
    path.weights = vec![1., 2.];
    let mut guide = crate::primitives::line([1., 0., 0.], [1., 0., 10.]).unwrap();
    guide.knots = vec![31., 31., 41., 41.];
    let mut twist = crate::primitives::line([0.; 3], [0.25, 0., 0.]).unwrap();
    twist.knots = vec![17., 17., 19., 19.];
    for t in [0.0_f64, 1.] {
        let derivative = vector_certificate::certify_endpoint_first(&path, t == 1., 3).unwrap();
        assert_eq!(derivative.cells, 3);
        let expected_derivative = if t == 0. { 20. / 3. } else { 5. / 3. };
        let dz = derivative.first.unwrap()[2];
        assert!(dz[0] <= expected_derivative && expected_derivative <= dz[1]);
        let partial = vector_certificate::certify_endpoint_first(&path, t == 1., 2).unwrap();
        assert!(partial.first.is_none() && partial.cells == 2);
        let report = certify_path_guide_values(&path, &guide, &twist, [t, t], 100).unwrap();
        assert_eq!(report.status, Status::Certified);
        let n = report.transverse.unwrap();
        let expected = [(0.25 * t).cos(), (0.25 * t).sin(), 0.];
        for k in 0..3 {
            assert!(n[k][0] <= expected[k] && expected[k] <= n[k][1]);
        }
        let refused =
            certify_path_guide_values(&path, &guide, &twist, [t, t], report.cells - 1).unwrap();
        assert_eq!(refused.status, Status::Unresolved);
        assert!(refused.longitudinal.is_none() && refused.transverse.is_none());
    }
}

/// Guided second jets from the original path's third jets, not polyline tangents.
pub fn certify_path_guide(
    path: &Curve,
    guide: &Curve,
    twist: &Curve,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<Report> {
    let p = vector_certificate::certify_third_traversal(path, traversal, max_cells)?;
    let mut out = Report {
        status: Status::Unresolved,
        cells: p.base.cells,
        longitudinal: None,
        transverse: None,
        binormal: None,
        single_span: false,
        reason: p.base.reason,
    };
    if p.base.status != Status::Certified {
        return Ok(out);
    }
    out.reason = Some("guide-law-enclosure-unresolved");
    let g = vector_certificate::certify_traversal(guide, traversal, max_cells - out.cells, false)?;
    out.cells += g.cells;
    if g.status != Status::Certified {
        return Ok(out);
    }
    let position = jet(path, &p.base)?;
    let rail = jet(guide, &g)?;
    let [a, b] = path.domain();
    let width = I::point(b).sub(I::point(a))?;
    let jerk = mul(decode(p.third.unwrap())?, width.mul(width)?.mul(width)?)?;
    out.reason = Some("path-tangent-nonzero-unproved");
    let Some(t) = normalize(Jet {
        v: position.d,
        d: position.dd,
        dd: jerk,
    })?
    else {
        return Ok(out);
    };
    let offset = Jet {
        v: sub(rail.v, position.v)?,
        d: sub(rail.d, position.d)?,
        dd: sub(rail.dd, position.dd)?,
    };
    out.reason = Some("guide-transverse-direction-unproved");
    let Some(b) = normalize(cross_jet(t, offset)?)? else {
        return Ok(out);
    };
    let n = cross_jet(b, t)?;
    out.longitudinal = Some(encode(t));
    out.transverse = Some(encode(n));
    out.binormal = Some(encode(b));
    out.single_span = p.base.single_span && g.single_span;
    out.reason = None;
    out.status = Status::Certified;
    apply_twist(out, twist, traversal, max_cells)
}

#[test]
fn guided_curved_path_jets_enclose_analytic_tangent_derivatives() {
    let path = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 5., 5., 5.],
        control_points: vec![vec![0.; 3], vec![0.5, 0., 0.], vec![1., 0., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let mut guide = path.clone();
    guide.knots = vec![17., 17., 17., 19., 19., 19.];
    for p in &mut guide.control_points {
        p[0] += 1.;
    }
    let mut twist = crate::primitives::line([0.; 3], [1., 0., 0.]).unwrap();
    twist.control_points = vec![vec![0.; 3]; 2];
    let report = certify_path_guide(&path, &guide, &twist, [0.25, 0.5], 1000).unwrap();
    assert_eq!(report.status, Status::Certified);
    assert!(report.single_span);
    let axis = report.longitudinal.unwrap();
    for t in [0.25_f64, 0.375, 0.5] {
        let d = 1. + 4. * t * t;
        let expected = [
            [1. / d.sqrt(), 0., 2. * t / d.sqrt()],
            [-4. * t / d.powf(1.5), 0., 2. / d.powf(1.5)],
            [
                (-4. + 32. * t * t) / d.powf(2.5),
                0.,
                -24. * t / d.powf(2.5),
            ],
        ];
        for (range, values) in [axis.value, axis.first, axis.second]
            .into_iter()
            .zip(expected)
        {
            for k in 0..3 {
                assert!(range[k][0] <= values[k] && values[k] <= range[k][1]);
            }
        }
    }
    let refused = certify_path_guide(&path, &guide, &twist, [0.25, 0.5], report.cells - 1).unwrap();
    assert_eq!(refused.status, Status::Unresolved);
    assert!(refused.transverse.is_none());
}

/// Original guided trajectory jets; local coordinates remain an explicit premise.
pub fn certify_path_guide_control_trajectory(
    path: &Curve,
    guide: &Curve,
    scale: &Curve,
    twist: &Curve,
    affine: Option<(&Curve, &Curve)>,
    q: [[f64; 2]; 3],
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<TrajectoryReport> {
    let frame = certify_path_guide(path, guide, twist, traversal, max_cells)?;
    super::trajectory::control_trajectory_from_frame(
        path, scale, affine, q, traversal, max_cells, frame,
    )
}

#[test]
fn guided_control_jets_include_joint_laws_and_tight_linear_remainder() {
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let mut guide = crate::primitives::line([1., 0., 0.], [1., 0., 10.]).unwrap();
    guide.knots = vec![31., 31., 41., 41.];
    let mut scale = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    scale.knots = vec![7., 7., 9., 9.];
    let mut twist = path.clone();
    twist.control_points = vec![vec![0.; 3]; 2];
    let axes = crate::primitives::line([1.; 3], [2., 1., 1.]).unwrap();
    let center = crate::primitives::line([0.; 3], [0.5, 0., 0.]).unwrap();
    let q = [[2., 2.], [3., 3.], [4., 4.]];
    let report = certify_path_guide_control_trajectory(
        &path,
        &guide,
        &scale,
        &twist,
        Some((&axes, &center)),
        q,
        [0.25, 0.5],
        1000,
    )
    .unwrap();
    assert_eq!(report.status, Status::Certified);
    assert!(report.single_span);
    let jet = report.jet.as_ref().unwrap();
    for t in [0.25_f64, 0.375, 0.5] {
        let expected = [
            [2. + 4.5 * t + 2. * t * t, 3. + 3. * t, 4. + 14. * t],
            [4.5 + 4. * t, 3., 14.],
            [4., 0., 0.],
        ];
        for (range, values) in [jet.value, jet.first, jet.second].into_iter().zip(expected) {
            for k in 0..3 {
                assert!(range[k][0] <= values[k] && values[k] <= range[k][1]);
            }
        }
    }
    let remainder = report.linear_remainder_upper().unwrap().unwrap();
    assert!(remainder >= 0.03125 && remainder < 0.03125 + 1e-9);
    let refused = certify_path_guide_control_trajectory(
        &path,
        &guide,
        &scale,
        &twist,
        Some((&axes, &center)),
        q,
        [0.25, 0.5],
        report.cells - 1,
    )
    .unwrap();
    assert_eq!(refused.status, Status::Unresolved);
    assert!(refused.jet.is_none());
}
