//! Original UV correspondence on a curved rational parameter chart.
use super::CandidateCertificate;
use super::mapped::{self, ParameterMap};
use crate::{Result, check, distance_bounds::Interval as I, surface::Surface};

struct Map<'a> {
    surface: &'a Surface,
    hull: [[f64; 2]; 2],
}
impl ParameterMap for Map<'_> {
    fn enclosure(&self, rect: [[f64; 2]; 2]) -> Result<[[f64; 2]; 2]> {
        let b = super::jacobian_bounds(self.surface, rect, 0., 4096)?;
        let image = b.image.ok_or_else(|| {
            crate::Error::new(
                "NURBS_PARAMETRIC_OFFSET_REFUSED",
                "UV map enclosure unresolved",
            )
        })?;
        let mut out = [[0.; 2]; 2];
        for k in 0..2 {
            let x =
                I::new(image[k][0], image[k][1])?.intersect(self.hull[k][0], self.hull[k][1])?;
            out[k] = [x.lo, x.hi];
        }
        Ok(out)
    }
    fn jacobian(&self, rect: [[f64; 2]; 2]) -> Result<[[I; 2]; 2]> {
        let b = super::jacobian_bounds(self.surface, rect, 0., 4096)?;
        let d = b.derivatives.ok_or_else(|| {
            crate::Error::new(
                "NURBS_PARAMETRIC_OFFSET_REFUSED",
                "UV map derivatives unresolved",
            )
        })?;
        Ok([
            [
                I::new(d[0][0][0], d[0][0][1])?,
                I::new(d[0][1][0], d[0][1][1])?,
            ],
            [
                I::new(d[1][0][0], d[1][0][1])?,
                I::new(d[1][1][0], d[1][1][1])?,
            ],
        ])
    }
}

/// A midpoint inverse is only a proposed constant preconditioner. Positive
/// definiteness of the symmetric part of P*J must hold on the WHOLE rectangle.
/// Integrating its quadratic form along every parameter segment proves strong
/// monotonicity, hence global UV injectivity. Positive det(P) then also proves
/// positive orientation of J. No sampled Jacobian authorizes the chart.
fn qualify(map: &Map<'_>, domain: [[f64; 2]; 2]) -> Result<()> {
    let center = domain.map(|r| [r[0] * 0.5 + r[1] * 0.5; 2]);
    let j = map.jacobian(center)?;
    let m = |x: I| x.lo * 0.5 + x.hi * 0.5;
    let [a, b, c, d] = [m(j[0][0]), m(j[1][0]), m(j[0][1]), m(j[1][1])];
    let det = a * d - b * c;
    check(
        det.is_finite() && det > 0.,
        "UV chart midpoint has no positive inverse proposal",
    )?;
    let p = [[d / det, -b / det], [-c / det, a / det]];
    check(
        p.iter().flatten().all(|x| x.is_finite()),
        "UV chart preconditioner overflow",
    )?;
    let dp = I::point(p[0][0])
        .mul(I::point(p[1][1]))?
        .sub(I::point(p[0][1]).mul(I::point(p[1][0]))?)?;
    check(dp.lo > 0., "UV chart preconditioner orientation unresolved")?;
    let j = map.jacobian(domain)?;
    let mut a = [[I::point(0.); 2]; 2];
    for row in 0..2 {
        for col in 0..2 {
            a[row][col] = I::point(p[row][0])
                .mul(j[col][0])?
                .add(I::point(p[row][1]).mul(j[col][1])?)?;
        }
    }
    let off = a[0][1].add(a[1][0])?.mul(I::point(0.5))?;
    let det = a[0][0].mul(a[1][1])?.sub(off.mul(off)?)?;
    check(
        a[0][0].lo > 0. && a[1][1].lo > 0. && det.lo > 0.,
        "Whole UV chart strong monotonicity/orientation is unresolved",
    )
}

/// Compare a complete candidate against original normal offsets under an
/// authored rational UV surface. The map stores `(source_u, source_v, 0)` and
/// has exactly the candidate's active parameter domain. Its positive-weight
/// control hull must stay inside the original chart; all three charts need
/// continuous nonperiodic jets. A constant preconditioner with a uniformly
/// positive symmetric Jacobian proves global map injectivity and orientation.
/// Valid charts outside this sufficient admission condition may refuse.
///
/// The map may have curved boundaries, independent degrees/weights and knots.
/// Offset jets use both UV chain-rule terms over every candidate cell. This
/// does not discover a map or a contact branch, sew topology, prove world
/// embedding, or assert any thickness over omitted source parameters.
pub fn certify_parametric_candidate(
    source: &Surface,
    candidate: &Surface,
    uv_map: &Surface,
    distance: f64,
    tolerance: f64,
    max_cells: usize,
) -> Result<CandidateCertificate> {
    source.validate()?;
    candidate.validate()?;
    uv_map.validate()?;
    check(
        distance.is_finite() && tolerance.is_finite() && tolerance > 0.,
        "Invalid parametric offset tolerance/distance",
    )?;
    check(
        (1..=1_000_000).contains(&max_cells),
        "Invalid parametric offset budget",
    )?;
    let cd = mapped::domain(candidate);
    check(
        mapped::domain(uv_map) == cd
            && !candidate.periodic_u
            && !candidate.periodic_v
            && super::source::source_continuous_for_offset(candidate, cd),
        "Candidate must retain the continuous nonperiodic map domain",
    )?;
    validate_map(source, uv_map)?;
    let hull = hull(uv_map);
    let map = Map {
        surface: uv_map,
        hull,
    };
    mapped::compare(source, candidate, &map, distance, tolerance, max_cells)
}
fn hull(uv_map: &Surface) -> [[f64; 2]; 2] {
    std::array::from_fn(|k| {
        [
            uv_map
                .control_points
                .iter()
                .flatten()
                .map(|p| p[k])
                .fold(f64::INFINITY, f64::min),
            uv_map
                .control_points
                .iter()
                .flatten()
                .map(|p| p[k])
                .fold(f64::NEG_INFINITY, f64::max),
        ]
    })
}
pub(super) fn validate_map(source: &Surface, uv_map: &Surface) -> Result<()> {
    source.validate()?;
    uv_map.validate()?;
    let sd = mapped::domain(source);
    let cd = mapped::domain(uv_map);
    check(
        cd.iter().all(|r| r[0] < r[1] && (r[1] - r[0]).is_finite()),
        "UV map must retain the candidate's finite active domain",
    )?;
    check(
        uv_map
            .control_points
            .iter()
            .flatten()
            .all(|p| p[2] == 0. && (0..2).all(|k| p[k] >= sd[k][0] && p[k] <= sd[k][1])),
        "UV map must store original (u,v,0) inside the source chart",
    )?;
    let hull = hull(uv_map);
    check(
        [source, uv_map]
            .iter()
            .all(|s| !s.periodic_u && !s.periodic_v)
            && super::source::source_continuous_for_offset(source, hull)
            && super::source::source_continuous_for_offset(uv_map, cd),
        "Parametric comparison requires continuous nonperiodic jets",
    )?;
    let map = Map {
        surface: uv_map,
        hull,
    };
    qualify(&map, cd)?;
    Ok(())
}
