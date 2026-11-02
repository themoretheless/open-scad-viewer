//! Continuous same-basis candidate verification. This proves pointwise normal
//! displacement only, not injectivity, closest thickness, or solid topology.
use crate::{Result, check, distance_bounds::Interval as I, surface::Surface};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateStatus {
    Within,
    Exceeds,
    Unresolved,
}
#[derive(Clone, Debug)]
pub struct CandidateCertificate {
    pub status: CandidateStatus,
    /// Present only after every rectangle of the compared carrier domain was covered.
    pub error_upper: Option<f64>,
    pub cells: usize,
}

/// Bound `candidate(u,v) - source(u,v) - distance * unit_normal(source)`
/// everywhere on the original domain. Positive, identical rational bases are
/// required; pole-subtraction roundoff is included using interval arithmetic.
/// Every cell is charged, including failed normal enclosures. A finite work
/// limit never turns an unresolved normal or tolerance into acceptance.
pub fn certify_candidate(
    source: &Surface,
    candidate: &Surface,
    distance: f64,
    tolerance: f64,
    max_cells: usize,
) -> Result<CandidateCertificate> {
    source.validate()?;
    candidate.validate()?;
    check(
        source
            .control_points
            .iter()
            .flatten()
            .chain(candidate.control_points.iter().flatten())
            .all(|p| p.len() == 3),
        "Offset certificate requires 3D poles",
    )?;
    check(
        distance.is_finite() && tolerance.is_finite() && tolerance > 0.,
        "Invalid offset certificate distance/tolerance",
    )?;
    check(
        max_cells > 0 && max_cells <= 1_000_000,
        "Invalid offset certificate budget",
    )?;
    check(
        source.degree_u == candidate.degree_u
            && source.degree_v == candidate.degree_v
            && source.knots_u == candidate.knots_u
            && source.knots_v == candidate.knots_v
            && source.weights == candidate.weights
            && source.periodic_u == candidate.periodic_u
            && source.periodic_v == candidate.periodic_v
            && source.control_points.len() == candidate.control_points.len()
            && source.control_points[0].len() == candidate.control_points[0].len(),
        "Offset certificate requires identical rational bases",
    )?;
    check(
        source.weights.iter().flatten().all(|w| *w > 0.),
        "Offset certificate requires positive weights",
    )?;
    let mut delta = source.clone();
    let mut rounding = [0_f64; 3];
    for (i, row) in delta.control_points.iter_mut().enumerate() {
        for (j, pole) in row.iter_mut().enumerate() {
            for k in 0..3 {
                let exact = I::point(candidate.control_points[i][j][k])
                    .sub(I::point(source.control_points[i][j][k]))?;
                pole[k] = candidate.control_points[i][j][k] - source.control_points[i][j][k];
                let error = exact.sub(I::point(pole[k]))?;
                rounding[k] = rounding[k].max(error.lo.abs().max(error.hi.abs()));
            }
        }
    }
    let domain = [
        [
            source.knots_u[source.degree_u],
            source.knots_u[source.control_points.len()],
        ],
        [
            source.knots_v[source.degree_v],
            source.knots_v[source.control_points[0].len()],
        ],
    ];
    let mut pending = std::collections::VecDeque::from([(domain, 0usize)]);
    let mut cells = 0;
    let mut upper = 0_f64;
    while let Some((rect, depth)) = pending.pop_front() {
        if cells == max_cells {
            return Ok(CandidateCertificate {
                status: CandidateStatus::Unresolved,
                error_upper: None,
                cells,
            });
        }
        cells += 1;
        let normals = super::bounds(source, rect, 1., 4096)?.unit_normals;
        if let Some(normals) = normals {
            let displacement = crate::surface_distance::rectangle_bounds(&delta, rect)?;
            let mut bound = 0_f64;
            let mut lower = 0_f64;
            for k in 0..3 {
                let d = I::new(displacement[k][0], displacement[k][1])?
                    .add(I::new(-rounding[k], rounding[k])?)?;
                let residual =
                    d.sub(I::new(normals[k][0], normals[k][1])?.mul(I::point(distance))?)?;
                bound = (bound + residual.lo.abs().max(residual.hi.abs())).next_up();
                lower = lower.max(if residual.lo > 0. {
                    residual.lo
                } else if residual.hi < 0. {
                    -residual.hi
                } else {
                    0.
                });
            }
            if lower > tolerance {
                return Ok(CandidateCertificate {
                    status: CandidateStatus::Exceeds,
                    error_upper: None,
                    cells,
                });
            }
            if bound <= tolerance {
                upper = upper.max(bound);
                continue;
            }
        }
        if depth == 48 {
            return Ok(CandidateCertificate {
                status: CandidateStatus::Unresolved,
                error_upper: None,
                cells,
            });
        }
        let axis = if (rect[0][1] - rect[0][0]) / (domain[0][1] - domain[0][0])
            >= (rect[1][1] - rect[1][0]) / (domain[1][1] - domain[1][0])
        {
            0
        } else {
            1
        };
        let mid = rect[axis][0] * 0.5 + rect[axis][1] * 0.5;
        if mid <= rect[axis][0] || mid >= rect[axis][1] {
            return Ok(CandidateCertificate {
                status: CandidateStatus::Unresolved,
                error_upper: None,
                cells,
            });
        }
        let mut left = rect;
        let mut right = rect;
        left[axis][1] = mid;
        right[axis][0] = mid;
        pending.push_back((right, depth + 1));
        pending.push_back((left, depth + 1));
    }
    Ok(CandidateCertificate {
        status: CandidateStatus::Within,
        error_upper: Some(upper),
        cells,
    })
}
