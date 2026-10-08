//! Source-owned initial principal phase extended through planar inflections.
//! Callers must first charge and prove the exact original plane premise.
//! This is an ideal reference; actual constructor rounding and phase choice
//! remain covered by retained endpoint displacement, never sampled admission.
use super::*;

pub(crate) fn certify_initial_planar_principal_phase(
    path: &Curve,
    normal: [f64; 3],
    max_cells: usize,
) -> Result<(usize, Option<f64>)> {
    let zero = crate::sweeps::progressive_sweep::constant_vector_law([0.; 3])?;
    let principal = certify_frenet_path_values(path, &zero, [0., 0.], max_cells)?;
    let mut cells = principal.cells;
    if principal.status != Status::Certified {
        // A regular clamped rational Bezier may have zero initial curvature.
        // Its first off-tangent coefficient owns the initial principal limit,
        // independently of station count or the constructor's sampled search.
        let tangent = fixed_normal_path::fixed_normal_values_at(
            path,
            normal,
            &zero,
            [0., 0.],
            [0., 0.],
            max_cells - cells,
            true,
        )?;
        cells += tangent.cells;
        if tangent.status != Status::Certified {
            return Ok((cells, None));
        }
        let (work, phase) = initial_bezier_phase(path, normal, (max_cells - cells) as u64)?;
        return Ok((cells + work as usize, phase));
    }
    let canonical = fixed_normal_path::fixed_normal_values_at(
        path,
        normal,
        &zero,
        [0., 0.],
        [0., 0.],
        max_cells - cells,
        true,
    )?;
    cells += canonical.cells;
    if canonical.status != Status::Certified {
        return Ok((cells, None));
    }
    let product = dot(
        decode(principal.transverse.unwrap())?,
        decode(canonical.binormal.unwrap())?,
    )?;
    let sign = if product.lo > 0. {
        Some(1.)
    } else if product.hi < 0. {
        Some(-1.)
    } else {
        None
    };
    Ok((cells, sign))
}

fn initial_bezier_phase(
    path: &Curve,
    normal: [f64; 3],
    max_work: u64,
) -> Result<(u64, Option<f64>)> {
    use cad_predicates::{
        AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
    };
    let p = path.degree;
    let [a, b] = path.domain();
    if max_work == 0
        || p < 2
        || path.periodic
        || path.control_points.len() != p + 1
        || path.knots[..=p].iter().any(|u| *u != a)
        || path.knots[p + 1..].iter().any(|u| *u != b)
        || path.control_points.len() > 32768
    {
        return Ok((0, None));
    }
    let Some(axis) = (0..3).find(|&k| normal[k] != 0.) else {
        return Ok((0, None));
    };
    let coordinates = [(axis + 1) % 3, (axis + 2) % 3];
    let values = path
        .control_points
        .iter()
        .flat_map(|point| coordinates.map(|k| AuthoredScalar::Binary64Bits(point[k].to_bits())))
        .collect();
    let Ok(source) = SourceArena::authored("original-bezier-leading-principal-phase", 1, values)
    else {
        return Ok((0, None));
    };
    let refs = |i| [source.leaf(2 * i).unwrap(), source.leaf(2 * i + 1).unwrap()];
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work,
            ..Limits::default()
        },
        None,
    );
    // The caller proved every original pole lies in the plane normal to B.
    // Thus cross(P1-P0,Pk-P0) is parallel to B; one cyclic projected
    // determinant establishes its exact sign, with no rounded differences.
    // If preceding poles are collinear, the first transverse homogeneous
    // Bezier term is binomial(p,k)*wk*Pk*t^k. Positive weights and nonzero
    // initial velocity make the leading curvature multiplier positive.
    for k in 2..=p {
        let decision = cad_predicates::orient2d(&mut ctx, refs(0), refs(1), refs(k));
        let phase = match decision.map(|d| d.outcome) {
            Ok(Outcome::Sign(Sign::Zero)) => continue,
            Ok(Outcome::Sign(Sign::Positive)) => Some(-normal[axis].signum()),
            Ok(Outcome::Sign(Sign::Negative)) => Some(normal[axis].signum()),
            _ => None,
        };
        return Ok((ctx.work_used(), phase));
    }
    Ok((ctx.work_used(), None))
}

fn signed(values: [[f64; 2]; 3], sign: f64) -> [[f64; 2]; 3] {
    if sign > 0. {
        values
    } else {
        values.map(|v| [-v[1], -v[0]])
    }
}

