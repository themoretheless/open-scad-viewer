//! Native evidence produced by surface projection uniqueness checks.
#[derive(Debug)]
pub enum SurfaceProjectionProof {
    DistanceSeparation,
    Affine {
        gram_determinant_lower: f64,
    },
    Krawczyk {
        root: [f64; 2],
        excluded_boxes: usize,
        separated_by_lower_bound: usize,
        global_distance_upper: f64,
    },
}

#[path = "surface_projection/serialization.rs"]
#[cfg(feature = "codec")]
mod serialization;

use super::{CurveProjection, ProjectionStatus, ToleranceContext};

#[derive(Clone, Copy, Debug)]
pub enum SurfaceCandidateClassification {
    Affine,
    KrawczykOrSeparated,
    GlobalCandidateBox,
}
#[derive(Debug)]
pub struct SurfaceProjectionCandidate {
    pub parameter_box: [f64; 4],
    pub point: [f64; 3],
    pub distance_lower: f64,
    pub distance_upper: f64,
    pub classification: SurfaceCandidateClassification,
}
#[derive(Debug)]
pub struct BoundaryProjection {
    pub edge: &'static str,
    pub certificate: CurveProjection,
}
#[derive(Debug)]
pub struct SurfaceProjection {
    pub status: ProjectionStatus,
    pub global_distance_upper: f64,
    pub candidates: Vec<SurfaceProjectionCandidate>,
    pub boundary_reductions: Vec<BoundaryProjection>,
    pub uniqueness_proof: Option<SurfaceProjectionProof>,
    pub subdivisions: usize,
    pub tolerance: ToleranceContext,
}

use super::{
    Axis, MAX_CERTIFICATE_CELLS, Result, Surface, SurfaceBox, box_of, check, context, copy_context,
    distance, next_down, next_up, point_box_distance, project_curve_report,
    prove_surface_projection_uniqueness,
};

