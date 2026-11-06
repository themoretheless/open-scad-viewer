//! Principal-curvature bounds on the authored trimmed support. No radius law,
//! material, embedding or whole-solid admission is inferred from this report.
use crate::{Model, face_domain::FaceDomain};
use nurbs_core::{
    Error, Result, interval_eval::Interval as I, surface_shape_operator, trim_domain::Location,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Within,
    Exceeds,
    Unresolved,
}
#[derive(Clone, Copy)]
pub struct Limits {
    pub cells: usize,
    pub domain_cells: usize,
    pub shape_cells: usize,
}
pub struct Report {
    pub decision: Decision,
    pub cells: usize,
    pub domain_cells: usize,
    pub shape_cells: usize,
    pub covered_cells: usize,
    pub omitted_cells: usize,
    pub absolute_curvature_upper: Option<f64>,
    pub translation_invariant_axes: [bool; 2],
    pub unresolved_region: Option<[[f64; 2]; 2]>,
    pub unresolved_curvature: Option<[f64; 2]>,
    pub witness_uv: Option<[f64; 2]>,
    pub witness_region: Option<[[f64; 2]; 2]>,
    pub witness_curvature: Option<[f64; 2]>,
    pub reason: &'static str,
}
fn check(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::new("BREP_FACE_CURVATURE_INPUT", message))
    }
}
fn abs_lower(x: I) -> f64 {
    if x.lo <= 0. && x.hi >= 0. {
        0.
    } else {
        x.lo.abs().min(x.hi.abs())
    }
}
fn abs_upper(x: I) -> f64 {
    x.lo.abs().max(x.hi.abs())
}
fn spectral_bounds(t: [[[f64; 2]; 3]; 3]) -> Result<Option<[f64; 2]>> {
    surface_shape_operator::absolute_spectral_bounds(t)
}
// With two degree-one layers, row-constant weights and exactly identical
// Cartesian displacements, S(u,v)=C(u)+lambda(v)*D (or the transposed case).
// All first/second geometric jets, hence the WORLD curvature tensor, are
// independent of that translation parameter. Binary-rational equality is
// required; a close-looking extrusion is never enough.
fn translation_axes(s: &nurbs_core::surface::Surface) -> [bool; 2] {
    use num_rational::BigRational as Q;
    let q = |v: f64| Q::from_float(v).expect("validated finite pole");
    let mut axes = [false; 2];
    if s.degree_v == 1 && s.control_points[0].len() == 2 {
        let delta: Vec<_> = (0..3)
            .map(|k| q(s.control_points[0][1][k]) - q(s.control_points[0][0][k]))
            .collect();
        axes[1] = (0..s.control_points.len()).all(|i| {
            s.weights[i][0] == s.weights[i][1]
                && (0..3).all(|k| {
                    q(s.control_points[i][1][k]) - q(s.control_points[i][0][k]) == delta[k]
                })
        });
    }
    if s.degree_u == 1 && s.control_points.len() == 2 {
        let delta: Vec<_> = (0..3)
            .map(|k| q(s.control_points[1][0][k]) - q(s.control_points[0][0][k]))
            .collect();
        axes[0] = (0..s.control_points[0].len()).all(|j| {
            s.weights[0][j] == s.weights[1][j]
                && (0..3).all(|k| {
                    q(s.control_points[1][j][k]) - q(s.control_points[0][j][k]) == delta[k]
                })
        });
    }
    axes
}
fn shape_bound(
    model: &Model,
    face: usize,
    r: [[f64; 2]; 2],
    out: &mut Report,
    limit: usize,
    continuity: Option<&nurbs_core::analysis::surface_continuity::Certificate<'_>>,
    span: Option<[usize; 2]>,
) -> Result<Option<[f64; 2]>> {
    let mut query = r;
    for k in 0..2 {
        if out.translation_invariant_axes[k] {
            let mid = r[k][0] * 0.5 + r[k][1] * 0.5;
            query[k] = [mid, mid];
        }
    }
    let budget = limit.saturating_sub(out.shape_cells).min(100000);
    let support = &model.faces[face].surface;
    let shape = if let Some(span) = span {
        surface_shape_operator::rectangle_span_bounds(support, span, query, budget)?
    } else if let Some(proof) = continuity {
        surface_shape_operator::rectangle_bounds_with_continuity(support, query, budget, proof)?
    } else {
        surface_shape_operator::rectangle_bounds(support, query, budget)?
    };
    out.shape_cells += shape.cells;
    let Some(tensor) = shape.shape.tensor else {
        return Ok(None);
    };
    if shape.shape.normal.is_none() {
        return Ok(None);
    }
    // An unresolved arithmetic enclosure cannot become a positive receipt.
    match spectral_bounds(tensor) {
        Ok(b) => Ok(b),
        Err(_) => Ok(None),
    }
}
fn probe(
    model: &Model,
    face: usize,
    domain: &FaceDomain,
    uv: [f64; 2],
    maximum: f64,
    limits: Limits,
    out: &mut Report,
    continuity: Option<&nurbs_core::analysis::surface_continuity::Certificate<'_>>,
    span: Option<[usize; 2]>,
) -> Result<bool> {
    let budget = limits
        .domain_cells
        .saturating_sub(out.domain_cells)
        .min(100000);
    if budget == 0 {
        return Ok(false);
    }
    let r = uv.map(|x| [x, x]);
    let membership = domain.classify(r, budget)?;
    out.domain_cells += membership.cells;
    if membership.location != Location::Inside {
        return Ok(false);
    }
    if let Some(b) = shape_bound(model, face, r, out, limits.shape_cells, continuity, span)? {
        if b[0] > maximum {
            out.decision = Decision::Exceeds;
            out.unresolved_region = None;
            out.unresolved_curvature = None;
            out.witness_uv = Some(uv);
            out.witness_region = Some(r);
            out.witness_curvature = Some(b);
            out.reason = "certified_inside_witness";
            return Ok(true);
        }
    }
    Ok(false)
}
/// Cover the complete authored trimmed face using fresh original-support
/// world tensors. Hints can REJECT only after an Inside classification and
/// certified curvature lower bound. Only a complete interval cover admits.
/// Principal magnitudes use the spectral norm of the symmetric world tensor;
/// no rank-two assumption, sampled maximum or Frobenius substitute is used.
/// The declared maximum has units /mm; a radius law remains a separate test.
pub fn inspect(
    model: &Model,
    face: usize,
    maximum: f64,
    tolerance_uv: f64,
    limits: Limits,
    hints: &[[f64; 2]],
) -> Result<Report> {
    inspect_impl(model, face, maximum, tolerance_uv, limits, hints, None)
}
/// An exact C2 certificate may replace the conservative knot-multiplicity gate.
/// It must borrow this same original face support; a caller flag, copied report,
/// C1 proof or certificate for an equal-looking surface cannot authorize it.
pub fn inspect_with_continuity(
    model: &Model,
    face: usize,
    maximum: f64,
    tolerance_uv: f64,
    limits: Limits,
    hints: &[[f64; 2]],
    continuity: &nurbs_core::analysis::surface_continuity::Certificate<'_>,
) -> Result<Report> {
    let support = model
        .faces
        .get(face)
        .ok_or_else(|| Error::new("BREP_FACE_CURVATURE_INPUT", "Curvature face is missing"))?;
    check(
        continuity.order() >= 2 && std::ptr::eq(continuity.source(), &support.surface),
        "Curvature continuity must certify this original support to order two",
    )?;
    inspect_impl(
        model,
        face,
        maximum,
        tolerance_uv,
        limits,
        hints,
        Some(continuity),
    )
}
fn inspect_impl(
    model: &Model,
    face: usize,
    maximum: f64,
    tolerance_uv: f64,
    limits: Limits,
    hints: &[[f64; 2]],
    continuity: Option<&nurbs_core::analysis::surface_continuity::Certificate<'_>>,
) -> Result<Report> {
    inspect_mode(
        model,
        face,
        maximum,
        tolerance_uv,
        limits,
        hints,
        continuity,
        false,
    )
}
/// Complete one-sided original-span curvature coverage. This deliberately
/// carries no chart-join G2 or pole-limit certificate; callers must combine
/// those obligations independently before any whole-body admission.
pub struct OneSidedReport {
    pub field: Report,
    pub original_spans: usize,
}
pub fn inspect_one_sided(
    model: &Model,
    face: usize,
    maximum: f64,
    tolerance_uv: f64,
    limits: Limits,
) -> Result<OneSidedReport> {
    let field = inspect_mode(model, face, maximum, tolerance_uv, limits, &[], None, true)?;
    let s = &model.faces[face].surface;
    let count = |p: usize, k: &[f64], n: usize| (p..n).filter(|&i| k[i] < k[i + 1]).count();
    Ok(OneSidedReport {
        field,
        original_spans: count(s.degree_u, &s.knots_u, s.control_points.len())
            * count(s.degree_v, &s.knots_v, s.control_points[0].len()),
    })
}
fn inspect_mode(
    model: &Model,
    face: usize,
    maximum: f64,
    tolerance_uv: f64,
    limits: Limits,
    hints: &[[f64; 2]],
    continuity: Option<&nurbs_core::analysis::surface_continuity::Certificate<'_>>,
    one_sided: bool,
) -> Result<Report> {
    check(
        maximum.is_finite()
            && maximum > 0.
            && limits.cells <= 50000
            && limits.domain_cells <= 1_000_000
            && limits.shape_cells <= 1_000_000
            && hints.len() <= 64,
        "Curvature inspection needs a positive finite maximum and bounded work",
    )?;
    let domain = FaceDomain::new(model, face, tolerance_uv)?;
    let s = &model.faces[face].surface;
    check(
        !s.periodic_u
            && !s.periodic_v
            && s.degree_u <= 12
            && s.degree_v <= 12
            && s.control_points
                .len()
                .saturating_mul(s.control_points[0].len())
                <= 4096,
        "Curvature inspection needs a bounded nonperiodic support",
    )?;
    let natural = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    check(
        hints.iter().all(|p| {
            (0..2).all(|k| p[k].is_finite() && p[k] >= natural[k][0] && p[k] <= natural[k][1])
        }),
        "Curvature hints must lie in the natural support",
    )?;
    for wire in
        std::iter::once(model.faces[face].outer).chain(model.faces[face].holes.iter().copied())
    {
        let curves = model.loops[wire]
            .coedges
            .iter()
            .map(|c| c.pcurve.clone())
            .collect::<Vec<_>>();
        check(
            nurbs_core::trim_domain::exact_loop_joins(&curves)? == Some(true),
            "Curvature coverage needs exact closed authored UV loops",
        )?;
        check(
            curves
                .iter()
                .all(|c| c.weights.iter().all(|w| w.is_finite() && *w > 0.)),
            "UV control hull coverage requires positive rational weights",
        )?;
        check(
            curves.iter().all(|c| {
                c.control_points.iter().all(|p| {
                    p.len() == 2 && (0..2).all(|k| p[k] >= natural[k][0] && p[k] <= natural[k][1])
                })
            }),
            "Authored UV loop containment in natural support is not proven",
        )?;
    }
    let mut out = Report {
        decision: Decision::Unresolved,
        cells: 0,
        domain_cells: 0,
        shape_cells: 0,
        covered_cells: 0,
        omitted_cells: 0,
        absolute_curvature_upper: None,
        translation_invariant_axes: translation_axes(s),
        unresolved_region: None,
        unresolved_curvature: None,
        witness_uv: None,
        witness_region: None,
        witness_curvature: None,
        reason: "work_limit",
    };
    for &uv in hints {
        if probe(
            model, face, &domain, uv, maximum, limits, &mut out, continuity, None,
        )? {
            return Ok(out);
        }
    }
    for (degree, knots, n) in [
        (s.degree_u, &s.knots_u, s.control_points.len()),
        (s.degree_v, &s.knots_v, s.control_points[0].len()),
    ] {
        if !one_sided
            && continuity.is_none()
            && knots.iter().any(|&k| {
                k > knots[degree]
                    && k < knots[n]
                    && knots.iter().filter(|&&x| x == k).count() + 2 > degree
            })
        {
            out.reason = "continuity_not_proven";
            out.unresolved_region = Some(natural);
            return Ok(out);
        }
    }
    let mut stack = Vec::new();
    if one_sided {
        for u in s.degree_u..s.control_points.len() {
            for v in s.degree_v..s.control_points[0].len() {
                let r = [
                    [s.knots_u[u], s.knots_u[u + 1]],
                    [s.knots_v[v], s.knots_v[v + 1]],
                ];
                if r.iter().all(|d| d[0] < d[1]) {
                    stack.push((r, Some([u, v])));
                }
            }
        }
        stack.reverse();
    } else {
        stack.push((natural, None));
    }
    let mut maximum_upper = 0f64;
    while let Some((r, span)) = stack.pop() {
        out.unresolved_region = Some(r);
        out.unresolved_curvature = None;
        if out.cells == limits.cells
            || out.shape_cells == limits.shape_cells
            || out.domain_cells == limits.domain_cells
        {
            return Ok(out);
        }
        out.cells += 1;
        let membership = domain.classify(
            r,
            limits
                .domain_cells
                .saturating_sub(out.domain_cells)
                .min(100000),
        )?;
        out.domain_cells += membership.cells;
        if membership.location == Location::Outside {
            out.omitted_cells += 1;
            continue;
        }
        let bound = shape_bound(
            model,
            face,
            r,
            &mut out,
            limits.shape_cells,
            continuity,
            span,
        )?;
        out.unresolved_curvature = bound;
        if let Some(b) = bound {
            if b[1] <= maximum {
                out.covered_cells += 1;
                maximum_upper = maximum_upper.max(b[1]);
                continue;
            }
            if b[0] > maximum && membership.location == Location::Inside {
                out.decision = Decision::Exceeds;
                out.unresolved_region = None;
                out.unresolved_curvature = None;
                out.witness_region = Some(r);
                out.witness_uv = Some(r.map(|d| d[0] * 0.5 + d[1] * 0.5));
                out.witness_curvature = Some(b);
                out.reason = "certified_inside_region";
                return Ok(out);
            }
        }
        let middle = r.map(|d| d[0] * 0.5 + d[1] * 0.5);
        if probe(
            model, face, &domain, middle, maximum, limits, &mut out, continuity, span,
        )? {
            return Ok(out);
        }
        if out.translation_invariant_axes == [true, true] {
            out.reason = "invariant_curvature_unproven";
            return Ok(out);
        }
        let widths = std::array::from_fn::<_, 2, _>(|k| {
            if out.translation_invariant_axes[k] {
                -1.
            } else {
                (r[k][1] - r[k][0]) / (natural[k][1] - natural[k][0])
            }
        });
        let axis = usize::from(widths[1] > widths[0]);
        if middle[axis] == r[axis][0] || middle[axis] == r[axis][1] {
            out.reason = "precision_limit";
            return Ok(out);
        }
        let mut a = r;
        let mut b = r;
        a[axis][1] = middle[axis];
        b[axis][0] = middle[axis];
        stack.push((b, span));
        stack.push((a, span));
    }
    out.decision = Decision::Within;
    out.unresolved_region = None;
    out.unresolved_curvature = None;
    out.absolute_curvature_upper = Some(maximum_upper);
    out.reason = if one_sided {
        "complete_one_sided_interval_cover"
    } else {
        "complete_interval_cover"
    };
    Ok(out)
}
#[cfg(test)]
mod spectral_tests {
    use super::*;
    #[test]
    fn proposed_eigenframe_is_checked_including_nonzero_third_eigenvalue() {
        for matrix in [
            [[5.5, 4.5, 0.], [4.5, 5.5, 0.], [0., 0., 0.]],
            [[1., 0., 0.], [0., 1., 0.], [0., 0., -2.]],
        ] {
            let expected = if matrix[0][0] > 2. { 10. } else { 2. };
            let b = spectral_bounds(matrix.map(|r| r.map(|x| [x, x])))
                .unwrap()
                .unwrap();
            assert!(
                b[0] <= expected && b[1] >= expected && b[1] - b[0] < 1e-10,
                "{b:?}"
            );
        }
    }
    #[test]
    fn interval_uncertainty_is_not_replaced_by_midpoint_eigenvalues() {
        let matrix = [
            [[0.9, 1.1], [0., 0.], [0., 0.]],
            [[0., 0.], [1.9, 2.1], [0., 0.]],
            [[0., 0.], [0., 0.], [-0.1, 0.1]],
        ];
        let b = spectral_bounds(matrix).unwrap().unwrap();
        assert!(b[0] <= 1.9 && b[1] >= 2.1);
    }
}
