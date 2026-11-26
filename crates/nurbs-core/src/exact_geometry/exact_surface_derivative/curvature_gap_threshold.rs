//! Threshold certificates for the original squared principal curvature gap.
use super::*;
use num_traits::Signed;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurvatureGapComparison {
    /// The squared gap meets or exceeds the level everywhere on the cell.
    AtLeast,
    /// The squared gap meets or falls below the level everywhere on the cell.
    AtMost,
    EqualEverywhere,
    /// At least one corner is strictly above and another strictly below.
    CornerStraddle,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CurvatureGapCell {
    pub domain: [[f64; 2]; 2],
    pub comparison: CurvatureGapComparison,
}

#[derive(Clone, Debug)]
pub struct CurvatureGapAudit {
    pub cells: Option<Vec<CurvatureGapCell>>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}

/// Compare the original squared curvature gap with a finite positive level.
/// The input level is in inverse length squared and its stored bits are
/// interpreted exactly. This comparison does not round or store a field.
/// Results describe one-sided traces on each original UV cell.
pub fn compare_curvature_gap(
    source: &Surface,
    level: f64,
    max_work: u64,
) -> Result<CurvatureGapAudit> {
    source.validate()?;
    check(
        level.is_finite() && level > 0.,
        "Curvature gap level must be finite and positive",
    )?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Curvature gap comparison work must be in [1,1000000]",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = audit(source, level, &mut work);
    let (cells, refusal) = match result {
        Ok(cells) => (Some(cells), None),
        Err(reason) => (None, Some(reason)),
    };
    Ok(CurvatureGapAudit {
        cells,
        refusal,
        work_used: work.used,
    })
}

fn audit(source: &Surface, level: f64, work: &mut Work) -> PResult<Vec<CurvatureGapCell>> {
    let level = work.lift(level)?;
    let mut cells = Vec::new();
    for ku in source.degree_u..source.control_points.len() {
        let u = [source.knots_u[ku], source.knots_u[ku + 1]];
        if u[0] == u[1] {
            continue;
        }
        for kv in source.degree_v..source.control_points[0].len() {
            let v = [source.knots_v[kv], source.knots_v[kv + 1]];
            if v[0] == v[1] {
                continue;
            }
            let residual = threshold_residual(source, ku, kv, &level, work)?;
            let comparison = compare(&residual, work)?;
            cells.push(CurvatureGapCell {
                domain: [u, v],
                comparison,
            });
        }
    }
    Ok(cells)
}

pub(super) fn threshold_residual(
    source: &Surface,
    ku: usize,
    kv: usize,
    level: &Q,
    work: &mut Work,
) -> PResult<Poly> {
    let target = original_target(source, ku, kv, Field::SquaredPrincipalCurvatureGap, work)?;
    // The original denominator is positive. An exact constant identity
    // replaces that denominator with one before the degree check.
    let target = polynomial::constant_field(&target, work)?.unwrap_or(target);
    let [numerator, _, _, mut denominator]: [Poly; 4] =
        target.try_into().map_err(|_| Refusal::UnprovedIdentity)?;
    for coefficient in denominator.values_mut() {
        *coefficient = work.op(coefficient, level, |a, b| a * b)?;
    }
    polynomial::subtract(numerator, &denominator, work)
}

fn compare(residual: &Poly, work: &mut Work) -> PResult<CurvatureGapComparison> {
    use CurvatureGapComparison::*;
    if residual.is_empty() {
        return Ok(EqualEverywhere);
    }
    let (pu, pv) = storage::degrees(std::slice::from_ref(residual));
    if pu > 25 || pv > 25 {
        return Ok(Unresolved);
    }
    let net = storage::bernstein(residual, pu, pv, work)?;
    Ok(compare_net(&net, [pu + 1, pv + 1]))
}

pub(super) fn compare_net(net: &[Q], [nu, nv]: [usize; 2]) -> CurvatureGapComparison {
    use CurvatureGapComparison::*;
    if net.iter().all(|value| !value.is_negative()) {
        return AtLeast;
    }
    if net.iter().all(|value| !value.is_positive()) {
        return AtMost;
    }
    let corners = [
        &net[0],
        &net[nv - 1],
        &net[(nu - 1) * nv],
        &net[net.len() - 1],
    ];
    if corners.iter().any(|v| v.is_positive()) && corners.iter().any(|v| v.is_negative()) {
        CornerStraddle
    } else {
        Unresolved
    }
}
