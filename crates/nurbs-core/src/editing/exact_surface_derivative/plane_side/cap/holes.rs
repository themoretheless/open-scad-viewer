//! Planar section caps with strictly nested, disjoint hole boundaries.
use super::{
    Curve, PResult, ProjectedTrimmedFace, Q, Refusal, Result, Work, cap, check, orient,
    segments_contact, validate_edges,
};
use num_traits::Signed;

#[derive(Clone, Debug)]
pub struct PlanarSectionCapWithHoles {
    pub face: ProjectedTrimmedFace,
    /// Whether each input ring has its edge order and clocks reversed.
    pub reversed: Vec<bool>,
    pub boundary_gap_upper: Q,
}

#[derive(Clone, Debug)]
pub struct PlanarSectionCapWithHolesReport {
    pub outcome: std::result::Result<PlanarSectionCapWithHoles, Refusal>,
    pub work_used: u64,
}

/// Build a planar cap with an outer ring followed by disjoint holes.
/// Exact crossing and containment tests certify the topology in the common
/// plane chart. The outer ring is counterclockwise and holes are clockwise.
pub fn cap_surface_plane_section_with_holes(
    rings: &[&[Curve]],
    normal: [f64; 3],
    offset: f64,
    boundary_tolerance: f64,
    max_work: u64,
) -> Result<PlanarSectionCapWithHolesReport> {
    validate_rings(rings, normal, offset, boundary_tolerance, max_work)?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let outcome = build(rings, normal, offset, boundary_tolerance, &mut work);
    Ok(PlanarSectionCapWithHolesReport {
        outcome,
        work_used: work.used,
    })
}

fn validate_rings(
    rings: &[&[Curve]],
    normal: [f64; 3],
    offset: f64,
    boundary_tolerance: f64,
    max_work: u64,
) -> Result<()> {
    check(
        (1..=256).contains(&rings.len())
            && rings.iter().all(|ring| (1..=256).contains(&ring.len()))
            && normal.iter().all(|x| x.is_finite())
            && normal.iter().any(|&x| x != 0.)
            && offset.is_finite()
            && boundary_tolerance.is_finite()
            && boundary_tolerance >= 0.
            && (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "A cap with holes needs bounded rings, a finite nonzero plane and nonnegative tolerance",
    )?;
    for ring in rings {
        validate_edges(ring)?;
    }
    Ok(())
}

fn build(
    rings: &[&[Curve]],
    normal: [f64; 3],
    offset: f64,
    tolerance: f64,
    work: &mut Work,
) -> PResult<PlanarSectionCapWithHoles> {
    let outer = cap(rings[0], normal, offset, tolerance, work)?;
    let mut face = outer.face;
    let mut reversed = vec![outer.reversed];
    let mut boundary_gap_upper = outer.boundary_gap_upper;
    let mut polygons = vec![Ring::new(&face.loops[0], work)?];
    for ring in &rings[1..] {
        let hole = cap(ring, normal, offset, tolerance, work)?;
        boundary_gap_upper = boundary_gap_upper.max(hole.boundary_gap_upper);
        let mut contour = hole
            .face
            .loops
            .into_iter()
            .next()
            .ok_or(Refusal::InvalidOutput)?;
        let candidate = Ring::new(&contour, work)?;
        for (i, accepted) in polygons.iter().enumerate() {
            if contact(accepted, &candidate, work)? {
                return Err(Refusal::UnprovedIdentity);
            }
            let inside_accepted = inside(accepted, &candidate.exact[0], work)?;
            if i == 0 {
                if !inside_accepted {
                    return Err(Refusal::UnprovedIdentity);
                }
            } else if inside_accepted || inside(&candidate, &accepted.exact[0], work)? {
                return Err(Refusal::UnprovedIdentity);
            }
        }
        reverse_ring(&mut contour);
        face.loops.push(contour);
        polygons.push(candidate);
        reversed.push(!hole.reversed);
    }
    Ok(PlanarSectionCapWithHoles {
        face,
        reversed,
        boundary_gap_upper,
    })
}

struct Ring {
    points: Vec<[f64; 2]>,
    exact: Vec<[Q; 2]>,
}

impl Ring {
    fn new(contour: &[Curve], work: &mut Work) -> PResult<Self> {
        let points: Vec<_> = contour
            .iter()
            .flat_map(|curve| {
                curve.control_points[..curve.control_points.len() - 1]
                    .iter()
                    .map(|p| [p[0], p[1]])
            })
            .collect();
        let exact = points
            .iter()
            .map(|&[x, y]| Ok([work.lift(x)?, work.lift(y)?]))
            .collect::<PResult<Vec<_>>>()?;
        Ok(Self { points, exact })
    }
}

fn contact(a: &Ring, b: &Ring, work: &mut Work) -> PResult<bool> {
    for i in 0..a.points.len() {
        let j = (i + 1) % a.points.len();
        for k in 0..b.points.len() {
            let l = (k + 1) % b.points.len();
            if segments_contact(
                [a.points[i], a.points[j], b.points[k], b.points[l]],
                [&a.exact[i], &a.exact[j], &b.exact[k], &b.exact[l]],
                work,
            )? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

// Ring contacts have already been excluded. Apply the half-open ray rule.
fn inside(ring: &Ring, point: &[Q; 2], work: &mut Work) -> PResult<bool> {
    let mut inside = false;
    for i in 0..ring.exact.len() {
        work.charge(1)?;
        let a = &ring.exact[i];
        let b = &ring.exact[(i + 1) % ring.exact.len()];
        if (a[1] > point[1]) != (b[1] > point[1])
            && orient(a, b, point, work)?.is_positive() == (b[1] > a[1])
        {
            inside = !inside;
        }
    }
    Ok(inside)
}

fn reverse_ring(contour: &mut [Curve]) {
    contour.reverse();
    for curve in contour {
        curve.control_points.reverse();
    }
}

mod arrangement;
pub use arrangement::{
    PlanarSectionCapArrangement, PlanarSectionCapArrangementReport, PlanarSectionCapRegion,
    arrange_surface_plane_section_caps,
};