#[test]
fn leading_coefficient_predicates_preserve_exact_zero_prefix_and_shared_limit() {
    let origin = 1e16;
    let mut path = Curve {
        degree: 4,
        knots: [vec![2.; 5], vec![5.; 5]].concat(),
        control_points: vec![
            vec![origin, origin, 0.],
            vec![origin + 2., origin + 2., 0.],
            vec![origin + 4., origin + 4., 0.],
            vec![origin + 6., origin + 8., 0.],
            vec![origin + 8., origin + 16., 0.],
        ],
        weights: vec![1., 2., 0.75, 1.25, 1.],
        periodic: false,
    };
    let (work, phase) = initial_bezier_phase(&path, [0., 0., 1.], 100000).unwrap();
    assert_eq!(phase, Some(-1.));
    assert!(work > 0);
    assert_eq!(
        initial_bezier_phase(&path, [0., 0., -3.], 100000)
            .unwrap()
            .1,
        Some(1.)
    );
    assert_eq!(
        initial_bezier_phase(&path, [0., 0., 1.], 0).unwrap(),
        (0, None)
    );
    let short = initial_bezier_phase(&path, [0., 0., 1.], work - 1).unwrap();
    assert!(short.0 <= work - 1);
    assert_eq!(short.1, None);
    path.control_points[3][1] = origin + 4.;
    assert_eq!(
        initial_bezier_phase(&path, [0., 0., 1.], 100000).unwrap().1,
        Some(1.)
    );
    for pole in &mut path.control_points {
        pole[1] = pole[0];
    }
    assert_eq!(
        initial_bezier_phase(&path, [0., 0., 1.], 100000).unwrap().1,
        None
    );
}
fn phase_values(mut frame: ValuesReport, sign: f64) -> Result<ValuesReport> {
    check(sign == 1. || sign == -1., "Invalid planar principal phase")?;
    if frame.status == Status::Certified {
        let n = frame.transverse.unwrap();
        let b = frame.binormal.unwrap();
        frame.transverse = Some(signed(b, sign));
        frame.binormal = Some(signed(n, -sign));
    }
    Ok(frame)
}
pub(crate) fn certify_planar_principal_values(
    path: &Curve,
    normal: [f64; 3],
    sign: f64,
    twist: &Curve,
    path_traversal: [f64; 2],
    law_traversal: [f64; 2],
    max_cells: usize,
) -> Result<ValuesReport> {
    // A constant quarter-turn commutes with the authored twist. Exact swaps
    // and sign changes avoid approximate pi/2 or fitted angle premises.
    phase_values(
        fixed_normal_path::fixed_normal_values_at(
            path,
            normal,
            twist,
            path_traversal,
            law_traversal,
            max_cells,
            true,
        )?,
        sign,
    )
}
pub(crate) fn certify_planar_principal_control_values(
    path: &Curve,
    normal: [f64; 3],
    sign: f64,
    scale: &Curve,
    twist: &Curve,
    affine: Option<(&Curve, &Curve)>,
    qs: &[[[f64; 2]; 3]],
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<ControlValuesReport> {
    let frame = certify_planar_principal_values(
        path, normal, sign, twist, traversal, traversal, max_cells,
    )?;
    super::trajectory::control_values_with_fit(
        path, scale, affine, qs, traversal, max_cells, frame, None,
    )
}
pub(crate) fn certify_planar_principal_relative_values(
    path: &Curve,
    normal: [f64; 3],
    sign: f64,
    scale: &Curve,
    twist: &Curve,
    affine: Option<(&Curve, &Curve)>,
    qs: &[[[f64; 2]; 3]],
    path_traversal: [f64; 2],
    law_traversal: [f64; 2],
    max_cells: usize,
) -> Result<ControlValuesReport> {
    let frame = certify_planar_principal_values(
        path,
        normal,
        sign,
        twist,
        path_traversal,
        law_traversal,
        max_cells,
    )?;
    let zero = crate::sweeps::progressive_sweep::constant_vector_law([0.; 3])?;
    super::trajectory::control_values_with_fit(
        &zero,
        scale,
        affine,
        qs,
        law_traversal,
        max_cells,
        frame,
        None,
    )
}
pub(crate) fn certify_planar_principal_control_trajectories(
    path: &Curve,
    normal: [f64; 3],
    sign: f64,
    scale: &Curve,
    twist: &Curve,
    affine: Option<(&Curve, &Curve)>,
    qs: &[[[f64; 2]; 3]],
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<TrajectoriesReport> {
    check(sign == 1. || sign == -1., "Invalid planar principal phase")?;
    let mut frame = certify_fixed_normal_path(path, normal, twist, traversal, max_cells)?;
    if frame.status == Status::Certified {
        let signed_jet = |mut j: FrameJet, s: f64| {
            j.value = signed(j.value, s);
            j.first = signed(j.first, s);
            j.second = signed(j.second, s);
            j
        };
        let n = frame.transverse.take().unwrap();
        let b = frame.binormal.take().unwrap();
        frame.transverse = Some(signed_jet(b, sign));
        frame.binormal = Some(signed_jet(n, -sign));
    }
    super::trajectory::control_trajectories_with_fit(
        path, scale, affine, qs, traversal, max_cells, frame, None,
    )
}
