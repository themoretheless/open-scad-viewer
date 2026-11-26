//! Exact dyadic refinement of an original principal-curvature gap polynomial.
use super::{umbilic::UmbilicStatus, *};
use num_traits::Signed;

#[derive(Clone, Debug, PartialEq)]
pub struct UmbilicRegion {
    pub original_domain: [[f64; 2]; 2],
    /// Exact dyadic coordinates in the unit square of the original cell.
    /// Apply the original cell's affine map to recover absolute parameters.
    pub unit_domain: [[f64; 2]; 2],
    pub status: UmbilicStatus,
}

#[derive(Clone, Debug)]
pub struct RefinedUmbilicAudit {
    pub regions: Option<Vec<UmbilicRegion>>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}

/// Subdivide cells whose strict curvature separation remains unresolved.
/// Every returned region belongs to an exact cover in its own original chart.
/// A depth limit leaves unproved regions as `ContainsNonUmbilicPoint`.
/// An unproved source normal or exhausted work ledger refuses the full audit.
pub fn refine_umbilics(
    source: &Surface,
    max_depth: u8,
    max_work: u64,
) -> Result<RefinedUmbilicAudit> {
    source.validate()?;
    check(max_depth <= 8, "Umbilic subdivision depth must be in [0,8]")?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Umbilic subdivision work must be in [1,1000000]",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = audit(source, max_depth, &mut work);
    let (regions, refusal) = match result {
        Ok(regions) => (Some(regions), None),
        Err(reason) => (None, Some(reason)),
    };
    Ok(RefinedUmbilicAudit {
        regions,
        refusal,
        work_used: work.used,
    })
}

fn audit(source: &Surface, max_depth: u8, work: &mut Work) -> PResult<Vec<UmbilicRegion>> {
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
            let target =
                original_target(source, ku, kv, Field::SquaredPrincipalCurvatureGap, work)?;
            let numerator = &target[0];
            let (pu, pv) = storage::degrees(std::slice::from_ref(numerator));
            let original_domain = [u, v];
            let unit_domain = [[0., 1.]; 2];
            let initial = if numerator.is_empty() {
                Some(UmbilicStatus::EverywhereUmbilic)
            } else if pu > 25 || pv > 25 {
                Some(UmbilicStatus::ContainsNonUmbilicPoint)
            } else {
                None
            };
            if let Some(status) = initial {
                regions.push(UmbilicRegion {
                    original_domain,
                    unit_domain,
                    status,
                });
                continue;
            }
            let net = storage::bernstein(numerator, pu, pv, work)?;
            let mut pending = vec![(unit_domain, net, 0)];
            while let Some((unit_domain, net, depth)) = pending.pop() {
                let status = if net.iter().all(Signed::is_positive) {
                    UmbilicStatus::EverywhereSeparated
                } else if depth == max_depth {
                    UmbilicStatus::ContainsNonUmbilicPoint
                } else {
                    let axis = usize::from(depth % 2);
                    let [left, right] = split(&net, [pu + 1, pv + 1], axis, work)?;
                    let mut lower = unit_domain;
                    let mut upper = unit_domain;
                    let [a, b] = unit_domain[axis];
                    let middle = (a + b) * 0.5;
                    lower[axis][1] = middle;
                    upper[axis][0] = middle;
                    pending.push((upper, right, depth + 1));
                    pending.push((lower, left, depth + 1));
                    continue;
                };
                regions.push(UmbilicRegion {
                    original_domain,
                    unit_domain,
                    status,
                });
            }
        }
    }
    Ok(regions)
}

pub(super) fn split(
    net: &[Q],
    size: [usize; 2],
    axis: usize,
    work: &mut Work,
) -> PResult<[Vec<Q>; 2]> {
    let [nu, nv] = size;
    let len = size[axis];
    let lines = size[1 - axis];
    let index = |line, i| {
        if axis == 0 {
            i * nv + line
        } else {
            line * nv + i
        }
    };
    let mut left = vec![Q::zero(); nu * nv];
    let mut right = left.clone();
    for line in 0..lines {
        let mut row: Vec<_> = (0..len).map(|i| net[index(line, i)].clone()).collect();
        left[index(line, 0)] = row[0].clone();
        right[index(line, len - 1)] = row[len - 1].clone();
        for level in 1..len {
            for i in 0..len - level {
                row[i] = work.op(&row[i], &row[i + 1], |a, b| {
                    (a + b) / Q::from_integer(2.into())
                })?;
            }
            left[index(line, level)] = row[0].clone();
            right[index(line, len - level - 1)] = row[len - level - 1].clone();
        }
    }
    Ok([left, right])
}
