//! Rank obstructions for original closed sources with one fixed axis.
use super::exact::{R, add, mul, span_jet, sub};
use super::*;
use crate::core::bounded_rational::Work;
use num_rational::BigRational as Q;
use num_traits::Zero;

/// For a common carrier moved by a constant linear map and a closed
/// translation, choose any interior UV point of a source span. Its two
/// carrier derivatives are constant in station. Their cross product is
/// either zero, or a fixed vector whose dot product with the translation
/// is a differentiable scalar with equal endpoints. Rolle's theorem puts
/// a zero of the volume determinant in the interior. Arc mode must already
/// have a positive-speed inverse clock, so differentiation in station
/// preserves the obstruction.
fn translation_laws_constant(s: &Sweep<'_>, out: &mut IdealVolumeReport, max_cells: usize) -> bool {
    for law in [s.scale, s.twist].into_iter().chain(
        s.affine_laws
            .into_iter()
            .flat_map(|(axis, center)| [axis, center]),
    ) {
        let charge = law.control_points.len();
        if charge > max_cells - out.cells {
            return false;
        }
        out.cells += charge;
        if !law
            .control_points
            .iter()
            .all(|p| p == &law.control_points[0])
        {
            return false;
        }
    }
    true
}

fn transverse_carrier(path: &Curve, cap: &Surface, work: &mut Work) -> R<bool> {
    let p = path.degree;
    if !(1..=32).contains(&p) {
        return Ok(false);
    }
    let mut span = p;
    while path.knots[span] == path.knots[span + 1] {
        work.charge(1)?;
        span += 1;
        if span == path.control_points.len() {
            return Ok(false);
        }
    }
    let first = span - p;
    let knots = path.knots[first..=span + p + 1]
        .iter()
        .map(|&k| work.lift(k))
        .collect::<R<Vec<_>>>()?;
    let mut rows = Vec::new();
    for i in first..=span {
        let weight = work.lift(path.weights[i])?;
        let mut row = Vec::new();
        for &value in &path.control_points[i] {
            let value = work.lift(value)?;
            row.push(mul(work, &value, &weight)?);
        }
        row.push(weight);
        rows.push(row);
    }
    let (h, d) = span_jet(p, &knots, &rows, &knots[p], p, work, true)?;
    // This is P' times the strictly positive squared denominator.
    let numer = (0..3)
        .map(|axis| {
            let first = mul(work, &d[axis], &h[3])?;
            let second = mul(work, &h[axis], &d[3])?;
            sub(work, &first, &second)
        })
        .collect::<R<Vec<_>>>()?;
    if numer.iter().all(Zero::is_zero) {
        return Ok(false);
    }
    let origin = cap.control_points[0][0]
        .iter()
        .map(|&x| work.lift(x))
        .collect::<R<Vec<_>>>()?;
    for point in cap.control_points.iter().flatten() {
        let mut dot = Q::zero();
        for axis in 0..3 {
            let x = work.lift(point[axis])?;
            let delta = sub(work, &x, &origin[axis])?;
            let term = mul(work, &numer[axis], &delta)?;
            dot = add(work, &dot, &term)?;
        }
        if !dot.is_zero() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn interior_c1(curve: &Curve, out: &mut IdealVolumeReport, max_work: u64) -> Result<bool> {
    use super::super::endpoint_jets;
    let basis = endpoint_jets::certify_basis_knots(curve, 1, max_work - out.exact_work)?;
    out.exact_work += basis.exact_work;
    if basis.certified {
        return Ok(true);
    }
    let exact = endpoint_jets::certify_knots(curve, 1, max_work - out.exact_work)?;
    out.exact_work += exact.exact_work;
    Ok(exact.certified)
}

pub(super) fn obstruction(
    s: &Sweep<'_>,
    cap: &Surface,
    out: &mut IdealVolumeReport,
    max_cells: usize,
    max_work: u64,
) -> Result<Option<&'static str>> {
    if s.options.orientation != Orientation::Fixed
        || s.frame_laws.is_some()
        || s.orientation_guide.is_some()
        || !interior_c1(s.path, out, max_work)?
    {
        return Ok(None);
    }
    if translation_laws_constant(s, out, max_cells) {
        return Ok(Some("ideal-volume-fixed-translation-rank-loss"));
    }
    if out.cells == max_cells {
        return Ok(None);
    }
    // With a fixed longitudinal axis and a transverse carrier, the axial
    // coordinate of the whole section is independent of UV, even with
    // varying scale, axis and center. Twist leaves that coordinate intact.
    // Exact closure and C1 through the original joins then force its
    // station derivative to vanish at an interior extremum.
    let mut work = Work {
        used: 0,
        limit: (max_work - out.exact_work).min(100000),
    };
    let transverse = transverse_carrier(s.path, cap, &mut work).unwrap_or(false);
    out.exact_work += work.used;
    if !transverse {
        return Ok(None);
    }
    for law in std::iter::once(s.scale).chain(
        s.affine_laws
            .into_iter()
            .flat_map(|(axis, center)| [axis, center]),
    ) {
        if !interior_c1(law, out, max_work)? {
            return Ok(None);
        }
    }
    Ok(Some("ideal-volume-fixed-transverse-rank-loss"))
}