/// Certified global surface projection by outward hull branch-and-bound.
/// Every unpruned parameter rectangle is returned; four boundary problems are
/// reduced to the certified curve projector.
pub fn project_surface_report(
    surface: &Surface,
    point: [f64; 3],
    tolerance: Option<ToleranceContext>,
) -> Result<SurfaceProjection> {
    surface.validate()?;
    check(
        point.iter().all(|value| value.is_finite()),
        "Projection point must be finite",
    )?;
    let tolerance = context(tolerance);
    let nu = surface.control_points.len();
    let nv = surface.control_points[0].len();
    let domain = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[nu],
        surface.knots_v[surface.degree_v],
        surface.knots_v[nv],
    ];
    let make_box = |bounds: [f64; 4]| -> Result<SurfaceBox> {
        let patch = surface.trim(bounds)?;
        let controls: Vec<Vec<f64>> = patch.control_points.iter().flatten().cloned().collect();
        let (min, max) = box_of(&controls);
        let center = [(bounds[0] + bounds[1]) * 0.5, (bounds[2] + bounds[3]) * 0.5];
        let evaluated = surface.evaluate_validated(center[0], center[1])?.point;
        Ok(SurfaceBox {
            bounds,
            lower: next_down(point_box_distance(&min, &max, &point).max(0.)),
            upper: next_up(distance(&evaluated, &point)),
            point: evaluated,
        })
    };
    let mut active = vec![make_box(domain)?];
    let mut best = active[0].upper;
    let target = tolerance.parametric_bounds().floor;
    let mut subdivisions = 0_usize;
    while subdivisions < MAX_CERTIFICATE_CELLS {
        active.retain(|cell| cell.lower <= best);
        let Some((index, _)) = active
            .iter()
            .enumerate()
            .filter(|(_, cell)| {
                (cell.bounds[1] - cell.bounds[0]).max(cell.bounds[3] - cell.bounds[2]) > target
            })
            .max_by(|(_, a), (_, b)| {
                (a.bounds[1] - a.bounds[0])
                    .max(a.bounds[3] - a.bounds[2])
                    .total_cmp(&(b.bounds[1] - b.bounds[0]).max(b.bounds[3] - b.bounds[2]))
            })
        else {
            break;
        };
        if active.len() + 1 >= MAX_CERTIFICATE_CELLS {
            break;
        }
        let cell = active.swap_remove(index);
        let [u0, u1, v0, v1] = cell.bounds;
        let children = if u1 - u0 >= v1 - v0 {
            let middle = (u0 + u1) * 0.5;
            [
                make_box([u0, middle, v0, v1])?,
                make_box([middle, u1, v0, v1])?,
            ]
        } else {
            let middle = (v0 + v1) * 0.5;
            [
                make_box([u0, u1, v0, middle])?,
                make_box([u0, u1, middle, v1])?,
            ]
        };
        best = best.min(children[0].upper).min(children[1].upper);
        active.extend(children);
        subdivisions += 1;
    }
    active.retain(|cell| cell.lower <= best);
    active.sort_by(|a, b| {
        a.bounds[0]
            .total_cmp(&b.bounds[0])
            .then(a.bounds[2].total_cmp(&b.bounds[2]))
    });
    let boundaries = [
        ("u-min", surface.iso(Axis::U, domain[0])?),
        ("u-max", surface.iso(Axis::U, domain[1])?),
        ("v-min", surface.iso(Axis::V, domain[2])?),
        ("v-max", surface.iso(Axis::V, domain[3])?),
    ]
    .into_iter()
    .map(|(edge, curve)| {
        project_curve_report(&curve, &point, Some(copy_context(&tolerance)?))
            .map(|certificate| BoundaryProjection { edge, certificate })
    })
    .collect::<Result<Vec<_>>>()?;
    let mut boxes = active;
    let affine_uniqueness = if surface.degree_u == 1
        && surface.degree_v == 1
        && nu == 2
        && nv == 2
        && surface
            .weights
            .iter()
            .flatten()
            .all(|weight| *weight == surface.weights[0][0])
    {
        let origin = &surface.control_points[0][0];
        let du = surface.control_points[1][0]
            .iter()
            .zip(origin)
            .map(|(x, o)| x - o)
            .collect::<Vec<_>>();
        let dv = surface.control_points[0][1]
            .iter()
            .zip(origin)
            .map(|(x, o)| x - o)
            .collect::<Vec<_>>();
        let closure = surface.control_points[1][1]
            .iter()
            .zip(origin)
            .zip(&du)
            .zip(&dv)
            .all(|(((x, o), u), v)| (*x - *o - *u - *v).abs() <= 64. * f64::EPSILON);
        let a = du.iter().map(|x| x * x).sum::<f64>();
        let b = du.iter().zip(&dv).map(|(x, y)| x * y).sum::<f64>();
        let c = dv.iter().map(|x| x * x).sum::<f64>();
        let determinant = a * c - b * b;
        let rhsu = point
            .iter()
            .zip(origin)
            .zip(&du)
            .map(|((x, o), d)| (x - o) * d)
            .sum::<f64>();
        let rhsv = point
            .iter()
            .zip(origin)
            .zip(&dv)
            .map(|((x, o), d)| (x - o) * d)
            .sum::<f64>();
        let su = (rhsu * c - rhsv * b) / determinant;
        let sv = (rhsv * a - rhsu * b) / determinant;
        (closure && determinant > 0. && (0.0..=1.0).contains(&su) && (0.0..=1.0).contains(&sv))
            .then_some((su, sv, determinant))
    } else {
        None
    };
    let mut uniqueness_proof = None;
    let mut status = if boxes.len() == 1 {
        ProjectionStatus::IsolatedCandidate
    } else {
        ProjectionStatus::NonuniqueOrUnresolved
    };
    if let Some((su, sv, determinant)) = affine_uniqueness {
        let u = domain[0] + (domain[1] - domain[0]) * su;
        let v = domain[2] + (domain[3] - domain[2]) * sv;
        let projected = surface.evaluate_validated(u, v)?.point;
        let d = next_up(distance(&projected, &point));
        boxes = vec![SurfaceBox {
            bounds: [u, u, v, v],
            lower: next_down(d),
            upper: d,
            point: projected,
        }];
        status = ProjectionStatus::Unique;
        uniqueness_proof = Some(SurfaceProjectionProof::Affine {
            gram_determinant_lower: next_down(determinant),
        });
    } else if let Some(proof) =
        prove_surface_projection_uniqueness(surface, &point, &mut boxes, best, target)?
    {
        status = ProjectionStatus::Unique;
        uniqueness_proof = Some(proof);
        best = boxes
            .iter()
            .map(|cell| cell.upper)
            .fold(f64::INFINITY, f64::min);
    }
    let classification = if matches!(
        uniqueness_proof,
        Some(SurfaceProjectionProof::Affine { .. })
    ) {
        SurfaceCandidateClassification::Affine
    } else if uniqueness_proof.is_some() {
        SurfaceCandidateClassification::KrawczykOrSeparated
    } else {
        SurfaceCandidateClassification::GlobalCandidateBox
    };
    let candidates = boxes
        .into_iter()
        .map(|cell| SurfaceProjectionCandidate {
            parameter_box: [
                next_down(cell.bounds[0]),
                next_up(cell.bounds[1]),
                next_down(cell.bounds[2]),
                next_up(cell.bounds[3]),
            ],
            point: cell.point,
            distance_lower: cell.lower,
            distance_upper: cell.upper,
            classification,
        })
        .collect();
    Ok(SurfaceProjection {
        status,
        global_distance_upper: best,
        candidates,
        boundary_reductions: boundaries,
        uniqueness_proof,
        subdivisions,
        tolerance,
    })
}
