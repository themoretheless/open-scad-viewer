//! Continuous normal-offset comparison over a bilinear original UV region.
use super::CandidateCertificate;
use super::mapped::{self, ParameterMap};
use crate::{Result, check, distance_bounds::Interval as I, surface::Surface};

struct Map {
    target: [[f64; 2]; 2],
    corners: [[[f64; 2]; 2]; 2],
    hull: [[f64; 2]; 2],
}
impl Map {
    fn normalized(&self, rect: [[f64; 2]; 2]) -> Result<[I; 2]> {
        let mut out = [I::point(0.); 2];
        for k in 0..2 {
            out[k] = I::new(rect[k][0], rect[k][1])?
                .sub(I::point(self.target[k][0]))?
                .div(I::point(self.target[k][1]).sub(I::point(self.target[k][0]))?)?
                .intersect(0., 1.)?;
        }
        Ok(out)
    }
}
fn lerp(a: I, b: I, t: I) -> Result<I> {
    a.add(b.sub(a)?.mul(t)?)
}
impl ParameterMap for Map {
    fn enclosure(&self, rect: [[f64; 2]; 2]) -> Result<[[f64; 2]; 2]> {
        let [u, v] = self.normalized(rect)?;
        let mut out = [[0.; 2]; 2];
        for k in 0..2 {
            let a = lerp(
                I::point(self.corners[0][0][k]),
                I::point(self.corners[1][0][k]),
                u,
            )?;
            let b = lerp(
                I::point(self.corners[0][1][k]),
                I::point(self.corners[1][1][k]),
                u,
            )?;
            let value = lerp(a, b, v)?.intersect(self.hull[k][0], self.hull[k][1])?;
            out[k] = [value.lo, value.hi];
        }
        Ok(out)
    }
    fn jacobian(&self, rect: [[f64; 2]; 2]) -> Result<[[I; 2]; 2]> {
        let t = self.normalized(rect)?;
        let mut out = [[I::point(0.); 2]; 2];
        for axis in 0..2 {
            let width = I::point(self.target[axis][1]).sub(I::point(self.target[axis][0]))?;
            for k in 0..2 {
                let delta = |other: usize| {
                    let (a, b) = if axis == 0 {
                        (self.corners[0][other][k], self.corners[1][other][k])
                    } else {
                        (self.corners[other][0][k], self.corners[other][1][k])
                    };
                    I::point(b).sub(I::point(a))
                };
                out[axis][k] = lerp(delta(0)?, delta(1)?, t[1 - axis])?.div(width)?;
            }
        }
        Ok(out)
    }
}

/// Compare every candidate parameter to the normal offset at its bilinearly
/// mapped ORIGINAL UV coordinate. `corners[u_end][v_end]` are literal source
/// coordinates, including nonunit source and candidate domains. Their convex
/// hull must lie in the original chart. A full-domain interval Jacobian must
/// prove positive orientation; folded, reversed and unresolved maps refuse.
///
/// The mapped region has four straight UV sides, which need not be parallel.
/// This certificate covers displacement on the complete candidate, not omitted
/// source parameters, world injectivity, contact-branch completeness or sewing.
pub fn certify_quadrilateral_candidate(
    source: &Surface,
    candidate: &Surface,
    corners: [[[f64; 2]; 2]; 2],
    distance: f64,
    tolerance: f64,
    max_cells: usize,
) -> Result<CandidateCertificate> {
    source.validate()?;
    candidate.validate()?;
    check(
        distance.is_finite() && tolerance.is_finite() && tolerance > 0.,
        "Invalid quadrilateral offset tolerance/distance",
    )?;
    check(
        (1..=1_000_000).contains(&max_cells),
        "Invalid quadrilateral offset budget",
    )?;
    let sd = mapped::domain(source);
    let cd = mapped::domain(candidate);
    check(
        corners
            .iter()
            .flatten()
            .all(|p| (0..2).all(|k| p[k].is_finite() && p[k] >= sd[k][0] && p[k] <= sd[k][1])),
        "Quadrilateral corners must stay inside the original chart",
    )?;
    check(
        cd.iter().all(|r| r[0] < r[1] && (r[1] - r[0]).is_finite()),
        "Candidate chart must have finite positive widths",
    )?;
    let hull = std::array::from_fn(|k| {
        [
            corners
                .iter()
                .flatten()
                .map(|p| p[k])
                .fold(f64::INFINITY, f64::min),
            corners
                .iter()
                .flatten()
                .map(|p| p[k])
                .fold(f64::NEG_INFINITY, f64::max),
        ]
    });
    check(
        !source.periodic_u
            && !source.periodic_v
            && !candidate.periodic_u
            && !candidate.periodic_v
            && super::source::source_continuous_for_offset(source, hull)
            && super::source::source_continuous_for_offset(candidate, cd),
        "Quadrilateral comparison requires continuous nonperiodic offset jets",
    )?;
    let map = Map {
        target: cd,
        corners,
        hull,
    };
    let j = map.jacobian(cd)?;
    let determinant = j[0][0].mul(j[1][1])?.sub(j[0][1].mul(j[1][0])?)?;
    check(
        determinant.lo > 0.,
        "Quadrilateral map orientation is folded, reversed or unresolved",
    )?;
    mapped::compare(source, candidate, &map, distance, tolerance, max_cells)
}
