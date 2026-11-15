//! Continuous range bounds for the original principal-curvature gap.
use super::*;
use num_traits::{Signed, ToPrimitive};

#[derive(Clone, Debug, PartialEq)]
pub struct CurvatureGapRangeCell {
    pub domain: [[f64; 2]; 2],
    /// Outward bounds in inverse length squared. None means the degree cap,
    /// denominator hull or numeric storage prevented a finite enclosure.
    pub bounds: Option<[f64; 2]>,
}

#[derive(Clone, Debug)]
pub struct CurvatureGapRanges {
    pub cells: Option<Vec<CurvatureGapRangeCell>>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}

/// Bound the squared difference of principal curvatures on each complete
/// original UV cell. Positive rational Bernstein weights make the value
/// a convex combination of coefficient ratios. No field storage or samples
/// are required. Each cell describes its own one-sided knot traces.
pub fn curvature_gap_ranges(source: &Surface, max_work: u64) -> Result<CurvatureGapRanges> {
    source.validate()?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Curvature gap range work must be in [1,1000000]",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = audit(source, &mut work);
    let (cells, refusal) = match result {
        Ok(cells) => (Some(cells), None),
        Err(reason) => (None, Some(reason)),
    };
    Ok(CurvatureGapRanges {
        cells,
        refusal,
        work_used: work.used,
    })
}

fn audit(source: &Surface, work: &mut Work) -> PResult<Vec<CurvatureGapRangeCell>> {
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
            let target =
                original_target(source, ku, kv, Field::SquaredPrincipalCurvatureGap, work)?;
            let target = polynomial::constant_field(&target, work)?.unwrap_or(target);
            let bounds = bound(&target, work)?;
            cells.push(CurvatureGapRangeCell {
                domain: [u, v],
                bounds,
            });
        }
    }
    Ok(cells)
}

fn bound(target: &[Poly], work: &mut Work) -> PResult<Option<[f64; 2]>> {
    let (pu, pv) = storage::degrees(target);
    if pu > 25 || pv > 25 {
        return Ok(None);
    }
    let denominator = storage::bernstein(&target[3], pu, pv, work)?;
    if !denominator.iter().all(Signed::is_positive) {
        return Ok(None);
    }
    let numerator = storage::bernstein(&target[0], pu, pv, work)?;
    let mut lower = None::<Q>;
    let mut upper = Q::zero();
    for (n, d) in numerator.iter().zip(&denominator) {
        let ratio = work.op(n, d, |a, b| a / b)?;
        if lower.as_ref().is_none_or(|old| ratio < *old) {
            lower = Some(ratio.clone());
        }
        upper = upper.max(ratio);
    }
    // Eigenvalues of the regular symmetric shape form are real, so the
    // squared gap is nonnegative even if its control hull crosses zero.
    let lower = lower.unwrap_or_else(Q::zero).max(Q::zero());
    let Some(lo) = outward(&lower, false, work)? else {
        return Ok(None);
    };
    let Some(hi) = outward(&upper, true, work)? else {
        return Ok(None);
    };
    Ok(Some([lo, hi]))
}

pub(super) fn outward(value: &Q, upper: bool, work: &mut Work) -> PResult<Option<f64>> {
    let Some(mut approx) = value.to_f64().filter(|x| x.is_finite()) else {
        return Ok(None);
    };
    let admits = |bound: &Q| {
        if upper {
            bound >= value
        } else {
            bound <= value
        }
    };
    if admits(&work.lift(approx)?) {
        return Ok(Some(approx));
    }
    approx = if upper {
        approx.next_up()
    } else {
        approx.next_down()
    };
    if !approx.is_finite() || !admits(&work.lift(approx)?) {
        return Ok(None);
    }
    Ok(Some(approx))
}
