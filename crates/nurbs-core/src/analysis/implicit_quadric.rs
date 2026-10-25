//! Exact represented patch membership and regular implicit support. This does
//! NOT prove the patch is two-dimensional, injective, a valid pole quotient,
//! a complete vertex fan, or a Solid. Those remain native owner obligations.
use crate::{Result, check, distance_bounds::Interval as I, surface::Surface};
use cad_predicates::{
    AuthoredScalar, BezierIdentity, Limits, PredicateContext, SourceArena, ToleranceContext,
};
#[derive(Clone, Debug)]
pub struct Report {
    pub exact_membership: bool,
    pub gradient_nonzero: bool,
    pub regular_implicit_support: bool,
    pub exact_work: u64,
    pub cells: usize,
    pub gradient_lower: Option<f64>,
    pub reason: Option<&'static str>,
}
fn bezier(s: &Surface) -> bool {
    let (p, q) = (s.degree_u, s.degree_v);
    let (n, m) = (s.control_points.len(), s.control_points[0].len());
    !s.periodic_u
        && !s.periodic_v
        && p + 1 == n
        && q + 1 == m
        && s.knots_u[..n].iter().all(|x| *x == s.knots_u[p])
        && s.knots_u[n..].iter().all(|x| *x == s.knots_u[n])
        && s.knots_v[..m].iter().all(|x| *x == s.knots_v[q])
        && s.knots_v[m..].iter().all(|x| *x == s.knots_v[m])
}
/// Q is the literal symmetric matrix in [x,y,z,1]^T Q [x,y,z,1]=0.
/// A positive result supplies a UNIQUE smooth ambient implicit surface over the
/// entire represented patch image, including parameter poles. Native callers
/// must independently prove patch coverage/quotients/vertex incidence before
/// transferring that implicit surface's tangent plane and second fundamental
/// form to an actual/original body boundary. No caller proof is accepted.
pub fn inspect(
    s: &Surface,
    q: [[f64; 4]; 4],
    max_exact_work: u64,
    max_cells: usize,
) -> Result<Report> {
    inspect_with_constant_products(s, q, &[], max_exact_work, max_cells)
}
/// Same support theorem with original constant products retained exactly.
pub fn inspect_with_constant_products(
    s: &Surface,
    q: [[f64; 4]; 4],
    products: &[[f64; 2]],
    max_exact_work: u64,
    max_cells: usize,
) -> Result<Report> {
    s.validate()?;
    check(
        (1..=8).contains(&s.degree_u)
            && (1..=8).contains(&s.degree_v)
            && s.control_points
                .iter()
                .flatten()
                .all(|p| p.len() == 3 && p.iter().all(|x| x.abs() <= 1e9))
            && s.weights.iter().flatten().all(|x| *x <= 1e9)
            && q.iter().flatten().all(|x| x.is_finite() && x.abs() <= 1e9)
            && (0..4).all(|i| (0..4).all(|j| q[i][j] == q[j][i]))
            && products.len() <= 16
            && products
                .iter()
                .flatten()
                .all(|x| x.is_finite() && x.abs() <= 1e9)
            && max_exact_work <= 16000000
            && max_cells <= 100000,
        "Invalid implicit quadric source or limits",
    )?;
    let mut out = Report {
        exact_membership: false,
        gradient_nonzero: false,
        regular_implicit_support: false,
        exact_work: 0,
        cells: 0,
        gradient_lower: None,
        reason: Some("implicit-support-unproved"),
    };
    if !bezier(s) {
        out.reason = Some("implicit-single-bezier-required");
        return Ok(out);
    }
    let mut scalars = Vec::new();
    for (i, row) in s.control_points.iter().enumerate() {
        for (j, p) in row.iter().enumerate() {
            scalars.extend(p.iter().copied());
            scalars.push(s.weights[i][j]);
        }
    }
    let offset = scalars.len();
    scalars.extend(q.iter().flatten());
    scalars.extend(products.iter().flatten());
    let source = SourceArena::authored(
        "original-implicit-quadric-support",
        1,
        scalars
            .into_iter()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .map_err(|e| crate::input(e.to_string()))?;
    let nv = s.control_points[0].len();
    let surface = (0..s.control_points.len())
        .map(|i| {
            (0..nv)
                .map(|j| std::array::from_fn(|k| source.leaf((i * nv + j) * 4 + k).unwrap()))
                .collect()
        })
        .collect::<Vec<Vec<_>>>();
    let matrix =
        std::array::from_fn(|i| std::array::from_fn(|j| source.leaf(offset + 4 * i + j).unwrap()));
    let factors = (0..products.len())
        .map(|i| std::array::from_fn(|j| source.leaf(offset + 16 + 2 * i + j).unwrap()))
        .collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    let mut context = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work: max_exact_work.min(cad_predicates::MAX_WORK),
            ..Limits::default()
        },
        None,
    );
    let decision = cad_predicates::binary_bezier_quadric_with_constant_products_identity(
        &mut context,
        &surface,
        matrix,
        &factors,
    )
    .map_err(|e| crate::input(e.to_string()))?;
    out.exact_work = decision.work_used;
    if decision.outcome != BezierIdentity::Equal {
        out.reason = Some("exact-implicit-membership-unproved");
        return Ok(out);
    }
    out.exact_membership = true;
    if q[..3].iter().flatten().all(|x| *x == 0.) {
        out.reason = Some("implicit-gradient-identically-zero");
        return Ok(out);
    }
    let du = [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]];
    let dv = [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]];
    let mut pending = vec![[du, dv]];
    let mut lower = f64::INFINITY;
    while let Some(b) = pending.pop() {
        if max_cells - out.cells < 2 {
            out.reason = Some("implicit-gradient-work-limit");
            return Ok(out);
        }
        out.cells += 2;
        let value = crate::interval_eval::evaluate_surface_interval(
            s,
            I::new(b[0][0], b[0][1])?,
            I::new(b[1][0], b[1][1])?,
        )?;
        let mut margin: f64 = 0.;
        for k in 0..3 {
            let mut g = I::point(q[k][3]);
            for j in 0..3 {
                g = g.add(I::point(q[k][j]).mul(value[j])?)?;
            }
            g = g.mul(I::point(2.))?;
            margin = margin.max(if g.lo > 0. {
                g.lo
            } else if g.hi < 0. {
                -g.hi
            } else {
                0.
            });
        }
        if margin > 0. {
            lower = lower.min(margin);
            continue;
        }
        let axis = usize::from(
            (b[1][1] - b[1][0]) / (dv[1] - dv[0]) > (b[0][1] - b[0][0]) / (du[1] - du[0]),
        );
        let mid = crate::distance_bounds::midpoint(b[axis][0], b[axis][1]);
        if mid <= b[axis][0] || mid >= b[axis][1] {
            out.reason = Some("implicit-gradient-precision-limit");
            return Ok(out);
        }
        let mut a = b;
        let mut c = b;
        a[axis][1] = mid;
        c[axis][0] = mid;
        pending.extend([c, a]);
    }
    out.gradient_nonzero = true;
    out.regular_implicit_support = true;
    out.gradient_lower = Some(lower);
    out.reason = None;
    Ok(out)
}
#[cfg(test)]
mod tests;

/// Exact original sphere equation, without rounded center^2 or radius^2.
/// This still supplies regular implicit support only, not pole/fan/Solid proof.
pub fn inspect_sphere(
    s: &Surface,
    center: [f64; 3],
    radius: f64,
    max_exact_work: u64,
    max_cells: usize,
) -> Result<Report> {
    check(
        radius.is_finite()
            && radius > 0.
            && radius <= 1e9
            && center.iter().all(|x| x.is_finite() && x.abs() <= 1e9),
        "Invalid original implicit sphere inputs",
    )?;
    let mut q = [[0.; 4]; 4];
    for k in 0..3 {
        q[k][k] = 1.;
        q[k][3] = -center[k];
        q[3][k] = -center[k];
    }
    let products = [
        [center[0], center[0]],
        [center[1], center[1]],
        [center[2], center[2]],
        [-radius, radius],
    ];
    inspect_with_constant_products(s, q, &products, max_exact_work, max_cells)
}
