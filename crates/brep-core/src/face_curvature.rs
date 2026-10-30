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
// A floating Jacobi frame is only a proposal. The actual bound below uses
// interval Q^T A Q and a certified positive lower bound for Q^T Q, never
// assumes floating rotations/eigenvalues to be exact or orthonormal.
fn spectral_bounds(t: [[[f64; 2]; 3]; 3]) -> Result<Option<[f64; 2]>> {
    if t.iter().flatten().flatten().any(|x| !x.is_finite()) {
        return Ok(None);
    }
    let mut a = [[I::point(0.); 3]; 3];
    let mut m = [[0.; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            let lo = t[i][j][0].max(t[j][i][0]);
            let hi = t[i][j][1].min(t[j][i][1]);
            if lo > hi {
                return Ok(None);
            }
            a[i][j] = I::new(lo, hi)?;
            m[i][j] = lo * 0.5 + hi * 0.5;
        }
    }
    let mut q = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    for _ in 0..24 {
        let (p, r) = [(0, 1), (0, 2), (1, 2)]
            .into_iter()
            .max_by(|&(i, j), &(k, l)| m[i][j].abs().total_cmp(&m[k][l].abs()))
            .unwrap();
        if !m[p][r].is_finite() || m[p][r] == 0. {
            break;
        }
        let angle = 0.5 * (2. * m[p][r]).atan2(m[r][r] - m[p][p]);
        let (s, c) = angle.sin_cos();
        if !s.is_finite() || !c.is_finite() {
            break;
        }
        for row in &mut m {
            let (x, y) = (row[p], row[r]);
            row[p] = c * x - s * y;
            row[r] = s * x + c * y;
        }
        for j in 0..3 {
            let (x, y) = (m[p][j], m[r][j]);
            m[p][j] = c * x - s * y;
            m[r][j] = s * x + c * y;
        }
        for row in &mut q {
            let (x, y) = (row[p], row[r]);
            row[p] = c * x - s * y;
            row[r] = s * x + c * y;
        }
    }
    let mut gram = [[I::point(0.); 3]; 3];
    let mut b = gram;
    for k in 0..3 {
        for l in 0..3 {
            for i in 0..3 {
                gram[k][l] = gram[k][l].add(I::point(q[i][k]).mul(I::point(q[i][l]))?)?;
                for j in 0..3 {
                    b[k][l] =
                        b[k][l].add(I::point(q[i][k]).mul(a[i][j])?.mul(I::point(q[j][l]))?)?;
                }
            }
        }
    }
    let mut gram_lower = f64::INFINITY;
    let mut row_upper = 0f64;
    let mut lower = 0f64;
    for i in 0..3 {
        let mut off = I::point(0.);
        let mut row = I::point(0.);
        for j in 0..3 {
            if j != i {
                off = off.add(I::point(abs_upper(gram[i][j])))?;
            }
            row = row.add(I::point(abs_upper(b[i][j])))?;
        }
        gram_lower = gram_lower.min(I::point(gram[i][i].lo).sub(off)?.lo);
        row_upper = row_upper.max(row.hi);
        if gram[i][i].lo <= 0. {
            return Ok(None);
        }
        lower = lower.max(abs_lower(b[i][i].div(gram[i][i])?));
    }
    if gram_lower <= 0. || !row_upper.is_finite() {
        return Ok(None);
    }
    let upper = I::point(row_upper).div(I::point(gram_lower))?.hi;
    Ok(upper.is_finite().then_some([lower, upper]))
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
) -> Result<Option<[f64; 2]>> {
    let mut query = r;
    for k in 0..2 {
        if out.translation_invariant_axes[k] {
            let mid = r[k][0] * 0.5 + r[k][1] * 0.5;
            query[k] = [mid, mid];
        }
    }
    let shape = surface_shape_operator::rectangle_bounds(
        &model.faces[face].surface,
        query,
        limit.saturating_sub(out.shape_cells).min(100000),
    )?;
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
    if let Some(b) = shape_bound(model, face, r, out, limits.shape_cells)? {
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
        if probe(model, face, &domain, uv, maximum, limits, &mut out)? {
            return Ok(out);
        }
    }
    for (degree, knots, n) in [
        (s.degree_u, &s.knots_u, s.control_points.len()),
        (s.degree_v, &s.knots_v, s.control_points[0].len()),
    ] {
        if knots.iter().any(|&k| {
            k > knots[degree]
                && k < knots[n]
                && knots.iter().filter(|&&x| x == k).count() + 2 > degree
        }) {
            out.reason = "continuity_not_proven";
            out.unresolved_region = Some(natural);
            return Ok(out);
        }
    }
    let mut stack = vec![natural];
    let mut maximum_upper = 0f64;
    while let Some(r) = stack.pop() {
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
        let bound = shape_bound(model, face, r, &mut out, limits.shape_cells)?;
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
        if probe(model, face, &domain, middle, maximum, limits, &mut out)? {
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
        stack.push(b);
        stack.push(a);
    }
    out.decision = Decision::Within;
    out.unresolved_region = None;
    out.unresolved_curvature = None;
    out.absolute_curvature_upper = Some(maximum_upper);
    out.reason = "complete_interval_cover";
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
