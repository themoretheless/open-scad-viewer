//! Whole-domain offset comparison under an authored affine rectangular UV map.
//! Bases, weights and active domains may differ. This proves only a candidate's
//! mapped normal-offset error, not closest distance, branch completeness or a solid.
use super::{CandidateCertificate, CandidateStatus as S};
use crate::{Result, check, distance_bounds::Interval as I, surface::Surface};
pub(super) fn domain(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
pub(super) trait ParameterMap {
    fn enclosure(&self, rect: [[f64; 2]; 2]) -> Result<[[f64; 2]; 2]>;
    /// [candidate axis][source coordinate].
    fn jacobian(&self, rect: [[f64; 2]; 2]) -> Result<[[I; 2]; 2]>;
}
struct Map {
    target: [[f64; 2]; 2],
    source: [[f64; 2]; 2],
    scale: [I; 2],
}
impl ParameterMap for Map {
    fn enclosure(&self, rect: [[f64; 2]; 2]) -> Result<[[f64; 2]; 2]> {
        let mut out = [[0.; 2]; 2];
        for k in 0..2 {
            let t = I::new(rect[k][0], rect[k][1])?
                .sub(I::point(self.target[k][0]))?
                .div(I::point(self.target[k][1]).sub(I::point(self.target[k][0]))?)?
                .intersect(0., 1.)?;
            let x = I::point(self.source[k][0])
                .add(t.mul(I::point(self.source[k][1]).sub(I::point(self.source[k][0]))?)?)?
                .intersect(self.source[k][0], self.source[k][1])?;
            out[k] = [x.lo, x.hi];
        }
        Ok(out)
    }
    fn jacobian(&self, _rect: [[f64; 2]; 2]) -> Result<[[I; 2]; 2]> {
        Ok([[self.scale[0], I::point(0.)], [I::point(0.), self.scale[1]]])
    }
}
/// The complete candidate active rectangle maps affinely onto `source_window`.
/// Each window interval must increase and stay inside the original source
/// domain. Explicit original UV correspondence is retained; no nearest-point
/// guess or sampled classification supplies the proof. Native offset jets
/// enclose residual derivatives, tightening the direct image bound around an
/// interval-enclosed midpoint. All accepted cells cover the original candidate
/// chart, including endpoints and authored nonunit domains.
pub fn certify_mapped_candidate(
    source: &Surface,
    candidate: &Surface,
    source_window: [[f64; 2]; 2],
    distance: f64,
    tolerance: f64,
    max_cells: usize,
) -> Result<CandidateCertificate> {
    source.validate()?;
    candidate.validate()?;
    check(
        distance.is_finite() && tolerance.is_finite() && tolerance > 0.,
        "Invalid mapped offset tolerance/distance",
    )?;
    check(
        (1..=1_000_000).contains(&max_cells),
        "Invalid mapped offset budget",
    )?;
    let sd = domain(source);
    let cd = domain(candidate);
    check(
        (0..2).all(|k| {
            source_window[k].iter().all(|x| x.is_finite())
                && source_window[k][0] < source_window[k][1]
                && source_window[k][0] >= sd[k][0]
                && source_window[k][1] <= sd[k][1]
                && cd[k][0] < cd[k][1]
        }),
        "Mapped source window must lie inside the original domain",
    )?;
    check(
        !source.periodic_u
            && !source.periodic_v
            && !candidate.periodic_u
            && !candidate.periodic_v
            && super::source::source_continuous_for_offset(source, source_window)
            && super::source::source_continuous_for_offset(candidate, cd),
        "Mapped offset comparison requires continuous nonperiodic source and candidate jets",
    )?;
    let mut scale = [I::point(0.); 2];
    for k in 0..2 {
        scale[k] = I::point(source_window[k][1])
            .sub(I::point(source_window[k][0]))?
            .div(I::point(cd[k][1]).sub(I::point(cd[k][0]))?)?;
    }
    let map = Map {
        target: cd,
        source: source_window,
        scale,
    };
    compare(source, candidate, &map, distance, tolerance, max_cells)
}
pub(super) fn compare(
    source: &Surface,
    candidate: &Surface,
    map: &impl ParameterMap,
    distance: f64,
    tolerance: f64,
    max_cells: usize,
) -> Result<CandidateCertificate> {
    let cd = domain(candidate);
    let mut pending = std::collections::VecDeque::from([(cd, 0usize)]);
    let mut cells = 0;
    let mut maximum = 0_f64;
    while let Some((rect, depth)) = pending.pop_front() {
        if cells == max_cells {
            return Ok(CandidateCertificate {
                status: S::Unresolved,
                error_upper: None,
                cells,
            });
        }
        cells += 1;
        if let Some(residual) = residual(source, candidate, map, rect, distance)? {
            let mut upper = 0_f64;
            let mut lower = 0_f64;
            for r in residual {
                upper = (upper + r.lo.abs().max(r.hi.abs())).next_up();
                lower = lower.max(if r.lo > 0. {
                    r.lo
                } else if r.hi < 0. {
                    -r.hi
                } else {
                    0.
                });
            }
            if lower > tolerance {
                return Ok(CandidateCertificate {
                    status: S::Exceeds,
                    error_upper: None,
                    cells,
                });
            }
            if upper <= tolerance {
                maximum = maximum.max(upper);
                continue;
            }
        }
        if depth == 48 {
            return Ok(CandidateCertificate {
                status: S::Unresolved,
                error_upper: None,
                cells,
            });
        }
        let axis = if (rect[0][1] - rect[0][0]) / (cd[0][1] - cd[0][0])
            >= (rect[1][1] - rect[1][0]) / (cd[1][1] - cd[1][0])
        {
            0
        } else {
            1
        };
        let mid = rect[axis][0] * 0.5 + rect[axis][1] * 0.5;
        if mid <= rect[axis][0] || mid >= rect[axis][1] {
            return Ok(CandidateCertificate {
                status: S::Unresolved,
                error_upper: None,
                cells,
            });
        }
        let mut a = rect;
        let mut b = rect;
        a[axis][1] = mid;
        b[axis][0] = mid;
        pending.push_back((a, depth + 1));
        pending.push_back((b, depth + 1));
    }
    Ok(CandidateCertificate {
        status: S::Within,
        error_upper: Some(maximum),
        cells,
    })
}
fn residual(
    source: &Surface,
    candidate: &Surface,
    map: &impl ParameterMap,
    rect: [[f64; 2]; 2],
    distance: f64,
) -> Result<Option<[I; 3]>> {
    let target = super::jacobian_bounds(candidate, rect, 0., 4096)?;
    let original = super::jacobian_bounds(source, map.enclosure(rect)?, distance, 4096)?;
    let (Some(ti), Some(td), Some(si), Some(sd)) = (
        target.image,
        target.derivatives,
        original.image,
        original.derivatives,
    ) else {
        return Ok(None);
    };
    let mid = rect.map(|r| r[0] * 0.5 + r[1] * 0.5);
    let tc = super::bounds(candidate, mid.map(|x| [x, x]), 0., 4096)?;
    let sc = super::bounds(source, map.enclosure(mid.map(|x| [x, x]))?, distance, 4096)?;
    let (Some(tc), Some(sc)) = (tc.image, sc.image) else {
        return Ok(None);
    };
    let jacobian = map.jacobian(rect)?;
    let mut out = [I::point(0.); 3];
    for k in 0..3 {
        let direct = I::new(ti[k][0], ti[k][1])?.sub(I::new(si[k][0], si[k][1])?)?;
        let mut centered = I::new(tc[k][0], tc[k][1])?.sub(I::new(sc[k][0], sc[k][1])?)?;
        for axis in 0..2 {
            let mapped = I::new(sd[0][k][0], sd[0][k][1])?
                .mul(jacobian[axis][0])?
                .add(I::new(sd[1][k][0], sd[1][k][1])?.mul(jacobian[axis][1])?)?;
            let derivative = I::new(td[axis][k][0], td[axis][k][1])?.sub(mapped)?;
            centered = centered.add(
                derivative.mul(I::new(rect[axis][0], rect[axis][1])?.sub(I::point(mid[axis]))?)?,
            )?;
        }
        out[k] = direct.intersect(centered.lo, centered.hi)?;
    }
    Ok(Some(out))
}
