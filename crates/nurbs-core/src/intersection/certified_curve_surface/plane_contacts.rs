//! Scalar multiplicities on an original affine patch, including tangent roots.
use super::affine_plane::{dot, enclosure, q, subtract};
use super::*;
use num_rational::BigRational as Q;
use num_traits::{ToPrimitive, Zero};
#[derive(Debug)]
pub struct AffinePlaneContact {
    pub parameter_box: Box3,
    pub position_bounds: [[f64; 2]; 3],
    /// Original scalar coverage cells; multiplicities remain one-sided at knots.
    pub scalar_cells: Vec<usize>,
}
#[derive(Debug)]
pub struct AffinePlaneContacts {
    pub contacts: Vec<AffinePlaneContact>,
    pub unresolved: Vec<[f64; 2]>,
    /// Authoritative original-coefficient root proofs and scalar multiplicities.
    pub scalar: crate::curve_plane::Report,
    /// All curve parameters classified relative to this untrimmed affine patch.
    pub complete: bool,
}
fn exact_float(x: &Q) -> Result<f64> {
    let f = x
        .to_f64()
        .filter(|f| f.is_finite())
        .ok_or_else(|| crate::input("Affine plane coefficients are not finite binary64"))?;
    check(
        q(f) == *x,
        "Affine plane coefficients are not exactly binary64-representable",
    )?;
    Ok(f)
}
/// Isolate original curve contacts with a single exact affine parallelogram.
/// Even scalar multiplicities are admitted through exact polynomial proofs,
/// without claiming a transverse crossing sign. Boundary/ambiguous patch
/// membership and coincident intervals remain unresolved. No trim/Body proof.
pub fn affine_plane_contacts(
    c: &Curve,
    s: &Surface,
    tolerance: f64,
    max_cells: usize,
    max_exact_work: u64,
) -> Result<AffinePlaneContacts> {
    c.validate()?;
    s.validate()?;
    check(
        !c.periodic
            && !s.periodic_u
            && !s.periodic_v
            && c.control_points[0].len() == 3
            && s.control_points[0][0].len() == 3,
        "Affine contacts need nonperiodic 3D sources",
    )?;
    check(
        s.degree_u == 1
            && s.degree_v == 1
            && s.control_points.len() == 2
            && s.control_points[0].len() == 2
            && s.weights.iter().flatten().all(|w| *w == s.weights[0][0])
            && s.knots_u[0] == s.knots_u[1]
            && s.knots_u[2] == s.knots_u[3]
            && s.knots_v[0] == s.knots_v[1]
            && s.knots_v[2] == s.knots_v[3],
        "Target must be one clamped uniform-weight affine patch",
    )?;
    let pole = |i: usize, j: usize| std::array::from_fn(|k| q(s.control_points[i][j][k]));
    let origin = pole(0, 0);
    let u = subtract(&pole(1, 0), &origin);
    let v = subtract(&pole(0, 1), &origin);
    check(
        subtract(&pole(1, 1), &pole(1, 0)) == v,
        "Target is not an exact parallelogram",
    )?;
    let normal = [
        &u[1] * &v[2] - &u[2] * &v[1],
        &u[2] * &v[0] - &u[0] * &v[2],
        &u[0] * &v[1] - &u[1] * &v[0],
    ];
    let offset = dot(&normal, &origin);
    let nf = [
        exact_float(&normal[0])?,
        exact_float(&normal[1])?,
        exact_float(&normal[2])?,
    ];
    let scalar = crate::curve_plane::inspect_with_exact_budget(
        c,
        nf,
        exact_float(&offset)?,
        tolerance,
        max_cells,
        max_exact_work,
    )?;
    let (i, j, det) = [(0, 1), (0, 2), (1, 2)]
        .into_iter()
        .map(|(i, j)| (i, j, &u[i] * &v[j] - &u[j] * &v[i]))
        .find(|(_, _, d)| !d.is_zero())
        .ok_or_else(|| crate::input("Degenerate affine patch"))?;
    let bound =
        |x: &Q| enclosure(x).ok_or_else(|| crate::numeric_err("No affine coefficient enclosure"));
    let determinant = bound(&det)?;
    let oi = bound(&origin[i])?;
    let oj = bound(&origin[j])?;
    let ui = bound(&u[i])?;
    let uj = bound(&u[j])?;
    let vi = bound(&v[i])?;
    let vj = bound(&v[j])?;
    let mut unresolved: Vec<_> = scalar
        .cells
        .iter()
        .filter(|c| {
            matches!(
                c.classification,
                crate::curve_plane::Classification::Unresolved
                    | crate::curve_plane::Classification::Coincident
            )
        })
        .map(|c| c.domain)
        .collect();
    let mut contacts = vec![];
    for event in scalar.root_events() {
        let p = crate::interval_eval::evaluate_interval(
            c,
            I::new(event.parameter_bounds[0], event.parameter_bounds[1])?,
        )?;
        let di = p[i].sub(oi)?;
        let dj = p[j].sub(oj)?;
        let a = di.mul(vj)?.sub(dj.mul(vi)?)?.div_signed(determinant)?;
        let b = ui.mul(dj)?.sub(uj.mul(di)?)?.div_signed(determinant)?;
        if a.hi < 0. || a.lo > 1. || b.hi < 0. || b.lo > 1. {
            continue;
        }
        if a.lo <= 0. || a.hi >= 1. || b.lo <= 0. || b.hi >= 1. {
            unresolved.push(event.parameter_bounds);
            continue;
        }
        let natural =
            |x: I, k: &[f64]| I::point(k[1]).add(I::point(k[2]).sub(I::point(k[1]))?.mul(x)?);
        let ub = natural(a, &s.knots_u)?;
        let vb = natural(b, &s.knots_v)?;
        contacts.push(AffinePlaneContact {
            parameter_box: [event.parameter_bounds, [ub.lo, ub.hi], [vb.lo, vb.hi]],
            position_bounds: std::array::from_fn(|k| [p[k].lo, p[k].hi]),
            scalar_cells: event.cells,
        });
    }
    let complete = scalar.coverage_resolved && scalar.all_roots_isolated && unresolved.is_empty();
    Ok(AffinePlaneContacts {
        contacts,
        unresolved,
        scalar,
        complete,
    })
}
