//! A planar cap bounded by a certified, simple degree-one section loop.
use super::{PResult, Q, Refusal, Result, Surface, Work, check};
use crate::{
    analysis::{
        exact_affine_hull::PlanarSurfaceProjection, planar_trim_projection::ProjectedTrimmedFace,
    },
    curve::Curve,
};
use num_traits::{Signed, Zero};

#[derive(Clone, Debug)]
pub struct PlanarSectionCap {
    pub face: ProjectedTrimmedFace,
    /// The cap contour reverses the input edge order and each edge clock.
    pub reversed: bool,
    /// Continuous XYZ gap upper bound between the cap and supplied edge chain.
    pub boundary_gap_upper: Q,
}

#[derive(Clone, Debug)]
pub struct PlanarSectionCapReport {
    pub outcome: std::result::Result<PlanarSectionCap, Refusal>,
    pub work_used: u64,
}

/// Cap a closed chain of degree-one section edges with integer segment knots.
/// Exact planar projection certifies the boundary gap. Nonadjacent touches,
/// self-intersections, collapsed segments and retracing corners refuse the cap.
pub fn cap_surface_plane_section(
    edges: &[Curve],
    normal: [f64; 3],
    offset: f64,
    boundary_tolerance: f64,
    max_work: u64,
) -> Result<PlanarSectionCapReport> {
    check(
        (1..=256).contains(&edges.len())
            && normal.iter().all(|x| x.is_finite())
            && normal.iter().any(|&x| x != 0.)
            && offset.is_finite()
            && boundary_tolerance.is_finite()
            && boundary_tolerance >= 0.
            && (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Plane cap needs a bounded edge chain, finite nonzero plane and nonnegative tolerance",
    )?;
    validate_edges(edges)?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let outcome = cap(edges, normal, offset, boundary_tolerance, &mut work);
    Ok(PlanarSectionCapReport {
        outcome,
        work_used: work.used,
    })
}

fn validate_edges(edges: &[Curve]) -> Result<()> {
    for edge in edges {
        edge.validate()?;
        let count = edge.control_points.len();
        check(
            edge.control_points[0].len() == 3
                && edge.degree == 1
                && !edge.periodic
                && edge.weights.iter().all(|w| *w == edge.weights[0])
                && edge.knots[0] == 0.
                && edge.knots[count + 1] == (count - 1) as f64
                && edge.knots[1..=count]
                    .iter()
                    .enumerate()
                    .all(|(i, &k)| k == i as f64),
            "Plane cap edges must use the degree-one section clock",
        )?;
    }
    Ok(())
}

fn cap(
    edges: &[Curve],
    normal: [f64; 3],
    offset: f64,
    tolerance: f64,
    work: &mut Work,
) -> PResult<PlanarSectionCap> {
    let mut omitted = 0;
    for i in 1..3 {
        if normal[i].abs() > normal[omitted].abs() {
            omitted = i;
        }
    }
    let mut axes = match omitted {
        0 => [1, 2],
        1 => [0, 2],
        _ => [0, 1],
    };
    if (omitted == 1) != normal[omitted].is_sign_negative() {
        axes.swap(0, 1);
    }
    let pivot = work.lift(normal[omitted])?;
    let a = work.lift(normal[axes[0]])?;
    let b = work.lift(normal[axes[1]])?;
    let offset = work.lift(offset)?;
    let offset = work.op(&offset, &pivot, |a, b| a / b)?;
    let slopes = [
        work.op(&a, &pivot, |a, b| -a / b)?,
        work.op(&b, &pivot, |a, b| -a / b)?,
    ];
    let tolerance = work.lift(tolerance)?;
    let mut points = Vec::new();
    let mut contour = Vec::with_capacity(edges.len());
    let mut gap = Q::zero();
    for (i, edge) in edges.iter().enumerate() {
        let next = &edges[(i + 1) % edges.len()];
        if edge.control_points.last() != next.control_points.first() {
            return Err(Refusal::UnprovedIdentity);
        }
        let mut projected = Vec::with_capacity(edge.control_points.len());
        for value in &edge.control_points {
            let uv = axes.map(|axis| value[axis]);
            let u = work.lift(uv[0])?;
            let v = work.lift(uv[1])?;
            let x = work.op(&u, &slopes[0], |a, b| a * b)?;
            let y = work.op(&v, &slopes[1], |a, b| a * b)?;
            let z = work.op(&x, &y, |a, b| a + b)?;
            let z = work.op(&z, &offset, |a, b| a + b)?;
            let stored = work.lift(value[omitted])?;
            gap = gap.max(work.op(&z, &stored, |a, b| (a - b).abs())?);
            if gap > tolerance {
                return Err(Refusal::UnprovedIdentity);
            }
            projected.push(uv.to_vec());
        }
        points.extend(
            projected[..projected.len() - 1]
                .iter()
                .map(|p| [p[0], p[1]]),
        );
        contour.push(Curve::from_polyline(projected).map_err(|_| Refusal::InvalidOutput)?);
    }
    let winding = simple_winding(&points, work)?;
    let reversed = winding.is_negative();
    if reversed {
        contour.reverse();
        for curve in &mut contour {
            curve.control_points.reverse();
        }
    }
    let [[u0, u1], [v0, v1]] = [0, 1].map(|axis| {
        points
            .iter()
            .fold([f64::INFINITY, f64::NEG_INFINITY], |[lo, hi], p| {
                [lo.min(p[axis]), hi.max(p[axis])]
            })
    });
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![u0, u0, u1, u1],
        knots_v: vec![v0, v0, v1, v1],
        control_points: vec![
            vec![vec![u0, v0, 0.], vec![u0, v1, 0.]],
            vec![vec![u1, v0, 0.], vec![u1, v1, 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate().map_err(|_| Refusal::InvalidOutput)?;
    let projection = PlanarSurfaceProjection {
        surface,
        retained_axes: axes,
        omitted_axis: omitted,
        offset,
        slopes,
    };
    Ok(PlanarSectionCap {
        face: ProjectedTrimmedFace {
            projection,
            loops: vec![contour],
        },
        reversed,
        boundary_gap_upper: gap,
    })
}

fn simple_winding(points: &[[f64; 2]], work: &mut Work) -> PResult<Q> {
    let n = points.len();
    if n < 3 {
        return Err(Refusal::UnprovedIdentity);
    }
    let exact = points
        .iter()
        .map(|&[x, y]| Ok([work.lift(x)?, work.lift(y)?]))
        .collect::<PResult<Vec<_>>>()?;
    let mut area = Q::zero();
    for i in 0..n {
        let j = (i + 1) % n;
        let k = (i + 2) % n;
        if points[i] == points[j] {
            return Err(Refusal::UnprovedIdentity);
        }
        let term = cross(&exact[i], &exact[j], work)?;
        area = work.op(&area, &term, |a, b| a + b)?;
        if orient(&exact[i], &exact[j], &exact[k], work)?.is_zero()
            && (contained(points[i], points[j], points[k])
                || contained(points[k], points[i], points[j]))
        {
            return Err(Refusal::UnprovedIdentity);
        }
        for m in i + 2..n {
            let p = (m + 1) % n;
            if p == i {
                continue;
            }
            if segments_contact(
                [points[i], points[j], points[m], points[p]],
                [&exact[i], &exact[j], &exact[m], &exact[p]],
                work,
            )? {
                return Err(Refusal::UnprovedIdentity);
            }
        }
    }
    if area.is_zero() {
        return Err(Refusal::UnprovedIdentity);
    }
    Ok(area)
}

fn segments_contact(
    [a, b, c, d]: [[f64; 2]; 4],
    [aq, bq, cq, dq]: [&[Q; 2]; 4],
    work: &mut Work,
) -> PResult<bool> {
    work.charge(1)?;
    if (0..2).any(|i| a[i].max(b[i]) < c[i].min(d[i]) || c[i].max(d[i]) < a[i].min(b[i])) {
        return Ok(false);
    }
    let p = orient(aq, bq, cq, work)?;
    let q = orient(aq, bq, dq, work)?;
    let r = orient(cq, dq, aq, work)?;
    let s = orient(cq, dq, bq, work)?;
    Ok((opposite(&p, &q) && opposite(&r, &s))
        || (p.is_zero() && contained(c, a, b))
        || (q.is_zero() && contained(d, a, b))
        || (r.is_zero() && contained(a, c, d))
        || (s.is_zero() && contained(b, c, d)))
}

fn contained(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> bool {
    (0..2).all(|i| p[i] >= a[i].min(b[i]) && p[i] <= a[i].max(b[i]))
}

fn opposite(a: &Q, b: &Q) -> bool {
    (a.is_positive() && b.is_negative()) || (a.is_negative() && b.is_positive())
}

fn orient(a: &[Q; 2], b: &[Q; 2], c: &[Q; 2], work: &mut Work) -> PResult<Q> {
    let delta = |p: &[Q; 2], work: &mut Work| -> PResult<[Q; 2]> {
        Ok([
            work.op(&p[0], &a[0], |a, b| a - b)?,
            work.op(&p[1], &a[1], |a, b| a - b)?,
        ])
    };
    let b = delta(b, work)?;
    let c = delta(c, work)?;
    cross(&b, &c, work)
}

fn cross(a: &[Q; 2], b: &[Q; 2], work: &mut Work) -> PResult<Q> {
    let x = work.op(&a[0], &b[1], |a, b| a * b)?;
    let y = work.op(&a[1], &b[0], |a, b| a * b)?;
    Ok(work.op(&x, &y, |a, b| a - b)?)
}

mod holes;
pub use holes::{
    PlanarSectionCapArrangement, PlanarSectionCapArrangementReport, PlanarSectionCapRegion,
    PlanarSectionCapWithHoles, PlanarSectionCapWithHolesReport, arrange_surface_plane_section_caps,
    cap_surface_plane_section_with_holes,
};

mod support;
pub use support::{
    BoundedPlanarCapSupport, BoundedPlanarCapSupportReport, lift_planar_cap_support_bounded,
};
