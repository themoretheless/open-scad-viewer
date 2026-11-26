//! Exact dyadic subdivision of the squared curvature-gap threshold residual.
use super::{
    curvature_gap_threshold::{CurvatureGapComparison, compare_net, threshold_residual},
    umbilic_subdivision::split,
    *,
};

#[derive(Clone, Debug, PartialEq)]
pub struct CurvatureGapRegion {
    pub original_domain: [[f64; 2]; 2],
    /// Exact dyadic coordinates in the original cell's unit square.
    pub unit_domain: [[f64; 2]; 2],
    pub comparison: CurvatureGapComparison,
}

#[derive(Clone, Debug)]
pub struct RefinedCurvatureGapAudit {
    pub regions: Option<Vec<CurvatureGapRegion>>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}

/// Compare each original cell against a positive squared-gap threshold,
/// bisecting ambiguous cells in alternating U and V directions. Leaves
/// exactly cover the original cell. At the depth cap, an ambiguous leaf
/// retains either a corner witness or an unresolved comparison. High-degree
/// residuals are returned unresolved. Work or regularity refusal is atomic.
pub fn refine_curvature_gap(
    source: &Surface,
    level: f64,
    max_depth: u8,
    max_work: u64,
) -> Result<RefinedCurvatureGapAudit> {
    source.validate()?;
    check(
        level.is_finite() && level > 0.,
        "Curvature gap level must be finite and positive",
    )?;
    check(
        max_depth <= 8,
        "Curvature gap subdivision depth must be in [0,8]",
    )?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Curvature gap subdivision work must be in [1,1000000]",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let (regions, refusal) = match audit(source, level, max_depth, &mut work) {
        Ok(regions) => (Some(regions), None),
        Err(reason) => (None, Some(reason)),
    };
    Ok(RefinedCurvatureGapAudit {
        regions,
        refusal,
        work_used: work.used,
    })
}

fn audit(
    source: &Surface,
    level: f64,
    max_depth: u8,
    work: &mut Work,
) -> PResult<Vec<CurvatureGapRegion>> {
    use CurvatureGapComparison::*;
    let level = work.lift(level)?;
    let mut regions = Vec::new();
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
            let original_domain = [u, v];
            let unit_domain = [[0., 1.]; 2];
            let residual = threshold_residual(source, ku, kv, &level, work)?;
            let (pu, pv) = storage::degrees(std::slice::from_ref(&residual));
            if residual.is_empty() || pu > 25 || pv > 25 {
                let comparison = if residual.is_empty() {
                    EqualEverywhere
                } else {
                    Unresolved
                };
                regions.push(CurvatureGapRegion {
                    original_domain,
                    unit_domain,
                    comparison,
                });
                continue;
            }
            let size = [pu + 1, pv + 1];
            let mut pending = vec![(unit_domain, storage::bernstein(&residual, pu, pv, work)?, 0)];
            while let Some((unit_domain, net, depth)) = pending.pop() {
                let comparison = compare_net(&net, size);
                if depth == max_depth || matches!(comparison, AtLeast | AtMost) {
                    regions.push(CurvatureGapRegion {
                        original_domain,
                        unit_domain,
                        comparison,
                    });
                    continue;
                }
                let axis = usize::from(depth % 2);
                let [left, right] = split(&net, size, axis, work)?;
                let mut lower = unit_domain;
                let mut upper = unit_domain;
                let [a, b] = unit_domain[axis];
                let middle = (a + b) * 0.5;
                lower[axis][1] = middle;
                upper[axis][0] = middle;
                pending.push((upper, right, depth + 1));
                pending.push((lower, left, depth + 1));
            }
        }
    }
    Ok(regions)
}
